use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::task::JoinHandle;

use crate::api::{ApiError, PrimeClient, TokenProvider};
use crate::catalog::{child_by_id, entity_by_id, record_id};
use crate::types::SessionConfig;

/// Resolved per-parent child spec (template-substituted at run time).
struct ResolvedChild {
    id: String,
    label: String,
    parent: String,
    single: bool,
    path: String,
    q_param: Option<String>,
    params: Vec<(String, String)>,
}

impl ResolvedChild {
    fn req(&self, parent_id: &str) -> crate::catalog::ChildReq {
        crate::catalog::ChildReq {
            path: self.path.replace("{id}", parent_id),
            params: self
                .q_param
                .as_ref()
                .map(|q| vec![("q".into(), q.replace("{id}", parent_id))])
                .unwrap_or_default()
                .into_iter()
                .chain(self.params.iter().map(|(k, v)| (k.clone(), v.replace("{id}", parent_id))))
                .collect(),
        }
    }
}

/// Entity spec for a session: provider-supplied specs first, built-in catalog
/// for sessions saved before specs existed.
fn entity_spec(config: &SessionConfig, id: &str) -> Option<crate::types::EntitySpec> {
    config
        .entity_specs
        .iter()
        .find(|e| e.id == id)
        .cloned()
        .or_else(|| {
            entity_by_id(id).map(|d| crate::types::EntitySpec {
                id: d.id.into(),
                label: d.label.into(),
                path: d.path.into(),
                group: d.group.into(),
                description: d.description.map(Into::into),
                downloadable: d.downloadable,
                method: None,
                body_template: None,
                data_path: None,
            })
        })
}

fn child_spec(config: &SessionConfig, id: &str) -> Option<ResolvedChild> {
    if let Some(s) = config.child_specs.iter().find(|c| c.id == id) {
        return Some(ResolvedChild {
            id: s.id.clone(),
            label: s.label.clone(),
            parent: s.parent.clone(),
            single: s.single,
            path: s.path.clone(),
            q_param: s.q_param.clone(),
            params: s.params.clone(),
        });
    }
    child_by_id(id).map(|d| {
        let probe = (d.request)("{id}");
        let q_param = probe.params.iter().find(|(k, _)| k == "q").map(|(_, v)| v.clone());
        ResolvedChild {
            id: d.id.into(),
            label: d.label.into(),
            parent: d.parent.into(),
            single: d.single,
            path: probe.path,
            q_param,
            params: probe
                .params
                .iter()
                .filter(|(k, _)| k != "q")
                .cloned()
                .collect(),
        }
    })
}
use crate::downloader::{attachment_filename, DownloadOutcome, Downloader};
use crate::limiter::{backoff, profile, ConcurrencyGate, ProfileDef, RateGate};
use crate::store::{FileRow, Store, TaskRow};
use crate::types::*;

const MAX_ATTEMPTS: u32 = 6;
const MAX_PAGES: i64 = 100_000;

pub type EmitFn = Arc<dyn Fn(&str) + Send + Sync>;

/// Tolerant unwrapping of Laravel/JSON:API-ish list responses.
pub fn extract_rows(resp: &Value, data_path: Option<&str>) -> Vec<Value> {
    if let Some(dp) = data_path {
        if let Some(v) = dp.split('.').fold(Some(resp), |v, k| v.and_then(|x| x.get(k))) {
            return extract_rows(v, None);
        }
    }
    if let Some(arr) = resp.as_array() {
        return arr.clone();
    }
    if let Some(obj) = resp.as_object() {
        if let Some(data) = obj.get("data") {
            if let Some(arr) = data.as_array() {
                return arr.clone();
            }
            if data.is_object() {
                return vec![data.clone()];
            }
        }
        return vec![resp.clone()];
    }
    vec![]
}

