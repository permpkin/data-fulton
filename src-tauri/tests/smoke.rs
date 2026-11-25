/**
 * End-to-end smoke test: runs the real extraction engine against a mock
 * Prime API server — no Tauri needed, core logic only.
 *
 * Verifies:
 *  - paginated entity extraction → JSONL files
 *  - per-job child extractors
 *  - attachment file download (+ Range resume)
 *  - pause/resume across a session
 *  - 429 handling + rate limiting
 *  - READ-ONLY: asserts the server only ever saw GET requests
 */
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use fulton_lib::api::TokenProvider;
use fulton_lib::downloader::{DownloadOutcome, Downloader};
use fulton_lib::extractor::Extractor;
use fulton_lib::limiter::ConcurrencyGate;
use fulton_lib::store::Store;
use fulton_lib::types::*;

// ---------- mock server ----------

struct MockState {
    seen_methods: Mutex<Vec<String>>,
    contacts_429ed: AtomicBool,
    port: u16,
}

async fn mock_server() -> (SocketAddr, Arc<MockState>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let state = Arc::new(MockState {
        seen_methods: Mutex::new(vec![]),
        contacts_429ed: AtomicBool::new(false),
        port: addr.port(),
    });
    let st = state.clone();
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let st = st.clone();
            tokio::spawn(handle(stream, st));
        }
    });
    (addr, state)
}

fn jobs() -> Vec<serde_json::Value> {
    (1..=5)
        .map(|i| json!({ "id": format!("job-{i}"), "name": format!("Job {i}") }))
        .collect()
}

fn file_content(name: &str) -> Vec<u8> {
    format!("FAKE-{name}-").repeat(100).into_bytes()
}

fn respond_json(status: u16, body: serde_json::Value, extra: &str) -> String {
    let b = body.to_string();
    format!(
        "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n{extra}\r\n{b}",
        b.len()
    )
}

