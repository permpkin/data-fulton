use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};

use crate::types::SpeedProfile;

#[derive(Debug, Clone, Copy)]
pub struct ProfileDef {
    /// requests per minute against the API
    pub rpm: u32,
    /// concurrent API requests
    pub concurrency: usize,
    /// concurrent file downloads
    pub file_concurrency: usize,
    /// records requested per page
    pub per_page: u32,
}

pub fn profile(p: SpeedProfile) -> ProfileDef {
    match p {
        SpeedProfile::Safe => ProfileDef {
            rpm: 25,
            concurrency: 1,
            file_concurrency: 1,
            per_page: 50,
        },
        SpeedProfile::Aggressive => ProfileDef {
            rpm: 150,
            concurrency: 4,
            file_concurrency: 3,
            per_page: 100,
        },
    }
}

struct GateState {
    next_slot_at: Instant,
    cooldown_until: Instant,
    interval: Duration,
}

/**
 * Spaced-request gate: paces requests to `rpm`, caps in-flight requests at
 * `concurrency`, and applies an adaptive cooldown when the API pushes back
 * (HTTP 429 / Retry-After). All API calls funnel through `run()`.
 */
pub struct RateGate {
    sem: Arc<Semaphore>,
    state: Mutex<GateState>,
    base_interval: Duration,
}

impl RateGate {
    pub fn new(p: ProfileDef) -> Self {
        Self {
            sem: Arc::new(Semaphore::new(p.concurrency)),
            base_interval: Duration::from_secs_f64(60.0 / p.rpm as f64),
            state: Mutex::new(GateState {
                next_slot_at: Instant::now(),
                cooldown_until: Instant::now(),
                interval: Duration::from_secs_f64(60.0 / p.rpm as f64),
            }),
        }
    }

    /// Wait for a paced slot + concurrency permit, then invoke `fut`.
    pub async fn run<F, T, Fut>(&self, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let _permit = self.sem.acquire().await.expect("semaphore closed");
        let wait = {
            let mut s = self.state.lock().await;
            let now = Instant::now();
            let start = s.next_slot_at.max(s.cooldown_until).max(now);
            s.next_slot_at = start + s.interval;
            start.saturating_duration_since(now)
        };
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
        f().await
    }

    /// Called when the API throttles us: cool everything off and double spacing.
    pub async fn report_throttle(&self, retry_after: Option<Duration>) {
        let mut s = self.state.lock().await;
        let penalty = retry_after.unwrap_or(Duration::from_secs(15));
        let until = Instant::now() + penalty;
        if until > s.cooldown_until {
            s.cooldown_until = until;
        }
        s.interval = (s.interval * 2).min(self.base_interval * 4);
    }

    /// Relax spacing back toward baseline on success.
    pub async fn report_success(&self) {
        let mut s = self.state.lock().await;
        if s.interval > self.base_interval {
            s.interval = Duration::from_secs_f64((s.interval.as_secs_f64() / 1.5).max(self.base_interval.as_secs_f64()));
        }
    }
}

/// Simple concurrency cap for file downloads.
pub struct ConcurrencyGate {
    sem: Arc<Semaphore>,
}

impl ConcurrencyGate {
    pub fn new(max: usize) -> Self {
        Self {
            sem: Arc::new(Semaphore::new(max)),
        }
    }

    pub async fn run<F, T, Fut>(&self, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let _permit = self.sem.acquire().await.expect("semaphore closed");
        f().await
    }
}

pub fn parse_retry_after(value: Option<&str>) -> Option<Duration> {
    let v = value?.trim();
    if let Ok(secs) = v.parse::<f64>() {
        return Some(Duration::from_secs_f64(secs.max(0.0)));
    }
    if let Ok(t) = chrono::DateTime::parse_from_rfc2822(v) {
        let ms = t.timestamp_millis() - chrono::Utc::now().timestamp_millis();
        return Some(Duration::from_millis(ms.max(0) as u64));
    }
    None
}

/// Exponential backoff with jitter.
pub fn backoff(attempt: u32) -> Duration {
    let base = (1000.0 * 2f64.powi(attempt as i32)).min(30_000.0);
    let jitter = 0.5 + rand_fraction() * 0.5;
    Duration::from_millis((base * jitter) as u64)
}

fn rand_fraction() -> f64 {
    // cheap jitter without pulling in rand: nanos-based fraction
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (n % 1000) as f64 / 1000.0
}