/// Decide whether the current page was the last one.
pub fn is_last_page(resp: &Value, page: i64, rows_on_page: usize, per_page: u32) -> bool {
    if let Some(obj) = resp.as_object() {
        if let Some(meta) = obj.get("meta").and_then(|m| m.as_object()) {
            if let Some(last) = meta
                .get("last_page")
                .or_else(|| meta.get("lastPage"))
                .and_then(|v| v.as_i64())
            {
                if last > 0 {
                    return page >= last;
                }
            }
            if let (Some(total), Some(pp)) = (
                meta.get("total").and_then(|v| v.as_i64()),
                meta.get("per_page")
                    .or_else(|| meta.get("perPage"))
                    .and_then(|v| v.as_i64()),
            ) {
                if pp > 0 {
                    return page * pp >= total;
                }
            }
        }
        if let Some(links) = obj.get("links").and_then(|l| l.as_object()) {
            if links.contains_key("next") {
                return links.get("next").map(|v| v.is_null()).unwrap_or(true);
            }
        }
    }
    (rows_on_page as i64) < per_page as i64
}

async fn with_retry<F, Fut, T, OnErr>(mut f: F, mut on_attempt: OnErr) -> Result<T, ApiError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, ApiError>>,
    OnErr: FnMut(u32, &ApiError),
{
    let mut attempt = 0u32;
    loop {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                attempt += 1;
                if !e.retryable || attempt >= MAX_ATTEMPTS {
                    return Err(e);
                }
                on_attempt(attempt, &e);
                let wait = e.retry_after.unwrap_or_else(|| backoff(attempt));
                tokio::time::sleep(wait.min(Duration::from_secs(60))).await;
            }
        }
    }
}

struct RunCtx {
    client: PrimeClient,
    downloader: Downloader,
    profile: ProfileDef,
    /// OAuth client-credentials providers: token is fetched in `run` and
    /// stored here for the client/downloader TokenProvider to read.
    oauth: Option<std::sync::Arc<OAuthState>>,
}

struct OAuthState {
    host: String,
    client_id: String,
    client_secret: String,
    token: std::sync::RwLock<Option<String>>,
}

pub struct Extractor {
    store: Arc<Store>,
    emit: EmitFn,
    running: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
}

