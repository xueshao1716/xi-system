use serde::Serialize;
use std::io::Write;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ModelTier {
    Cheap,
    Balanced,
    Premium,
}

impl ModelTier {
    pub fn as_str(&self) -> &str {
        match self {
            ModelTier::Cheap => "cheap",
            ModelTier::Balanced => "balanced",
            ModelTier::Premium => "premium",
        }
    }

    pub fn model_name<'a>(&'a self, premium_config: &'a str) -> &'a str {
        match self {
            ModelTier::Cheap => "deepseek-v3",
            ModelTier::Balanced => "deepseek-v4-flash",
            ModelTier::Premium => premium_config,
        }
    }
}

impl std::fmt::Display for ModelTier {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Serialize)]
pub struct RouterDecision {
    pub ts: String,
    pub role_hint: String,
    pub tier: ModelTier,
    pub picked_model: String,
    pub picked_label: String,
    pub reason: String,
    pub prompt_chars: usize,
    pub duration_ms: u64,
    pub success: bool,
    pub reply_chars: usize,
    pub tried_fallback: bool,
    pub cost_estimate_usd: f64,
}

pub fn iso_now() -> String {
    use chrono::Utc;
    Utc::now().to_rfc3339()
}

pub fn classify_task(_role_hint: &str, prompt_chars: usize) -> ModelTier {
    if prompt_chars > 4000 {
        ModelTier::Premium
    } else if prompt_chars > 500 {
        ModelTier::Balanced
    } else {
        ModelTier::Cheap
    }
}

pub fn estimate_cost_usd(tier: ModelTier, prompt_chars: usize, reply_chars: usize) -> f64 {
    let (input_price, output_price) = match tier {
        ModelTier::Cheap => (0.001, 0.004),
        ModelTier::Balanced => (0.003, 0.012),
        ModelTier::Premium => (0.010, 0.025),
    };
    let prompt_tokens = prompt_chars as f64 / 4.0;
    let reply_tokens = reply_chars as f64 / 4.0;
    prompt_tokens * input_price + reply_tokens * output_price
}

pub fn log_decision(state_dir: &str, decision: &RouterDecision) {
    let log_path = Path::new(state_dir).join("router_log.jsonl");
    let line = serde_json::to_string(decision).unwrap_or_default();
    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map(|mut f| writeln!(f, "{}", line));
}