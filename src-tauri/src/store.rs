use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Mutex;

use crate::types::*;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS kv (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS connections (
  id TEXT PRIMARY KEY,
  created_at TEXT NOT NULL,
  provider TEXT NOT NULL DEFAULT 'primeeco',
  credentials TEXT NOT NULL DEFAULT '{}',
  name TEXT NOT NULL,
  host TEXT NOT NULL,
  api_key TEXT NOT NULL,
  last_ok INTEGER
);
CREATE TABLE IF NOT EXISTS sessions (
  connection_id TEXT NOT NULL DEFAULT '',
  id TEXT PRIMARY KEY,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  status TEXT NOT NULL,
  profile TEXT NOT NULL,
  dest_dir TEXT NOT NULL,
  export_dir TEXT NOT NULL,
  config_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS tasks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL,
  kind TEXT NOT NULL,
  entity TEXT NOT NULL,
  label TEXT NOT NULL,
  parent_ref TEXT,
  path TEXT NOT NULL,
  params_json TEXT,
  page INTEGER NOT NULL DEFAULT 1,
  fetched INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'pending',
  attempts INTEGER NOT NULL DEFAULT 0,
  last_error TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_tasks_session ON tasks(session_id, status);
CREATE TABLE IF NOT EXISTS files (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL,
  source_entity TEXT,
  record_ref TEXT,
  url TEXT NOT NULL,
  dest_path TEXT NOT NULL,
  filename TEXT,
  status TEXT NOT NULL DEFAULT 'pending',
  bytes_total INTEGER,
  bytes_done INTEGER NOT NULL DEFAULT 0,
  attempts INTEGER NOT NULL DEFAULT 0,
  last_error TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(session_id, url, dest_path)
);
CREATE INDEX IF NOT EXISTS idx_files_session ON files(session_id, status);
CREATE TABLE IF NOT EXISTS events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL,
  ts TEXT NOT NULL,
  level TEXT NOT NULL,
  message TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id, id);
";

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[derive(Debug, Clone)]
pub struct TaskRow {
    pub id: i64,
    pub session_id: String,
    pub kind: String,
    pub entity: String,
    pub label: String,
    pub parent_ref: Option<String>,
    pub path: String,
    pub params_json: Option<String>,
    pub page: i64,
    pub fetched: i64,
    pub attempts: i64,
}

#[derive(Debug, Clone)]
pub struct FileRow {
    pub id: i64,
    pub session_id: String,
    pub url: String,
    pub dest_path: String,
    pub filename: Option<String>,
    pub bytes_done: i64,
    pub attempts: i64,
}

pub struct Store {
    conn: Mutex<Connection>,
}


fn creds_json(m: &std::collections::HashMap<String, String>) -> String {
    serde_json::to_string(m).unwrap_or_else(|_| "{}".into())
}

fn creds_map(s: String) -> std::collections::HashMap<String, String> {
    serde_json::from_str(&s).unwrap_or_default()
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path).context("open sqlite")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.execute_batch(SCHEMA)?;
        // Migration for pre-connections DBs: add the column if missing.
        let has_conn_col = conn
            .prepare("PRAGMA table_info(sessions)")?
            .query_map([], |r| r.get::<_, String>(1))?
            .any(|n| n.map(|n| n == "connection_id").unwrap_or(false));
        if !has_conn_col {
            conn.execute(
                "ALTER TABLE sessions ADD COLUMN connection_id TEXT NOT NULL DEFAULT ''",
                [],
            )?;
        }
        // Migration: add connections.provider for DBs created before providers.
        let has_provider = conn
            .prepare("PRAGMA table_info(connections)")?
            .query_map([], |r| r.get::<_, String>(1))?
            .any(|n| n.map(|n| n == "provider").unwrap_or(false));
        let has_creds = conn
            .prepare("PRAGMA table_info(connections)")?
            .query_map([], |r| r.get::<_, String>(1))?
            .any(|n| n.map(|n| n == "credentials").unwrap_or(false));
        if !has_creds {
            conn.execute("ALTER TABLE connections ADD COLUMN credentials TEXT NOT NULL DEFAULT '{}'", [])?;
        }
        if !has_provider {
            conn.execute(
                "ALTER TABLE connections ADD COLUMN provider TEXT NOT NULL DEFAULT 'primeeco'",
                [],
            )?;
        }
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// In-memory store for tests.
    pub fn memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    // ---- kv ----
    pub fn kv_get(&self, key: &str) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT value FROM kv WHERE key = ?1", params![key], |r| {
            r.get(0)
        })
        .optional()
        .ok()
        .flatten()
    }

    pub fn kv_set(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO kv (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn kv_delete(&self, key: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM kv WHERE key = ?1", params![key])?;
        Ok(())
    }

    // ---- connections ----

    pub fn create_connection(&self, c: &ConnectionRow) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO connections (id, created_at, provider, credentials, name, host, api_key, last_ok)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![c.id, c.created_at, c.provider, creds_json(&c.credentials), c.name, c.host, c.api_key, c.last_ok],
        )?;
        Ok(())
    }

    pub fn update_connection(&self, c: &ConnectionRow) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE connections SET provider = ?2, credentials = ?3, name = ?4, host = ?5, api_key = ?6, last_ok = ?7 WHERE id = ?1",
            params![c.id, c.provider, creds_json(&c.credentials), c.name, c.host, c.api_key, c.last_ok],
        )?;
        Ok(())
    }

    pub fn get_connection(&self, id: &str) -> Result<Option<ConnectionRow>> {
        let conn = self.conn.lock().unwrap();
        Ok(conn
            .query_row(
                "SELECT id, created_at, provider, credentials, name, host, api_key, last_ok FROM connections WHERE id = ?1",
                params![id],
                |r| {
                    Ok(ConnectionRow {
                        id: r.get(0)?,
                        created_at: r.get(1)?,
                        provider: r.get(2)?,
                        credentials: creds_map(r.get(3)?),
                        name: r.get(4)?,
                        host: r.get(5)?,
                        api_key: r.get(6)?,
                        last_ok: r.get(7)?,
                    })
                },
            )
            .optional()?)
    }

    pub fn list_connections(&self) -> Result<Vec<ConnectionRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, created_at, provider, credentials, name, host, api_key, last_ok FROM connections ORDER BY created_at",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(ConnectionRow {
                id: r.get(0)?,
                created_at: r.get(1)?,
                provider: r.get(2)?,
                credentials: creds_map(r.get(3)?),
                name: r.get(4)?,
                host: r.get(5)?,
                api_key: r.get(6)?,
                last_ok: r.get(7)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Delete finished (complete/error/cancelled) sessions for a connection.
    /// Exported files on disk are left untouched.
    pub fn clear_finished_sessions(&self, connection_id: &str) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let ids: Vec<String> = conn
            .prepare(
                "SELECT id FROM sessions WHERE connection_id = ?1
                 AND status IN ('complete', 'error', 'cancelled')",
            )?
            .query_map(params![connection_id], |r| r.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        let n = ids.len() as u64;
        for id in ids {
            for q in [
                "DELETE FROM tasks WHERE session_id = ?1",
                "DELETE FROM files WHERE session_id = ?1",
                "DELETE FROM events WHERE session_id = ?1",
                "DELETE FROM sessions WHERE id = ?1",
            ] {
                conn.execute(q, params![id])?;
            }
        }
        Ok(n)
    }

    /// Delete a connection and all of its sessions' bookkeeping rows
    /// (exported files on disk are left alone).
    pub fn delete_connection(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM connections WHERE id = ?1", params![id])?;
        for table in ["tasks", "files", "events"] {
            conn.execute(
                &format!(
                    "DELETE FROM {table} WHERE session_id IN (SELECT id FROM sessions WHERE connection_id = ?1)"
                ),
                params![id],
            )?;
        }
        conn.execute("DELETE FROM sessions WHERE connection_id = ?1", params![id])?;
        Ok(())
    }

    /// Delete one session and its bookkeeping rows (exported files stay on disk).
    pub fn delete_session(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        for table in ["tasks", "files", "events"] {
            conn.execute(&format!("DELETE FROM {table} WHERE session_id = ?1"), params![id])?;
        }
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ---- sessions ----
    pub fn create_session(
        &self,
        id: &str,
        connection_id: &str,
        config: &SessionConfig,
        export_dir: &str,
    ) -> Result<()> {
        let ts = now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (connection_id, id, created_at, updated_at, status, profile, dest_dir, export_dir, config_json)
             VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, ?7, ?8)",
            params![
                connection_id,
                id,
                ts,
                ts,
                config.profile.as_str(),
                config.dest_dir,
                export_dir,
                serde_json::to_string(config)?
            ],
        )?;
        Ok(())
    }

    fn row_to_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<SessionRow> {
        let status: String = row.get(4)?;
        let profile: String = row.get(5)?;
        let cfg: String = row.get(8)?;
        Ok(SessionRow {
            id: row.get(0)?,
            connection_id: row.get(1)?,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
            status: SessionStatus::from_str(&status),
            profile: if profile == "aggressive" {
                SpeedProfile::Aggressive
            } else {
                SpeedProfile::Safe
            },
            dest_dir: row.get(6)?,
            export_dir: row.get(7)?,
            config: serde_json::from_str(&cfg).unwrap_or(SessionConfig {
                entities: vec![],
                children: vec![],
                download_files: false,
                profile: SpeedProfile::Safe,
                dest_dir: String::new(),
                name: String::new(),
                entity_specs: vec![],
                child_specs: vec![],
            }),
        })
    }

    const SESSION_COLS: &'static str =
        "id, connection_id, created_at, updated_at, status, profile, dest_dir, export_dir, config_json";

    pub fn get_session(&self, id: &str) -> Result<Option<SessionRow>> {
        let conn = self.conn.lock().unwrap();
        Ok(conn
            .query_row(
                &format!("SELECT {} FROM sessions WHERE id = ?1", Self::SESSION_COLS),
                params![id],
                Self::row_to_session,
            )
            .optional()?)
    }

    pub fn sessions_for_connection(&self, connection_id: &str) -> Result<Vec<SessionRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&format!(
            "SELECT {} FROM sessions WHERE connection_id = ?1 ORDER BY created_at DESC",
            Self::SESSION_COLS
        ))?;
        let rows = stmt.query_map(params![connection_id], Self::row_to_session)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Update profile + downloadFiles for a resumable session.
    pub fn update_session_config(
        &self,
        id: &str,
        profile: SpeedProfile,
        download_files: bool,
    ) -> Result<()> {
        let mut s = self
            .get_session(id)?
            .ok_or_else(|| anyhow::anyhow!("unknown session"))?;
        s.config.profile = profile;
        s.config.download_files = download_files;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET profile = ?1, config_json = ?2, updated_at = ?3 WHERE id = ?4",
            params![profile.as_str(), serde_json::to_string(&s.config)?, now(), id],
        )?;
        Ok(())
    }

    /// Replace a session's whole config (pending-session editing).
    pub fn save_session_config(&self, id: &str, config: &SessionConfig) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET profile = ?1, dest_dir = ?2, config_json = ?3, updated_at = ?4 WHERE id = ?5",
            params![config.profile.as_str(), config.dest_dir, serde_json::to_string(config)?, now(), id],
        )?;
        Ok(())
    }

    pub fn session_status(&self, id: &str) -> SessionStatus {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT status FROM sessions WHERE id = ?1",
            params![id],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()
        .map(|s| SessionStatus::from_str(&s))
        .unwrap_or(SessionStatus::Cancelled)
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&format!(
            "SELECT {} FROM sessions ORDER BY created_at DESC",
            Self::SESSION_COLS
        ))?;
        let rows = stmt.query_map([], Self::row_to_session)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn set_session_status(&self, id: &str, status: SessionStatus) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status.as_str(), now(), id],
        )?;
        Ok(())
    }

    /// On startup/quit: anything mid-flight becomes resumable.
    pub fn sweep_interrupted(&self) -> Result<()> {
        let ts = now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET status = 'paused', updated_at = ?1 WHERE status = 'running'",
            params![ts],
        )?;
        conn.execute(
            "UPDATE tasks SET status = 'pending', updated_at = ?1 WHERE status = 'running'",
            params![ts],
        )?;
        conn.execute(
            "UPDATE files SET status = 'pending', updated_at = ?1 WHERE status = 'downloading'",
            params![ts],
        )?;
        Ok(())
    }

    /// Restart-on-error: failed task/file rows become runnable again.
    pub fn reset_error_tasks(&self, session_id: &str) -> Result<()> {
        let ts = now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tasks SET status = 'pending', last_error = NULL, updated_at = ?1 WHERE session_id = ?2 AND status IN ('error', 'running')",
            params![ts, session_id],
        )?;
        conn.execute(
            "UPDATE files SET status = 'pending', updated_at = ?1 WHERE session_id = ?2 AND status IN ('error', 'downloading')",
            params![ts, session_id],
        )?;
        Ok(())
    }

    /// Full restart: every task/file goes back to pending with counters cleared.
    pub fn reset_session_tasks(&self, session_id: &str) -> Result<()> {
        let ts = now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tasks SET status = 'pending', page = 0, fetched = 0, attempts = 0, last_error = NULL, updated_at = ?1 WHERE session_id = ?2",
            params![ts, session_id],
        )?;
        conn.execute(
            "UPDATE files SET status = 'pending', updated_at = ?1 WHERE session_id = ?2",
            params![ts, session_id],
        )?;
        Ok(())
    }

    // ---- tasks ----
    #[allow(clippy::too_many_arguments)]
    pub fn add_task(
        &self,
        session_id: &str,
        kind: &str,
        entity: &str,
        label: &str,
        path: &str,
        parent_ref: Option<&str>,
        params_json: Option<&str>,
    ) -> Result<()> {
        let ts = now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO tasks (session_id, kind, entity, label, parent_ref, path, params_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![session_id, kind, entity, label, parent_ref, path, params_json, ts, ts],
        )?;
        Ok(())
    }

    /// Atomically claim the next pending task (entity tasks first).
    pub fn claim_task(&self, session_id: &str) -> Result<Option<TaskRow>> {
        let conn = self.conn.lock().unwrap();
        let row = conn
            .query_row(
                "UPDATE tasks SET status = 'running', updated_at = ?2
                 WHERE id = (
                   SELECT id FROM tasks WHERE session_id = ?1 AND status = 'pending'
                   ORDER BY CASE kind WHEN 'entity' THEN 0 ELSE 1 END, id LIMIT 1
                 )
                 RETURNING id, session_id, kind, entity, label, parent_ref, path, params_json, page, fetched, attempts",
                params![session_id, now()],
                |r| {
                    Ok(TaskRow {
                        id: r.get(0)?,
                        session_id: r.get(1)?,
                        kind: r.get(2)?,
                        entity: r.get(3)?,
                        label: r.get(4)?,
                        parent_ref: r.get(5)?,
                        path: r.get(6)?,
                        params_json: r.get(7)?,
                        page: r.get(8)?,
                        fetched: r.get(9)?,
                        attempts: r.get(10)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    pub fn update_task_progress(&self, id: i64, page: i64, fetched: i64, attempts: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tasks SET page = ?1, fetched = ?2, attempts = ?3, updated_at = ?4 WHERE id = ?5",
            params![page, fetched, attempts, now(), id],
        )?;
        Ok(())
    }

    pub fn finish_task(&self, id: i64, status: &str, error: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tasks SET status = ?1, last_error = ?2, updated_at = ?3 WHERE id = ?4",
            params![status, error, now(), id],
        )?;
        Ok(())
    }

    /// Pause/stop: put a claimed task back without losing its page checkpoint.
    pub fn release_task(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tasks SET status = 'pending', updated_at = ?1 WHERE id = ?2 AND status = 'running'",
            params![now(), id],
        )?;
        Ok(())
    }

    pub fn tasks_for(&self, session_id: &str) -> Result<Vec<TaskProgress>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, kind, entity, label, parent_ref, status, page, fetched, attempts, last_error
             FROM tasks WHERE session_id = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map(params![session_id], |r| {
            Ok(TaskProgress {
                id: r.get(0)?,
                kind: r.get(1)?,
                entity: r.get(2)?,
                label: r.get(3)?,
                parent_ref: r.get(4)?,
                status: r.get(5)?,
                page: r.get(6)?,
                fetched: r.get(7)?,
                attempts: r.get(8)?,
                last_error: r.get(9)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn remaining_tasks(&self, session_id: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let c: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE session_id = ?1 AND status IN ('pending','running')",
            params![session_id],
            |r| r.get(0),
        )?;
        Ok(c)
    }

    // ---- files ----
    pub fn add_file(
        &self,
        session_id: &str,
        source_entity: &str,
        record_ref: Option<&str>,
        url: &str,
        dest_path: &str,
        filename: Option<&str>,
    ) -> Result<()> {
        let ts = now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO files (session_id, source_entity, record_ref, url, dest_path, filename, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(session_id, url, dest_path) DO NOTHING",
            params![session_id, source_entity, record_ref, url, dest_path, filename, ts, ts],
        )?;
        Ok(())
    }

    pub fn claim_file(&self, session_id: &str) -> Result<Option<FileRow>> {
        let conn = self.conn.lock().unwrap();
        let row = conn
            .query_row(
                "UPDATE files SET status = 'downloading', updated_at = ?2
                 WHERE id = (
                   SELECT id FROM files WHERE session_id = ?1 AND status = 'pending' ORDER BY id LIMIT 1
                 )
                 RETURNING id, session_id, url, dest_path, filename, bytes_done, attempts",
                params![session_id, now()],
                |r| {
                    Ok(FileRow {
                        id: r.get(0)?,
                        session_id: r.get(1)?,
                        url: r.get(2)?,
                        dest_path: r.get(3)?,
                        filename: r.get(4)?,
                        bytes_done: r.get(5)?,
                        attempts: r.get(6)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    pub fn update_file(
        &self,
        id: i64,
        status: Option<&str>,
        bytes_done: Option<i64>,
        bytes_total: Option<i64>,
        attempts: Option<i64>,
        last_error: Option<&str>,
    ) -> Result<()> {
        let mut sets = vec!["updated_at = ?1".to_string()];
        let mut vals: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(now())];
        let mut idx = 2;
        if let Some(v) = status {
            sets.push(format!("status = ?{idx}"));
            vals.push(Box::new(v.to_string()));
            idx += 1;
        }
        if let Some(v) = bytes_done {
            sets.push(format!("bytes_done = ?{idx}"));
            vals.push(Box::new(v));
            idx += 1;
        }
        if let Some(v) = bytes_total {
            sets.push(format!("bytes_total = ?{idx}"));
            vals.push(Box::new(v));
            idx += 1;
        }
        if let Some(v) = attempts {
            sets.push(format!("attempts = ?{idx}"));
            vals.push(Box::new(v));
            idx += 1;
        }
        if last_error.is_some() {
            sets.push(format!("last_error = ?{idx}"));
            vals.push(Box::new(last_error.unwrap().to_string()));
            idx += 1;
        }
        let _ = idx;
        vals.push(Box::new(id));
        let sql = format!("UPDATE files SET {} WHERE id = ?{}", sets.join(", "), vals.len());
        let conn = self.conn.lock().unwrap();
        let refs: Vec<&dyn rusqlite::ToSql> = vals.iter().map(|b| b.as_ref()).collect();
        conn.execute(&sql, refs.as_slice())?;
        Ok(())
    }

    pub fn remaining_files(&self, session_id: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let c: i64 = conn.query_row(
            "SELECT COUNT(*) FROM files WHERE session_id = ?1 AND status IN ('pending','downloading')",
            params![session_id],
            |r| r.get(0),
        )?;
        Ok(c)
    }

    pub fn file_stats(&self, session_id: &str) -> Result<FileProgress> {
        let conn = self.conn.lock().unwrap();
        let r = conn.query_row(
            "SELECT COUNT(*),
                    SUM(CASE WHEN status = 'done' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN status IN ('pending','downloading') THEN 1 ELSE 0 END),
                    SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN status = 'skipped' THEN 1 ELSE 0 END),
                    COALESCE(SUM(bytes_done), 0)
             FROM files WHERE session_id = ?1",
            params![session_id],
            |r| {
                Ok(FileProgress {
                    total: r.get(0)?,
                    done: r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                    pending: r.get::<_, Option<i64>>(2)?.unwrap_or(0),
                    error: r.get::<_, Option<i64>>(3)?.unwrap_or(0),
                    skipped: r.get::<_, Option<i64>>(4)?.unwrap_or(0),
                    bytes_done: r.get(5)?,
                })
            },
        )?;
        Ok(r)
    }

    // ---- events ----
    pub fn log_event(&self, session_id: &str, level: &str, message: &str) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute(
            "INSERT INTO events (session_id, ts, level, message) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, now(), level, message],
        );
    }

    pub fn recent_events(&self, session_id: &str, limit: i64) -> Result<Vec<SessionEvent>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, ts, level, message FROM events
             WHERE session_id = ?1 ORDER BY id DESC LIMIT ?2",
        )?;
        let mut rows: Vec<SessionEvent> = stmt
            .query_map(params![session_id, limit], |r| {
                Ok(SessionEvent {
                    id: r.get(0)?,
                    session_id: r.get(1)?,
                    ts: r.get(2)?,
                    level: r.get(3)?,
                    message: r.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        rows.reverse();
        Ok(rows)
    }
}