impl Extractor {
    pub fn new(store: Arc<Store>, emit: EmitFn) -> Self {
        Self {
            store,
            emit,
            running: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // ---------- session lifecycle ----------

    pub fn create_session(
        &self,
        connection_id: &str,
        config: &SessionConfig,
    ) -> Result<(String, PathBuf)> {
        let id = uuid::Uuid::new_v4().to_string();
        let stamp = chrono::Utc::now().format("%Y-%m-%d-%H%M%S");
        let export_dir = Path::new(&config.dest_dir)
            .join(format!("primeeco-export-{stamp}-{}", &id[..8]));
        std::fs::create_dir_all(export_dir.join("data"))?;
        std::fs::create_dir_all(export_dir.join("files"))?;

        self.store
            .create_session(&id, connection_id, config, &export_dir.to_string_lossy())?;
        for entity_id in &config.entities {
            if let Some(def) = entity_spec(config, entity_id) {
                self.store
                    .add_task(&id, "entity", &def.id, &def.label, &def.path, None, None)?;
            }
        }
        self.store
            .log_event(&id, "info", &format!("Session created → {}", export_dir.display()));
        Ok((id, export_dir))
    }

    pub fn start(self: &Arc<Self>, session_id: &str) -> Result<()> {
        self.launch(session_id)
    }

    pub fn resume(self: &Arc<Self>, session_id: &str) -> Result<()> {
        self.launch(session_id)
    }

    pub fn pause(&self, session_id: &str) -> Result<()> {
        self.store.set_session_status(session_id, SessionStatus::Paused)?;
        self.store.log_event(session_id, "info", "Paused");
        (self.emit)(session_id);
        Ok(())
    }

    pub fn cancel(&self, session_id: &str) -> Result<()> {
        self.store
            .set_session_status(session_id, SessionStatus::Cancelled)?;
        self.store.log_event(session_id, "info", "Cancelled");
        (self.emit)(session_id);
        Ok(())
    }

    fn launch(self: &Arc<Self>, session_id: &str) -> Result<()> {
        {
            let mut map = self.running.lock().unwrap();
            if let Some(h) = map.get(session_id) {
                if !h.is_finished() {
                    return Ok(()); // already running
                }
                map.remove(session_id); // stale finished handle
            }
        }
        let session = self
            .store
            .get_session(session_id)?
            .ok_or_else(|| anyhow::anyhow!("Unknown session {session_id}"))?;
        if matches!(session.status, SessionStatus::Complete) {
            return Ok(());
        }

        // Resolve the connection this session belongs to → host + API key.
        let conn = self
            .store
            .get_connection(&session.connection_id)?
            .ok_or_else(|| anyhow::anyhow!("session's connection was deleted"))?;
        let oauth = if matches!(conn.provider.as_str(), "buildpass" | "procore" | "simpro") {
            let cid = conn.credentials.get("clientId").cloned().unwrap_or_default();
            let csec = conn.credentials.get("clientSecret").cloned().unwrap_or_default();
            Some(OAuthState {
                host: conn.host.clone(),
                client_id: cid,
                client_secret: csec,
                token: std::sync::RwLock::new(None),
            })
        } else {
            None
        };
        let oauth = oauth.map(std::sync::Arc::new);
        let key = conn.api_key.clone();
        let token_fn: TokenProvider = match &oauth {
            Some(o) => {
                let o = std::sync::Arc::clone(o);
                Arc::new(move || o.token.read().ok().and_then(|t| t.clone()))
            }
            None => Arc::new(move || Some(key.clone())),
        };
        let prof = profile(session.profile);
        let client = PrimeClient::new(&conn.host, token_fn.clone(), Arc::new(RateGate::new(prof)));
        let origin = reqwest::Url::parse(&conn.host)
            .map(|u| u.origin().ascii_serialization())
            .unwrap_or_default();
        let ctx = Arc::new(RunCtx {
            client,
            downloader: Downloader::new(
                Arc::new(ConcurrencyGate::new(prof.file_concurrency)),
                origin,
                token_fn,
            ),
            profile: prof,
            oauth,
        });

        self.store
            .set_session_status(session_id, SessionStatus::Running)?;
        self.store.log_event(session_id, "info", "Extraction started");
        (self.emit)(session_id);

        let this = Arc::clone(self);
        let sid = session_id.to_string();
        let handle = tokio::spawn(async move {
            if let Err(e) = this.run(&sid, ctx).await {
                this.store
                    .log_event(&sid, "error", &format!("Fatal engine error: {e}"));
                let _ = this.store.set_session_status(&sid, SessionStatus::Error);
            }
            (this.emit)(&sid);
            this.running.lock().unwrap().remove(&sid);
        });

        // Register handle; if a stale entry exists something is wrong but we proceed.
        self.running
            .lock()
            .unwrap()
            .insert(session_id.to_string(), handle);
        Ok(())
    }

    pub fn is_running(&self, session_id: &str) -> bool {
        self.running.lock().unwrap().contains_key(session_id)
    }

    /// Await an in-flight session (tests/shutdown). Returns when it exits.
    pub async fn wait_for(&self, session_id: &str) {
        let handle = self.running.lock().unwrap().remove(session_id);
        if let Some(h) = handle {
            let _ = h.await;
        }
    }

    pub async fn shutdown(&self) {
        let handles: Vec<JoinHandle<()>> = self
            .running
            .lock()
            .unwrap()
            .drain()
            .map(|(_, h)| h)
            .collect();
        for h in handles {
            let _ = h.await;
        }
    }

    // ---------- engine ----------

    async fn run(&self, session_id: &str, ctx: Arc<RunCtx>) -> Result<()> {
        // Grace period — a moment to cancel an accidental start before any
        // API work begins.
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        if self.store.session_status(session_id) != SessionStatus::Running {
            return Ok(());
        }

        if let Some(o) = &ctx.oauth {
            match fetch_oauth_token(&o.host, &o.client_id, &o.client_secret, None).await {
                Ok(OAuthOutcome::Token(t)) => *o.token.write().unwrap() = Some(t),
                Ok(OAuthOutcome::Needs2fa(_)) => {
                    let msg = "OAuth 2FA required — this credential needs a second-factor code".to_string();
                    self.store.log_event(session_id, "error", &msg);
                    self.store.set_session_status(session_id, SessionStatus::Error)?;
                    return Err(anyhow::anyhow!(msg));
                }
                Err(e) => {
                    self.store
                        .log_event(session_id, "error", &format!("OAuth token exchange failed: {e}"));
                    self.store.set_session_status(session_id, SessionStatus::Error)?;
                    return Err(e);
                }
            }
            self.store.log_event(session_id, "info", "OAuth token acquired");
        }

        let mut workers = Vec::new();
        for _ in 0..ctx.profile.concurrency {
            workers.push(tokio::spawn(task_worker(
                Arc::clone(&self.store),
                Arc::clone(&ctx),
                self.emit.clone(),
                session_id.to_string(),
            )));
        }
        for _ in 0..ctx.profile.file_concurrency {
            workers.push(tokio::spawn(file_worker(
                Arc::clone(&self.store),
                Arc::clone(&ctx),
                self.emit.clone(),
                session_id.to_string(),
            )));
        }
        for w in workers {
            let _ = w.await;
        }

        if self.store.session_status(session_id) != SessionStatus::Running {
            return Ok(()); // paused / cancelled / error — leave resumable
        }

        let has_errors = self
            .store
            .tasks_for(session_id)?
            .iter()
            .any(|t| t.status == "error")
            || self.store.file_stats(session_id)?.error > 0;

        self.finalize(session_id, has_errors)?;
        Ok(())
    }

    // ---------- finalize ----------

    fn finalize(&self, session_id: &str, has_errors: bool) -> Result<()> {
        let session = self
            .store
            .get_session(session_id)?
            .ok_or_else(|| anyhow::anyhow!("session gone"))?;

        let tasks = self.store.tasks_for(session_id)?;
        let files = self.store.file_stats(session_id)?;
        let errors: Vec<String> = self
            .store
            .recent_events(session_id, 10_000)?
            .into_iter()
            .filter(|e| e.level == "error")
            .map(|e| e.message)
            .collect();

        let manifest = json!({
            "sessionId": session_id,
            "createdAt": session.created_at,
            "finishedAt": chrono::Utc::now().to_rfc3339(),
            "profile": session.profile.as_str(),
            "entities": tasks.iter().filter(|t| t.kind == "entity").map(|t| json!({
                "entity": t.entity, "label": t.label, "status": t.status,
                "records": t.fetched, "error": t.last_error,
            })).collect::<Vec<_>>(),
            "children": {
                "selected": session.config.children,
                "tasks": tasks.iter().filter(|t| t.kind == "child").count(),
                "failed": tasks.iter().filter(|t| t.kind == "child" && t.status == "error").count(),
            },
            "files": files,
            "errors": errors,
        });
        std::fs::write(
            Path::new(&session.export_dir).join("manifest.json"),
            serde_json::to_string_pretty(&manifest)?,
        )?;

        self.store.set_session_status(
            session_id,
            if has_errors {
                SessionStatus::Error
            } else {
                SessionStatus::Complete
            },
        )?;
        self.store.log_event(
            session_id,
            if has_errors { "warn" } else { "info" },
            if has_errors {
                "Extraction finished with errors — see manifest.json"
            } else {
                "Extraction complete"
            },
        );
        (self.emit)(session_id);
        Ok(())
    }
}

// ---------- workers (free functions to satisfy Send bounds) ----------

async fn task_worker(store: Arc<Store>, ctx: Arc<RunCtx>, emit: EmitFn, session_id: String) {
    loop {
        if store.session_status(&session_id) != SessionStatus::Running {
            return;
        }
        let task = match store.claim_task(&session_id) {
            Ok(Some(t)) => t,
            _ => {
                tokio::time::sleep(Duration::from_millis(250)).await;
                match store.remaining_tasks(&session_id) {
                    Ok(0) => return,
                    _ => continue,
                }
            }
        };
        let res = if task.kind == "entity" {
            run_entity_task(&store, &ctx, &session_id, &task, &emit).await
        } else {
            run_child_task(&store, &ctx, &session_id, &task).await
        };
        if let Err(e) = res {
            let msg = e.to_string();
            let _ = store.finish_task(task.id, "error", Some(&msg));
            store.log_event(&session_id, "error", &format!("{}: {msg}", task.label));
        }
        emit(&session_id);
    }
}

async fn file_worker(store: Arc<Store>, ctx: Arc<RunCtx>, emit: EmitFn, session_id: String) {
    loop {
        if store.session_status(&session_id) != SessionStatus::Running {
            return;
        }
        let file = match store.claim_file(&session_id) {
            Ok(Some(f)) => f,
            _ => {
                tokio::time::sleep(Duration::from_millis(300)).await;
                let rf = store.remaining_files(&session_id).unwrap_or(0);
                let rt = store.remaining_tasks(&session_id).unwrap_or(0);
                if rf == 0 && rt == 0 {
                    return;
                }
                continue;
            }
        };
        run_file_task(&store, &ctx, &session_id, &file).await;
        emit(&session_id);
    }
}

// ---------- task runners ----------

async fn run_entity_task(
    store: &Arc<Store>,
    ctx: &Arc<RunCtx>,
    session_id: &str,
    task: &TaskRow,
    emit: &EmitFn,
) -> Result<(), String> {
    let session = store
        .get_session(session_id)
        .map_err(|e| e.to_string())?
        .ok_or("session gone")?;
    let def = entity_spec(&session.config, &task.entity).ok_or(format!("Unknown entity {}", task.entity))?;

    if store.session_status(session_id) != SessionStatus::Running {
        let _ = store.release_task(task.id);
        return Ok(());
    }

    let data_file = Path::new(&session.export_dir)
        .join("data")
        .join(format!("{}.jsonl", def.id));
    let mut page = task.page;
    let mut fetched = task.fetched;
    let mut attempts = task.attempts as u32;

    loop {
        if store.session_status(session_id) != SessionStatus::Running {
            let _ = store.release_task(task.id); // keeps page/fetched checkpoint
            return Ok(());
        }
        if page > MAX_PAGES {
            let _ = store.finish_task(task.id, "error", Some("Exceeded page safety cap"));
            return Ok(());
        }

        let params = [
            ("per_page", ctx.profile.per_page.to_string()),
            ("page", page.to_string()),
        ];
        let resp = with_retry(
            || -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, ApiError>> + Send + '_>> {
                if def.method.as_deref() == Some("POST") {
                    let body = def.body_template.clone().unwrap_or_default()
                        .replace("{page}", &page.to_string())
                        .replace("{per_page}", &ctx.profile.per_page.to_string());
                    Box::pin(ctx.client.post_json(&def.path, serde_json::from_str(&body).unwrap_or(serde_json::json!({}))))
                } else {
                    Box::pin(ctx.client.get_json(&def.path, &params))
                }
            },
            |n, err| {
                attempts = n;
                store.log_event(
                    session_id,
                    "warn",
                    &format!("{} page {page}: retry {n} ({err})", def.label),
                );
            },
        )
        .await
        .map_err(|e| e.to_string())?;

        let rows = extract_rows(&resp, def.data_path.as_deref());
        if !rows.is_empty() {
            let mut buf = String::new();
            for r in &rows {
                buf.push_str(&serde_json::to_string(r).unwrap_or_default());
                buf.push('\n');
            }
            if let Err(e) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&data_file)
                .and_then(|mut f| std::io::Write::write_all(&mut f, buf.as_bytes()))
            {
                return Err(format!("write {}: {e}", data_file.display()));
            }
        }
        fetched += rows.len() as i64;

        let last = is_last_page(&resp, page, rows.len(), ctx.profile.per_page);

        if def.downloadable && session.config.download_files {
            enqueue_files(store, &session, &def.id, &rows)?;
        }
        enqueue_children(store, &session, &def.id, &rows)?;

        page += 1;
        let _ = store.update_task_progress(task.id, page, fetched, attempts as i64);
        emit(session_id);

        if last || rows.is_empty() {
            let _ = store.finish_task(task.id, "done", None);
            store.log_event(session_id, "info", &format!("{}: {fetched} records", def.label));
            return Ok(());
        }
    }
}

