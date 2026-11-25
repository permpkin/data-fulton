use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::types::KeyFoundEvent;

const PRIME_HOME: &str = "https://www.primeeco.tech";

/// Site URL to open — the origin of the connection's API host.
fn site_origin(host: &str) -> String {
    reqwest::Url::parse(host)
        .map(|u| u.origin().ascii_serialization())
        .unwrap_or_else(|_| PRIME_HOME.into())
}

/**
 * JS injected into the helper window on a timer. Scans web storage for
 * token-shaped keys; when found it navigates to #datafulton-token:<token> which
 * the Rust `on_navigation` hook picks up (no remote IPC needed).
 */
const SCAN_JS: &str = r#"(() => {
  try {
    const pat = /token|api[_-]?key|bearer/i;
    for (const store of [localStorage, sessionStorage]) {
      for (let i = 0; i < store.length; i++) {
        const k = store.key(i);
        const v = store.getItem(k);
        if (k && v && pat.test(k)) {
          const m = String(v).match(/eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+/);
          const token = m ? m[0] : (String(v).length >= 16 && String(v).length <= 512 ? String(v) : null);
          if (token) {
            location.hash = 'datafulton-token:' + encodeURIComponent(token);
            return;
          }
        }
      }
    }
    const cm = document.cookie.match(/(?:api[_-]?key|token)=([^;]{16,512})/i);
    if (cm) location.hash = 'datafulton-token:' + encodeURIComponent(cm[1]);
  } catch (e) {}
})()"#;

/**
 * "Find my key" helper: opens the Prime site in a Tauri window so the user
 * can log in; watches web storage for a bearer-token-shaped value and pushes
 * it back to the main window. Manual paste always remains available.
 */
pub fn open_key_helper(app: &AppHandle, host: &str) -> tauri::Result<()> {
    if let Some(existing) = app.get_webview_window("key-helper") {
        existing.set_focus()?;
        return Ok(());
    }

    let site = site_origin(host);
    let app_handle = app.clone();
    let win = WebviewWindowBuilder::new(
        app,
        "key-helper",
        WebviewUrl::External(site.parse().expect("valid url")),
    )
    .title("Sign in to Prime — DataFulton is watching for an API key")
    .inner_size(1100.0, 800.0)
    .on_navigation(move |url| {
        if let Some(hash) = url.fragment() {
            if let Some(encoded) = hash.strip_prefix("datafulton-token:") {
                if let Ok(token) = urlencoding_decode(encoded) {
                    let _ = app_handle.emit(
                        "key:found",
                        KeyFoundEvent {
                            token,
                            source: "storage".into(),
                        },
                    );
                    if let Some(w) = app_handle.get_webview_window("key-helper") {
                        let _ = w.close();
                    }
                }
            }
        }
        true
    })
    .build()?;

    let closed = Arc::new(AtomicBool::new(false));
    let closed_clone = closed.clone();
    win.on_window_event(move |ev| {
        if matches!(ev, tauri::WindowEvent::Destroyed) {
            closed_clone.store(true, Ordering::Relaxed);
        }
    });

    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
            if closed.load(Ordering::Relaxed) {
                return;
            }
            if win.eval(SCAN_JS).is_err() {
                return;
            }
        }
    });

    Ok(())
}

fn urlencoding_decode(s: &str) -> Result<String, ()> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).map_err(|_| ())?;
            let byte = u8::from_str_radix(hex, 16).map_err(|_| ())?;
            out.push(byte);
            i += 3;
        } else if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| ())
}