async fn handle(mut stream: tokio::net::TcpStream, state: Arc<MockState>) {
    let mut reader = BufReader::new(&mut stream);
    let mut head = String::new();
    let mut line = String::new();
    let mut headers: HashMap<String, String> = HashMap::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
            return;
        }
        let t = line.trim_end();
        if t.is_empty() {
            break;
        }
        head.push_str(t);
        head.push('\n');
        if let Some((k, v)) = t.split_once(':') {
            headers.insert(k.trim().to_lowercase(), v.trim().to_string());
        }
    }
    let mut parts = head.split_whitespace();
    let method = parts.next().unwrap_or("?").to_string();
    let raw_path = parts.next().unwrap_or("/").to_string();
    state.seen_methods.lock().unwrap().push(method.clone());

    let (path, query) = raw_path.split_once('?').unwrap_or((&raw_path, ""));
    let params: HashMap<String, String> = query
        .split('&')
        .filter(|p| !p.is_empty())
        .filter_map(|p| p.split_once('=').map(|(k, v)| (k.into(), urldecode(v))))
        .collect();

    let out = if path == "/api.prime/v2/jobs" {
        let page: i64 = params.get("page").and_then(|p| p.parse().ok()).unwrap_or(1);
        // 3-page shape regardless of per_page (meta.last_page drives pagination)
        let per = 2i64;
        let all = jobs();
        let start = ((page - 1) * per) as usize;
        let slice: Vec<_> = all.iter().skip(start).take(per as usize).cloned().collect();
        let last = ((all.len() as i64) + per - 1) / per;
        respond_json(
            200,
            json!({ "data": slice, "meta": { "current_page": page, "last_page": last, "per_page": per, "total": all.len() } }),
            "",
        )
    } else if path.starts_with("/api.prime/v2/jobs/") && path.ends_with("/notes") {
        let id = path.split('/').nth(4).unwrap_or("?");
        respond_json(200, json!({ "data": { "jobId": id, "notes": format!("Notes for {id}") } }), "")
    } else if path == "/api.prime/v2/histories" {
        let q = params.get("q").cloned().unwrap_or_default();
        let id = q.split("eq('").nth(1).and_then(|s| s.split('\'').next()).unwrap_or("");
        respond_json(200, json!({ "data": [{ "jobId": id, "action": "created" }] }), "")
    } else if path == "/api.prime/v2/contacts" {
        if !state.contacts_429ed.swap(true, Ordering::SeqCst) {
            "HTTP/1.1 429 Too Many Requests\r\nretry-after: 1\r\ncontent-length: 0\r\nconnection: close\r\n\r\n".to_string()
        } else {
            respond_json(200, json!({ "data": [{ "id": "c-1", "name": "Alice" }], "meta": { "last_page": 1, "total": 1 } }), "")
        }
    } else if path == "/api.prime/v2/attachments" {
        let base = format!("http://127.0.0.1:{}", state.port);
        respond_json(
            200,
            json!({ "data": [
                { "id": "att-1", "url": format!("{base}/files/report.pdf"), "name": "report.pdf" },
                { "id": "att-2", "url": format!("{base}/files/photo.jpg"), "name": "site photo.jpg" }
            ], "meta": { "last_page": 1, "total": 2 } }),
            "",
        )
    } else if path.starts_with("/files/") {
        let name = path.rsplit('/').next().unwrap();
        let buf = file_content(name);
        if let Some(range) = headers.get("range") {
            if let Some(start) = range
                .strip_prefix("bytes=")
                .and_then(|s| s.split('-').next())
                .and_then(|s| s.parse::<usize>().ok())
            {
                let part = &buf[start.min(buf.len())..];
                return write_all(
                    &mut stream,
                    &format!(
                        "HTTP/1.1 206 Partial Content\r\ncontent-length: {}\r\ncontent-range: bytes {}-{}/{}\r\nconnection: close\r\n\r\n",
                        part.len(), start, buf.len() - 1, buf.len()
                    ),
                    Some(part),
                )
                .await;
            }
        }
        return write_all(
            &mut stream,
            &format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\naccept-ranges: bytes\r\nconnection: close\r\n\r\n",
                buf.len()
            ),
            Some(&buf),
        )
        .await;
    } else {
        respond_json(404, json!({ "error": "not found" }), "")
    };

    write_all(&mut stream, &out, None).await;
}

async fn write_all(stream: &mut tokio::net::TcpStream, head: &str, body: Option<&[u8]>) {
    let _ = stream.write_all(head.as_bytes()).await;
    if let Some(b) = body {
        let _ = stream.write_all(b).await;
    }
    let _ = stream.shutdown().await;
}