async fn run_child_task(
    store: &Arc<Store>,
    ctx: &Arc<RunCtx>,
    session_id: &str,
    task: &TaskRow,
) -> Result<(), String> {
    let session = store
        .get_session(session_id)
        .map_err(|e| e.to_string())?
        .ok_or("session gone")?;
    let def = child_spec(&session.config, &task.entity).ok_or(format!("Unknown child extractor {}", task.entity))?;
    let parent_ref = task.parent_ref.clone().ok_or("missing parent_ref")?;

    if store.session_status(session_id) != SessionStatus::Running {
        let _ = store.release_task(task.id);
        return Ok(());
    }

    let req = def.req(&parent_ref);
    let params: Vec<(&str, String)> = req.params.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
    let mut attempts = task.attempts as u32;
    let resp = with_retry(
        || ctx.client.get_json(&req.path, &params),
        |n, err| {
            attempts = n;
            store.log_event(
                session_id,
                "warn",
                &format!("{} for {}: retry {n} ({err})", def.label, short_id(&parent_ref)),
            );
        },
    )
    .await
    .map_err(|e| e.to_string())?;

    let rows: Vec<Value> = if def.single {
        match resp.as_object().and_then(|o| o.get("data")) {
            Some(d) if d.is_object() => vec![d.clone()],
            _ if resp.is_object() => vec![resp.clone()],
            _ => extract_rows(&resp, None),
        }
    } else {
        extract_rows(&resp, None)
    };

    if !rows.is_empty() {
        let data_file = Path::new(&session.export_dir)
            .join("data")
            .join(format!("{}.jsonl", def.id));
        let mut buf = String::new();
        for r in &rows {
            let mut obj = r.clone();
            if let Some(o) = obj.as_object_mut() {
                o.insert("_parentType".into(), json!(def.parent));
                o.insert("_parentId".into(), json!(parent_ref));
            }
            buf.push_str(&serde_json::to_string(&obj).unwrap_or_default());
            buf.push('\n');
        }
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&data_file)
            .and_then(|mut f| std::io::Write::write_all(&mut f, buf.as_bytes()));
    }
    let _ = store.finish_task(task.id, "done", None);
    let _ = store.update_task_progress(task.id, 1, rows.len() as i64, attempts as i64);
    Ok(())
}

