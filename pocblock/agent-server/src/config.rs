#[derive(Debug, Clone)]
pub struct Config {
    pub provider: String,
    pub model: String,
    pub agent_port: u16,
    pub sync_url: String,
    pub debounce_ms: u64,
    pub confidence_threshold: f64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            provider: std::env::var("AGENT_PROVIDER").unwrap_or("anthropic".into()),
            model: std::env::var("AGENT_MODEL")
                .unwrap_or("claude-sonnet-4-5-20250514".into()),
            agent_port: std::env::var("AGENT_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3001),
            sync_url: std::env::var("SYNC_URL")
                .unwrap_or("ws://localhost:4444".into()),
            debounce_ms: std::env::var("DEBOUNCE_MS")
                .ok()
                .and_then(|d| d.parse().ok())
                .unwrap_or(800),
            confidence_threshold: std::env::var("CONFIDENCE_THRESHOLD")
                .ok()
                .and_then(|c| c.parse().ok())
                .unwrap_or(0.6),
        }
    }
}
