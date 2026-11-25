use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::api::PrimeClient;
use crate::catalog::catalog;
use crate::extractor::Extractor;
use crate::keyhelper::open_key_helper;
use crate::limiter::{profile, RateGate};
use crate::store::Store;
use crate::types::*;

pub const DEFAULT_HOST: &str = "https://www.primeeco.tech/api.prime";

/// Host fallback per provider spec (when the field is left blank).
fn default_host_for(provider: &str) -> &'static str {
    match provider {
        "buildpass" => "https://api.buildpass.global",
        "procore" => "https://api.procore.com",
        "simpro" => "https://mysimpro.simprosuite.com",
        "aroflow" => "https://api.aroflo.com",
        "monday" => "https://api.monday.com",
        _ => DEFAULT_HOST,
    }
}

const SETTINGS_DEST: &str = "settings.destDir";
const SETTINGS_PROFILE: &str = "settings.profile";

pub struct AppState {
    pub store: Arc<Store>,
    pub extractor: Arc<Extractor>,
    /// The connection the extractions window is bound to.
    pub selected: Mutex<Option<String>>,
}

/// (Re)create the connections launcher window.
pub fn open_connections_window(app: &AppHandle) {
    let _ = tauri::WebviewWindowBuilder::new(
        app,
        "connections",
        tauri::WebviewUrl::App("index.html#connections".into()),
    )
    .title("DataFulton")
    .inner_size(440.0, 340.0)
    .min_inner_size(400.0, 340.0)
    .title_bar_style(tauri::TitleBarStyle::Transparent)
    .visible(false)
    .on_page_load(|w, _| {
        let _ = w.show();
    })
    .build();
}

/// Create the main extractions window.
fn open_main_window(app: &AppHandle) {
    let _ = tauri::WebviewWindowBuilder::new(
        app,
        "main",
        tauri::WebviewUrl::App("index.html#main".into()),
    )
    .title("DataFulton — Extractions")
    .inner_size(560.0, 520.0)
    .min_inner_size(480.0, 520.0)
    .title_bar_style(tauri::TitleBarStyle::Transparent)
    .visible(false)
    .on_page_load(|w, _| {
        let _ = w.show();
    })
    .build();
}

/// Build the shared AppState; called once from setup().
pub fn build_state(app: &AppHandle, store: Arc<Store>) -> AppState {
    let app_handle = app.clone();
    let last_emit = Arc::new(Mutex::new(Instant::now() - Duration::from_secs(1)));
    let pending = Arc::new(Mutex::new(None::<String>));

    let push_snapshot: crate::extractor::EmitFn = {
        let app = app_handle.clone();
        let store = Arc::clone(&store);
        Arc::new(move |sid: &str| {
            if let Ok(Some(session)) = store.get_session(sid) {
                let snap = SessionSnapshot {
                    session,
                    tasks: store.tasks_for(sid).unwrap_or_default(),
                    files: store.file_stats(sid).unwrap_or_default(),
                    recent_events: store.recent_events(sid, 60).unwrap_or_default(),
                };
                let _ = app.emit("session:update", snap);
            }
        })
    };

    // Trailing-debounce emit: bursty progress → ~3 pushes/sec + delayed final.
    let emit: crate::extractor::EmitFn = {
        let push = push_snapshot.clone();
        Arc::new(move |sid: &str| {
            let now = Instant::now();
            let mut l = last_emit.lock().unwrap();
            if now.duration_since(*l) >= Duration::from_millis(350) {
                *l = now;
                push(sid);
            } else {
                let mut p = pending.lock().unwrap();
                if p.is_none() {
                    *p = Some(sid.to_string());
                    let push2 = push.clone();
                    let pending2 = pending.clone();
                    let sid2 = sid.to_string();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(400)).await;
                        let mut p2 = pending2.lock().unwrap();
                        if p2.take().is_some() {
                            drop(p2);
                            push2(&sid2);
                        }
                    });
                }
            }
        })
    };

    AppState {
        extractor: Arc::new(Extractor::new(Arc::clone(&store), emit)),
        store,
        selected: Mutex::new(None),
    }
}

// ---- settings ----

#[tauri::command]
pub fn settings_get(state: State<'_, AppState>) -> AppSettings {
    AppSettings {
        dest_dir: state.store.kv_get(SETTINGS_DEST).unwrap_or_default(),
        profile: match state.store.kv_get(SETTINGS_PROFILE).as_deref() {
            Some("aggressive") => SpeedProfile::Aggressive,
            _ => SpeedProfile::Safe,
        },
    }
}