// ---------- enqueue ----------

fn enqueue_children(
    store: &Arc<Store>,
    session: &SessionRow,
    entity_id: &str,
    rows: &[Value],
) -> Result<(), String> {
    let selected: Vec<ResolvedChild> = session
        .config
        .children
        .iter()
        .filter_map(|cid| child_spec(&session.config, cid))
        .filter(|c| c.parent == entity_id)
        .collect();
    if selected.is_empty() || rows.is_empty() {
        return Ok(());
    }
    for rec in rows {
        let Some(pid) = record_id(rec) else { continue };
        for def in &selected {
            let req = def.req(&pid);
            let params_json = if req.params.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&req.params).unwrap_or_default())
            };
            let _ = store.add_task(
                &session.id,
                "child",
                &def.id,
                &format!("{} · {}", def.label, short_id(&pid)),
                &req.path,
                Some(&pid),
                params_json.as_deref(),
            );
        }
    }
    Ok(())
}

fn enqueue_files(
    store: &Arc<Store>,
    session: &SessionRow,
    entity_id: &str,
    rows: &[Value],
) -> Result<(), String> {
    for rec in rows {
        let Some(url) = rec.get("url").and_then(|v| v.as_str()) else {
            continue;
        };
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            continue;
        }
        let rid = record_id(rec).unwrap_or_else(|| "noref".into());
        let filename = attachment_filename(rec, url);
        let dest = Path::new(&session.export_dir)
            .join("files")
            .join(entity_id)
            .join(format!("{}-{filename}", sanitize(&rid)));
        let _ = store.add_file(
            &session.id,
            entity_id,
            Some(&rid),
            url,
            &dest.to_string_lossy(),
            Some(&filename),
        );
    }
    Ok(())
}

