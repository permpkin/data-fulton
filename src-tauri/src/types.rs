use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SpeedProfile {
    Safe,
    Aggressive,
}

impl SpeedProfile {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Aggressive => "aggressive",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Pending,
    Running,
    Paused,
    Complete,
    Error,
    Cancelled,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Complete => "complete",
            Self::Error => "error",
            Self::Cancelled => "cancelled",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "running" => Self::Running,
            "paused" => Self::Paused,
            "complete" => Self::Complete,
            "error" => Self::Error,
            "cancelled" => Self::Cancelled,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionConfig {
    pub entities: Vec<String>,
    pub children: Vec<String>,
    pub download_files: bool,
    pub profile: SpeedProfile,
    pub dest_dir: String,
    #[serde(default)]
    pub name: String,
    /// Provider-supplied endpoint specs (new sessions). Empty for older
    /// sessions → the extractor falls back to the built-in catalog.
    #[serde(default)]
    pub entity_specs: Vec<EntitySpec>,
    #[serde(default)]
    pub child_specs: Vec<ChildSpec>,
}

/// Serializable entity endpoint spec carried inside a session config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntitySpec {
    pub id: String,
    pub label: String,
    pub path: String,
    /// "GET" (default) or "POST" — POST sends `body_template` as JSON.
    #[serde(default)]
    pub method: Option<String>,
    /// Request body template; `{page}`/`{per_page}` substituted per request.
    #[serde(default)]
    pub body_template: Option<String>,
    /// Dotted path to the rows array (default "data").
    #[serde(default)]
    pub data_path: Option<String>,
    pub group: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub downloadable: bool,
}

/// Serializable child (per-parent record) spec. `path`/`q_param` may contain
/// the `{id}` placeholder, substituted with the parent record id at run time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildSpec {
    pub id: String,
    pub label: String,
    pub parent: String,
    #[serde(default)]
    pub single: bool,
    pub path: String,
    #[serde(default)]
    pub q_param: Option<String>,
    /// Extra query params — values may contain the `{id}` placeholder.
    #[serde(default)]
    pub params: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionRow {
    pub id: String,
    pub created_at: String,
    pub provider: String,
    pub name: String,
    pub host: String,
    pub api_key: String,
    /// Provider-defined extra fields (e.g. client_id/client_secret for OAuth).
    #[serde(default)]
    pub credentials: std::collections::HashMap<String, String>,
    pub last_ok: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInput {
    pub id: Option<String>,
    pub provider: String,
    pub name: String,
    pub host: String,
    pub api_key: String,
    #[serde(default)]
    pub credentials: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRow {
    pub id: String,
    pub connection_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub status: SessionStatus,
    pub profile: SpeedProfile,
    pub dest_dir: String,
    pub export_dir: String,
    pub config: SessionConfig,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskProgress {
    pub id: i64,
    pub kind: String,
    pub entity: String,
    pub label: String,
    pub parent_ref: Option<String>,
    pub status: String,
    pub page: i64,
    pub fetched: i64,
    pub attempts: i64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileProgress {
    pub total: i64,
    pub done: i64,
    pub pending: i64,
    pub error: i64,
    pub skipped: i64,
    pub bytes_done: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEvent {
    pub id: i64,
    pub session_id: String,
    pub ts: String,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub session: SessionRow,
    pub tasks: Vec<TaskProgress>,
    pub files: FileProgress,
    pub recent_events: Vec<SessionEvent>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityInfo {
    pub id: String,
    pub label: String,
    pub group: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub downloadable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildInfo {
    pub id: String,
    pub label: String,
    pub parent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub entities: Vec<EntityInfo>,
    pub children: Vec<ChildInfo>,
    pub groups: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub dir: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateResult {
    pub ok: bool,
    pub message: String,
    /// The auth flow requires a second-factor code before it can proceed.
    #[serde(default)]
    pub needs_2fa: bool,
    /// Optional hint (e.g. where the code was sent).
    #[serde(default)]
    pub otp_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyFoundEvent {
    pub token: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub dest_dir: String,
    pub profile: SpeedProfile,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewSessionResult {
    pub session_id: String,
    pub export_dir: String,
}