#[tauri::command]
pub fn settings_set(settings: AppSettings, state: State<'_, AppState>) -> Result<(), String> {
    state
        .store
        .kv_set(SETTINGS_DEST, &settings.dest_dir)
        .map_err(|e| e.to_string())?;
    state
        .store
        .kv_set(SETTINGS_PROFILE, settings.profile.as_str())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn choose_dir(app: AppHandle) -> Result<Option<String>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_folder(move |path| {
        let _ = tx.send(path);
    });
    let path = rx.await.map_err(|e| e.to_string())?;
    Ok(path
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into()))
}

// ---- connections ----

#[tauri::command]
pub fn open_connections(app: tauri::AppHandle, window: tauri::WebviewWindow) -> Result<(), String> {
    open_connections_window(&app);
    let _ = window.close();
    Ok(())
}

#[tauri::command]
pub fn connections_list(state: State<'_, AppState>) -> Result<Vec<ConnectionRow>, String> {
    state.store.list_connections().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn connection_save(
    input: ConnectionInput,
    state: State<'_, AppState>,
) -> Result<ConnectionRow, String> {
    let oauth = matches!(input.provider.as_str(), "buildpass" | "procore" | "simpro");
    if oauth {
        let cid = input.credentials.get("clientId").map(|s| s.trim()).unwrap_or("");
        let csec = input.credentials.get("clientSecret").map(|s| s.trim()).unwrap_or("");
        if cid.is_empty() || csec.is_empty() {
            return Err("Client ID and secret are required".into());
        }
    } else if input.api_key.trim().len() < 4 {
        return Err("API key looks too short".into());
    }
    let host = if input.host.trim().is_empty() {
        default_host_for(&input.provider).to_string()
    } else {
        input.host.trim().trim_end_matches('/').to_string()
    };
    let row = match input.id {
        Some(id) => {
            let mut c = state
                .store
                .get_connection(&id)
                .map_err(|e| e.to_string())?
                .ok_or("unknown connection")?;
            c.name = input.name.trim().to_string();
            c.provider = input.provider.trim().to_string();
            c.credentials = input.credentials.clone();
            c.host = host;
            c.api_key = input.api_key.trim().to_string();
            c.last_ok = None;
            state.store.update_connection(&c).map_err(|e| e.to_string())?;
            c
        }
        None => {
            let c = ConnectionRow {
                id: uuid::Uuid::new_v4().to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
                credentials: input.credentials.clone(),
                provider: input.provider.trim().to_string(),
                name: if input.name.trim().is_empty() {
                    host.clone()
                } else {
                    input.name.trim().to_string()
                },
                host,
                api_key: input.api_key.trim().to_string(),
                last_ok: None,
            };
            state.store.create_connection(&c).map_err(|e| e.to_string())?;
            c
        }
    };
    Ok(row)
}

#[tauri::command]
pub fn connection_delete(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.store.delete_connection(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn connection_validate(
    id: String,
    state: State<'_, AppState>,
) -> Result<ValidateResult, String> {
    let conn = state
        .store
        .get_connection(&id)
        .map_err(|e| e.to_string())?
        .ok_or("unknown connection")?;
    let key = conn.api_key.clone();
    let client = PrimeClient::new(
        &conn.host,
        Arc::new(move || Some(key.clone())),
        Arc::new(RateGate::new(profile(SpeedProfile::Aggressive))),
    );
    let params = [("per_page", "1".to_string()), ("limit", "1".to_string())];
    if matches!(conn.provider.as_str(), "buildpass" | "procore" | "simpro") {
        let cid = conn.credentials.get("clientId").cloned().unwrap_or_default();
        let csec = conn.credentials.get("clientSecret").cloned().unwrap_or_default();
        let result = if cid.is_empty() || csec.is_empty() {
            ValidateResult { ok: false, message: "Client ID and secret are required".into(), needs_2fa: false, otp_hint: None }
        } else {
            match crate::extractor::fetch_oauth_token(&conn.host, &cid, &csec, None).await {
                Ok(crate::extractor::OAuthOutcome::Token(_)) =>
                    ValidateResult { ok: true, message: "Connected — token exchange succeeded.".into(), needs_2fa: false, otp_hint: None },
                Ok(crate::extractor::OAuthOutcome::Needs2fa(hint)) =>
                    ValidateResult { ok: false, message: "Two-factor code required".into(), needs_2fa: true, otp_hint: hint },
                Err(e) => ValidateResult { ok: false, message: e.to_string(), needs_2fa: false, otp_hint: None },
            }
        };
        if let Ok(Some(mut c)) = state.store.get_connection(&id) {
            c.last_ok = Some(result.ok);
            let _ = state.store.update_connection(&c);
        }
        return Ok(result);
    }
    let probe = if conn.provider == "monday" { "/v2/get_schema" } else { "/v2/users" };
    let result = match client.get_json(probe, &params).await {
        Ok(_) => ValidateResult {
            ok: true,
            message: "Connected — API key is valid.".into(),
            needs_2fa: false,
            otp_hint: None,
        },
        Err(e) => ValidateResult {
            ok: false,
            message: e.to_string(),
            needs_2fa: false,
            otp_hint: None,
        },
    };
    // Remember the last validation outcome on the connection.
    if let Ok(Some(mut c)) = state.store.get_connection(&id) {
        c.last_ok = Some(result.ok);
        let _ = state.store.update_connection(&c);
    }
    Ok(result)
}

/// Validate raw connection input without persisting anything.
#[tauri::command]
pub async fn connection_test(input: ConnectionInput, otp: Option<String>) -> Result<ValidateResult, String> {
    let key = input.api_key.trim().to_string();
    let host = input.host.trim().trim_end_matches('/');
    if host.is_empty() {
        return Ok(ValidateResult { ok: false, message: "Host is required".into(), needs_2fa: false, otp_hint: None });
    }
    let client = PrimeClient::new(
        host,
        Arc::new(move || Some(key.clone())),
        Arc::new(RateGate::new(profile(SpeedProfile::Aggressive))),
    );
    if matches!(input.provider.as_str(), "buildpass" | "procore" | "simpro") {
        // OAuth client-credentials: exchange for a token, then probe.
        let cid = input.credentials.get("clientId").cloned().unwrap_or_default();
        let csec = input.credentials.get("clientSecret").cloned().unwrap_or_default();
        if cid.is_empty() || csec.is_empty() {
            return Ok(ValidateResult { ok: false, message: "Client ID and secret are required".into(), needs_2fa: false, otp_hint: None });
        }
        match crate::extractor::fetch_oauth_token(host, &cid, &csec, otp.as_deref()).await {
            Ok(crate::extractor::OAuthOutcome::Token(_)) =>
                return Ok(ValidateResult { ok: true, message: "Connected — token exchange succeeded.".into(), needs_2fa: false, otp_hint: None }),
            Ok(crate::extractor::OAuthOutcome::Needs2fa(hint)) =>
                return Ok(ValidateResult { ok: false, message: "Two-factor code required".into(), needs_2fa: true, otp_hint: hint }),
            Err(e) => return Ok(ValidateResult { ok: false, message: e.to_string(), needs_2fa: false, otp_hint: None }),
        }
    }
    match client.get_json(if input.provider == "monday" { "/v2/get_schema" } else { "/v2/users" }, &[("per_page", "1".to_string()), ("limit", "1".to_string())]).await {
        Ok(_) => Ok(ValidateResult { ok: true, message: "Connected — API key is valid.".into(), needs_2fa: false, otp_hint: None }),
        Err(e) => Ok(ValidateResult { ok: false, message: e.to_string(), needs_2fa: false, otp_hint: None }),
    }
}

/// Select a connection, open the extractions window and close the
/// connections window. No network probe — keys are tested only from the
/// connection settings form ("Save & test").
#[tauri::command]
pub fn connection_open(
    id: String,
    window: tauri::WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ValidateResult, String> {
    let conn = state
        .store
        .get_connection(&id)
        .map_err(|e| e.to_string())?
        .ok_or("unknown connection")?;
    *state.selected.lock().unwrap() = Some(id);
    open_main_window(&app);
    let _ = window.close();
    Ok(ValidateResult {
        ok: true,
        message: format!("Opened {}", conn.name),
        needs_2fa: false,
        otp_hint: None,
    })
}

/// The connection bound to the extractions window.
#[tauri::command]
pub fn connection_current(state: State<'_, AppState>) -> Result<Option<ConnectionRow>, String> {
    let id = state.selected.lock().unwrap().clone();
    match id {
        Some(id) => state.store.get_connection(&id).map_err(|e| e.to_string()),
        None => Ok(None),
    }
}

/// Open the "find my key" helper at this connection's site origin.
#[tauri::command]
pub fn connection_key_helper(id: String, app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let conn = state
        .store
        .get_connection(&id)
        .map_err(|e| e.to_string())?
        .ok_or("unknown connection")?;
    let site = reqwest::Url::parse(&conn.host)
        .map(|u| u.origin().ascii_serialization())
        .unwrap_or_else(|_| "https://www.primeeco.tech".into());
    open_key_helper(&app, &site).map_err(|e| e.to_string())
}

// ---- catalog ----

#[tauri::command]
pub fn catalog_list() -> Catalog {
    catalog()
}

// ---- sessions ----

#[tauri::command]
pub fn session_create(
    connection_id: String,
    config: SessionConfig,
    state: State<'_, AppState>,
) -> Result<NewSessionResult, String> {
    if config.entities.is_empty() {
        return Err("Select at least one data type".into());
    }
    if config.dest_dir.trim().is_empty() {
        return Err("Choose a destination folder".into());
    }
    if state
        .store
        .get_connection(&connection_id)
        .map_err(|e| e.to_string())?
        .is_none()
    {
        return Err("unknown connection".into());
    }
    let (session_id, export_dir) = state
        .extractor
        .create_session(&connection_id, &config)
        .map_err(|e| e.to_string())?;
    Ok(NewSessionResult {
        session_id,
        export_dir: export_dir.to_string_lossy().into(),
    })
}

#[tauri::command]
pub async fn session_start(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.extractor.clone().start(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn session_pause(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.extractor.pause(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn session_resume(id: String, state: State<'_, AppState>) -> Result<(), String> {
    if let Ok(Some(s)) = state.store.get_session(&id) {
        if matches!(s.status, crate::types::SessionStatus::Error | crate::types::SessionStatus::Cancelled) {
            let _ = state.store.reset_error_tasks(&id);
        }
    }
    state.extractor.clone().resume(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn session_restart(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.store.reset_session_tasks(&id).map_err(|e| e.to_string())?;
    // Wipe previously exported rows/files so the fresh run doesn't append duplicates.
    if let Ok(Some(s)) = state.store.get_session(&id) {
        let dir = std::path::Path::new(&s.export_dir);
        let _ = std::fs::remove_dir_all(dir.join("data"));
        let _ = std::fs::remove_dir_all(dir.join("files"));
        let _ = std::fs::create_dir_all(dir.join("data"));
        let _ = std::fs::create_dir_all(dir.join("files"));
    }
    state.extractor.clone().resume(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn session_cancel(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.extractor.cancel(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn session_list(connection_id: String, state: State<'_, AppState>) -> Result<Vec<SessionRow>, String> {
    state
        .store
        .sessions_for_connection(&connection_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn session_snapshot(id: String, state: State<'_, AppState>) -> Option<SessionSnapshot> {
    let session = state.store.get_session(&id).ok().flatten()?;
    Some(SessionSnapshot {
        session,
        tasks: state.store.tasks_for(&id).unwrap_or_default(),
        files: state.store.file_stats(&id).unwrap_or_default(),
        recent_events: state.store.recent_events(&id, 60).unwrap_or_default(),
    })
}

/// Update profile + downloadFiles for a resumable (non-running) session.
#[tauri::command]
pub fn session_update_config(
    id: String,
    profile: SpeedProfile,
    download_files: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let s = state
        .store
        .get_session(&id)
        .map_err(|e| e.to_string())?
        .ok_or("unknown session")?;
    if s.status == SessionStatus::Running {
        return Err("Pause the session before changing settings".into());
    }
    state
        .store
        .update_session_config(&id, profile, download_files)
        .map_err(|e| e.to_string())
}

/// Replace the whole config of a non-running session (pending-session editing).
#[tauri::command]
pub fn session_save_config(id: String, config: SessionConfig, state: State<'_, AppState>) -> Result<(), String> {
    let s = state
        .store
        .get_session(&id)
        .map_err(|e| e.to_string())?
        .ok_or("unknown session")?;
    if s.status == SessionStatus::Running {
        return Err("Pause the session before changing settings".into());
    }
    state
        .store
        .save_session_config(&id, &config)
        .map_err(|e| e.to_string())
}

/// Delete a non-running session's bookkeeping (exported files stay on disk).
#[tauri::command]
pub fn session_delete(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let s = state
        .store
        .get_session(&id)
        .map_err(|e| e.to_string())?
        .ok_or("unknown session")?;
    if s.status == SessionStatus::Running {
        return Err("Stop the session before deleting it".into());
    }
    state.store.delete_session(&id).map_err(|e| e.to_string())
}

/// Write the session's event log to a file the user picked.
#[tauri::command]
pub fn session_export_log(id: String, path: String, state: State<'_, AppState>) -> Result<(), String> {
    let events = state
        .store
        .recent_events(&id, 1_000_000)
        .map_err(|e| e.to_string())?;
    let body = events
        .iter()
        .map(|e| format!("{} [{}] {}", e.ts, e.level, e.message))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&path, body).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn session_clear_finished(
    state: State<'_, AppState>,
    connection_id: String,
) -> Result<u64, String> {
    let n = state
        .store
        .clear_finished_sessions(&connection_id)
        .map_err(|e| e.to_string())?;
    Ok(n)
}

#[tauri::command]
pub fn session_open_export(id: String, app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let s = state
        .store
        .get_session(&id)
        .map_err(|e| e.to_string())?
        .ok_or("unknown session")?;
    app.opener()
        .open_path(&s.export_dir, None::<&str>)
        .map_err(|e| e.to_string())
}