async fn run_file_task(store: &Arc<Store>, ctx: &Arc<RunCtx>, session_id: &str, file: &FileRow) {
    let dest = PathBuf::from(&file.dest_path);
    let mut attempts = file.attempts as u32;

    let result = with_retry(
        || {
            let d = dest.clone();
            let url = file.url.clone();
            let dl = ctx.downloader.clone();
            async move {
                dl.download(&url, &d)
                    .await
                    .map_err(|e| dl_err_to_api(&e))
            }
        },
        |n, err| {
            attempts = n;
            store.log_event(
                session_id,
                "warn",
                &format!(
                    "File {}: retry {n} ({err})",
                    file.filename.as_deref().unwrap_or(&file.url)
                ),
            );
            let done = Downloader::bytes_on_disk(&dest);
            if done > 0 {
                let _ = store.update_file(file.id, None, Some(done as i64), None, Some(n as i64), None);
            }
        },
    )
    .await;

    match result {
        Ok(DownloadOutcome::Done { bytes }) | Ok(DownloadOutcome::Skipped { bytes }) => {
            let status = if matches!(result, Ok(DownloadOutcome::Skipped { .. })) {
                "skipped"
            } else {
                "done"
            };
            let _ = store.update_file(
                file.id,
                Some(status),
                Some(bytes as i64),
                None,
                Some(attempts as i64),
                None,
            );
        }
        Err(e) => {
            let done = Downloader::bytes_on_disk(&dest);
            let _ = store.update_file(
                file.id,
                Some("error"),
                Some(done as i64),
                None,
                Some(attempts as i64),
                Some(&e.to_string()),
            );
            store.log_event(
                session_id,
                "error",
                &format!(
                    "File failed {}: {e}",
                    file.filename.as_deref().unwrap_or(&file.url)
                ),
            );
        }
    }
}