fn urldecode(s: &str) -> String {
    let mut out = Vec::with_capacity(s.len());
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("00"), 16) {
                out.push(v);
                i += 3;
                continue;
            }
        } else if b[i] == b'+' {
            out.push(b' ');
            i += 1;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

// ---------- the test ----------

fn config(dest: &std::path::Path, entities: &[&str], children: &[&str]) -> SessionConfig {
    SessionConfig {
        entities: entities.iter().map(|s| s.to_string()).collect(),
        children: children.iter().map(|s| s.to_string()).collect(),
        download_files: true,
        profile: SpeedProfile::Aggressive,
        dest_dir: dest.to_string_lossy().into(),
        name: String::new(),
        entity_specs: vec![],
        child_specs: vec![],
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn end_to_end() {
    let (addr, state) = mock_server().await;
    let tmp = std::env::temp_dir().join(format!("datafulton-smoke-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&tmp).unwrap();

    let store = Arc::new(Store::memory().unwrap());
    let token: TokenProvider = Arc::new(|| Some("test-token".into()));
    let conn = ConnectionRow {
        id: "conn-1".into(),
        created_at: "now".into(),
        provider: "primeeco".into(),
        name: "test".into(),
        host: format!("http://{addr}/api.prime"),
        api_key: "test-token".into(),
        last_ok: None,
    };
    store.create_connection(&conn).unwrap();
    let downloader = Downloader::new(
        Arc::new(ConcurrencyGate::new(3)),
        format!("http://{addr}"),
        token,
    );
    let extractor = Arc::new(Extractor::new(store.clone(), Arc::new(|_| {})));

    // ---- session 1: full extraction ----
    let cfg = config(&tmp, &["jobs", "contacts", "attachments"], &["job-notes", "job-histories"]);
    let (sid, export_dir) = extractor.create_session("conn-1", &cfg).unwrap();
    extractor.clone().start(&sid).unwrap();
    extractor.wait_for(&sid).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let session = store.get_session(&sid).unwrap().unwrap();
    assert_eq!(session.status, SessionStatus::Complete, "session should complete");

    let jobs_rows = std::fs::read_to_string(export_dir.join("data/jobs.jsonl")).unwrap();
    assert_eq!(jobs_rows.trim().lines().count(), 5, "all 5 jobs extracted");

    let contacts = std::fs::read_to_string(export_dir.join("data/contacts.jsonl")).unwrap();
    assert_eq!(contacts.trim().lines().count(), 1, "contacts extracted after 429 retry");
    assert!(state.contacts_429ed.load(Ordering::SeqCst), "429 path exercised");

    let notes = std::fs::read_to_string(export_dir.join("data/job-notes.jsonl")).unwrap();
    assert_eq!(notes.trim().lines().count(), 5, "one notes row per job");
    assert!(notes.contains("_parentType"), "child rows carry parent ref");

    let hists = std::fs::read_to_string(export_dir.join("data/job-histories.jsonl")).unwrap();
    assert_eq!(hists.trim().lines().count(), 5);

    let f1 = export_dir.join("files/attachments/att-1-report.pdf");
    let f2 = export_dir.join("files/attachments/att-2-site photo.jpg");
    assert!(f1.exists() && f2.exists(), "attachment files downloaded");
    assert_eq!(std::fs::read(&f1).unwrap(), file_content("report.pdf"));

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(export_dir.join("manifest.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["entities"].as_array().unwrap().len(), 3);

    // READ-ONLY guarantee: every request the server saw was a GET.
    let methods = state.seen_methods.lock().unwrap();
    assert!(methods.iter().all(|m| m == "GET"), "non-GET seen: {:?}", methods);
    drop(methods);

    // ---- session 2: pause/resume ----
    let cfg2 = config(&tmp, &["jobs"], &[]);
    let (sid2, _) = extractor.create_session("conn-1", &cfg2).unwrap();
    extractor.clone().start(&sid2).unwrap();
    extractor.pause(&sid2).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    let s2 = store.get_session(&sid2).unwrap().unwrap();
    assert!(
        matches!(s2.status, SessionStatus::Paused | SessionStatus::Complete),
        "session should pause (or race to complete), got {:?}",
        s2.status
    );
    if s2.status == SessionStatus::Paused {
        extractor.clone().resume(&sid2).unwrap();
        extractor.wait_for(&sid2).await;
        assert_eq!(
            store.get_session(&sid2).unwrap().unwrap().status,
            SessionStatus::Complete
        );
    }

    // ---- range resume: truncate to .part, re-download ----
    let target = export_dir.join("files/attachments/att-1-report.pdf");
    let part = format!("{}.part", target.display());
    let full = file_content("report.pdf");
    std::fs::write(&part, &full[..full.len() / 2]).unwrap();
    std::fs::remove_file(&target).unwrap();
    let r = downloader
        .download(&format!("http://{addr}/files/report.pdf"), &target)
        .await
        .unwrap();
    let bytes = match r {
        DownloadOutcome::Done { bytes } | DownloadOutcome::Skipped { bytes } => bytes,
    };
    assert_eq!(bytes as usize, full.len(), "range-resumed file complete");

    println!("smoke: all assertions passed ({} requests, all GET)", state.seen_methods.lock().unwrap().len());
}
