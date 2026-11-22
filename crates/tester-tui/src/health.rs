use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnState {
    Unknown,
    Ok,
    Bad,
    Checking,
}

impl ConnState {
    pub fn glyph(self) -> &'static str {
        match self {
            ConnState::Unknown => "○",
            ConnState::Ok => "●",
            ConnState::Bad => "✗",
            ConnState::Checking => "...",
        }
    }
}

#[derive(Debug, Clone)]
pub struct HealthProbe {
    inner: Arc<Mutex<ProbeInner>>,
}

#[derive(Debug)]
struct ProbeInner {
    state: ConnState,
    last_check: Option<Instant>,
    base_url: String,
}

impl HealthProbe {
    pub fn new(base_url: &str) -> Self {
        let probe = Self {
            inner: Arc::new(Mutex::new(ProbeInner {
                state: ConnState::Unknown,
                last_check: None,
                base_url: base_url.trim_end_matches('/').to_string(),
            })),
        };
        probe.kick();
        probe
    }

    pub fn state(&self) -> ConnState {
        self.inner
            .lock()
            .map(|g| g.state)
            .unwrap_or(ConnState::Unknown)
    }

    pub fn tick(&self) {
        let should = self
            .inner
            .lock()
            .map(|g| {
                g.last_check
                    .map(|t| t.elapsed() > Duration::from_secs(30))
                    .unwrap_or(true)
                    && g.state != ConnState::Checking
            })
            .unwrap_or(false);
        if should {
            self.kick();
        }
    }

    fn kick(&self) {
        let inner = Arc::clone(&self.inner);
        if let Ok(mut g) = inner.lock() {
            g.state = ConnState::Checking;
        }
        thread::spawn(move || {
            let url = inner
                .lock()
                .map(|g| format!("{}/models", g.base_url))
                .unwrap_or_default();
            let ok = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(3))
                .build()
                .ok()
                .and_then(|c| c.get(&url).send().ok())
                .map(|r| r.status().is_success() || r.status().as_u16() == 401)
                .unwrap_or(false);
            if let Ok(mut g) = inner.lock() {
                g.state = if ok { ConnState::Ok } else { ConnState::Bad };
                g.last_check = Some(Instant::now());
            }
        });
    }
}