/// Map anyhow download errors into ApiError for the retry loop:
/// network/truncation failures are retryable, 4xx are not.
fn dl_err_to_api(e: &anyhow::Error) -> ApiError {
    let msg = e.to_string();
    let status = msg
        .strip_prefix("Download failed: ")
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);
    let retryable = status == 0 || status == 429 || status >= 500;
    ApiError {
        message: msg,
        status,
        retry_after: None,
        retryable,
    }
}

fn short_id(id: &str) -> String {
    if id.len() > 12 {
        format!("{}…", &id[..8])
    } else {
        id.to_string()
    }
}

fn sanitize(s: &str) -> String {
    let out: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .take(64)
        .collect();
    if out.is_empty() {
        "id".into()
    } else {
        out
    }
}

pub enum OAuthOutcome {
    Token(String),
    /// The provider requires a second-factor code to continue.
    Needs2fa(Option<String>),
}

/// POST {host}/oauth/token with client credentials → access_token, or a
/// 2FA challenge. `otp` retries a challenge once the user supplies a code.
pub async fn fetch_oauth_token(
    host: &str,
    client_id: &str,
    client_secret: &str,
    otp: Option<&str>,
) -> Result<OAuthOutcome> {
    let resp = reqwest::Client::new()
        .post(format!("{}/oauth/token", host.trim_end_matches('/')))
        .json(&serde_json::json!({
            "grant_type": "client_credentials",
            "client_id": client_id,
            "client_secret": client_secret,
            "otp": otp,
        }))
        .send()
        .await?;
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    let needs_2fa = ["requires_2fa", "mfa_required", "otp_required", "two_factor_required"]
        .iter()
        .any(|k| body.get(*k).and_then(|v| v.as_bool()).unwrap_or(false))
        || body
            .get("error")
            .or_else(|| body.get("message"))
            .and_then(|m| m.as_str())
            .map(|m| { let m = m.to_lowercase(); m.contains("2fa") || m.contains("otp") || m.contains("mfa") })
            .unwrap_or(false);
    if needs_2fa {
        let hint = body
            .get("otp_hint")
            .or_else(|| body.get("delivery"))
            .and_then(|m| m.as_str())
            .map(|s| s.to_string());
        return Ok(OAuthOutcome::Needs2fa(hint));
    }
    if !status.is_success() {
        anyhow::bail!("token endpoint {status}: {}", body.get("message").or(body.get("error")).map(|m| m.as_str().unwrap_or_default()).unwrap_or_default());
    }
    body.get("access_token")
        .and_then(|t| t.as_str())
        .map(|s| OAuthOutcome::Token(s.to_string()))
        .ok_or_else(|| anyhow::anyhow!("token response missing access_token"))
}
