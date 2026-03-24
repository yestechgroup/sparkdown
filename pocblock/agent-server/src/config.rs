#[derive(Debug, Clone)]
pub struct Config {
    pub llm_api_key: String,
    pub model: String,
    pub agent_port: u16,
    pub sync_url: String,
    pub debounce_ms: u64,
    pub confidence_threshold: f64,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        // Support both DEEPSEEK_API_KEY and ANTHROPIC_API_KEY for backwards compat
        let llm_api_key = std::env::var("DEEPSEEK_API_KEY")
            .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
            .map_err(|_| {
                anyhow::anyhow!(
                    "DEEPSEEK_API_KEY environment variable is required.\n\
                     Set it with: export DEEPSEEK_API_KEY=sk-..."
                )
            })?;

        Ok(Self {
            llm_api_key,
            model: std::env::var("AGENT_MODEL")
                .unwrap_or("deepseek-chat".into()),
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
        })
    }
}
