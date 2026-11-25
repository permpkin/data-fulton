use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

use crate::limiter::{parse_retry_after, RateGate};

#[derive(Debug)]
pub struct ApiError {
    pub message: String,
    pub status: u16,
    pub retry_after: Option<Duration>,
    /// Whether the request can safely be retried (429, 5xx, network errors).
    pub retryable: bool,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for ApiError {}

pub type TokenProvider = Arc<dyn Fn() -> Option<String> + Send + Sync>;

/**
 * Read-only client for the Prime Ecosystem API.
 *
 * There is intentionally NO post/put/patch/delete here — this application
 * only ever reads the user's own data. Every request is a GET.
 */
#[derive(Clone)]
pub struct PrimeClient {
    base_url: String,
    http: reqwest::Client,
    get_token: TokenProvider,
    gate: Arc<RateGate>,
}

impl PrimeClient {
    pub fn new(base_url: &str, get_token: TokenProvider, gate: Arc<RateGate>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent("datafulton-exporter/0.1")
            .build()
            .expect("reqwest client");
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http,
            get_token,
            gate,
        }
    }

    /// Same client bound to a different gate (per-session rate profiles).
    pub fn using(&self, gate: Arc<RateGate>) -> Self {
        Self {
            base_url: self.base_url.clone(),
            http: self.http.clone(),
            get_token: self.get_token.clone(),
            gate,
        }
    }

    pub fn origin(&self) -> String {
        reqwest::Url::parse(&self.base_url)
            .map(|u| u.origin().ascii_serialization())
            .unwrap_or_default()
    }

    /// One GET request → parsed JSON. The only HTTP verb this app ever sends.
    pub async fn get_json(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<Value, ApiError> {
        let token = (self.get_token)().ok_or(ApiError {
            message: "No API key configured".into(),
            status: 401,
            retry_after: None,
            retryable: false,
        })?;

        let url = format!("{}{}", self.base_url, path);
        let gate = self.gate.clone();
        let http = self.http.clone();

        let res = gate
            .run(|| async {
                http.get(&url)
                    .bearer_auth(&token)
                    .header(reqwest::header::ACCEPT, "application/json")
                    .query(params)
                    .send()
                    .await
            })
            .await;

        let res = match res {
            Ok(r) => r,
            Err(e) => {
                return Err(ApiError {
                    message: format!("Network error: {e}"),
                    status: 0,
                    retry_after: None,
                    retryable: true,
                })
            }
        };

        let status = res.status().as_u16();
        match status {
            429 => {
                let ra = parse_retry_after(
                    res.headers()
                        .get(reqwest::header::RETRY_AFTER)
                        .and_then(|v| v.to_str().ok()),
                );
                self.gate.report_throttle(ra).await;
                return Err(ApiError {
                    message: "Rate limited (429)".into(),
                    status,
                    retry_after: ra,
                    retryable: true,
                });
            }
            401 | 403 => {
                return Err(ApiError {
                    message: format!("Authentication failed ({status}). Check your API key."),
                    status,
                    retry_after: None,
                    retryable: false,
                })
            }
            _ if status >= 500 => {
                return Err(ApiError {
                    message: format!("Server error {status}"),
                    status,
                    retry_after: None,
                    retryable: true,
                })
            }
            _ if !res.status().is_success() => {
                return Err(ApiError {
                    message: format!("Request failed: {status}"),
                    status,
                    retry_after: None,
                    retryable: false,
                })
            }
            _ => {}
        }

        self.gate.report_success().await;
        res.json::<Value>().await.map_err(|e| ApiError {
            message: format!("Response was not valid JSON: {e}"),
            status,
            retry_after: None,
            retryable: false,
        })
    }

    /// POST request → parsed JSON (GraphQL providers like monday.com).
    pub async fn post_json(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<Value, ApiError> {
        let token = (self.get_token)().ok_or(ApiError {
            message: "No API key configured".into(),
            status: 401,
            retry_after: None,
            retryable: false,
        })?;

        let url = format!("{}{}", self.base_url, path);
        let gate = self.gate.clone();
        let http = self.http.clone();

        let res = gate
            .run(|| async {
                http.post(&url)
                    .bearer_auth(&token)
                    .header(reqwest::header::ACCEPT, "application/json")
                    .json(&body)
                    .send()
                    .await
            })
            .await;

        let res = match res {
            Ok(r) => r,
            Err(e) => {
                return Err(ApiError {
                    message: format!("Network error: {e}"),
                    status: 0,
                    retry_after: None,
                    retryable: true,
                })
            }
        };

        let status = res.status().as_u16();
        match status {
            429 => {
                let ra = parse_retry_after(
                    res.headers()
                        .get(reqwest::header::RETRY_AFTER)
                        .and_then(|v| v.to_str().ok()),
                );
                self.gate.report_throttle(ra).await;
                return Err(ApiError {
                    message: "Rate limited (429)".into(),
                    status,
                    retry_after: ra,
                    retryable: true,
                });
            }
            401 | 403 => {
                return Err(ApiError {
                    message: format!("Authentication failed ({status}). Check your API key."),
                    status,
                    retry_after: None,
                    retryable: false,
                })
            }
            _ if status >= 500 => {
                return Err(ApiError {
                    message: format!("Server error {status}"),
                    status,
                    retry_after: None,
                    retryable: true,
                })
            }
            _ if !res.status().is_success() => {
                return Err(ApiError {
                    message: format!("Request failed: {status}"),
                    status,
                    retry_after: None,
                    retryable: false,
                })
            }
            _ => {}
        }

        self.gate.report_success().await;
        res.json::<Value>().await.map_err(|e| ApiError {
            message: format!("Response was not valid JSON: {e}"),
            status,
            retry_after: None,
            retryable: false,
        })
    }
}
