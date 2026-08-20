// evolution/gates.rs —— 进化门/棘轮/双速引擎（2026-08-21 从 evolution.rs 拆出）
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Dual-Speed Evolution (MOMO CODE pattern) ──────────────
// Fast ring: experience accumulation (seconds)
// Slow ring: capability upgrade (periodic)

/// Beta distribution for Thompson Sampling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetaDistribution {
    pub alpha: f64,  // wins + 1
    pub beta: f64,   // losses + 1
    pub name: String,
}

impl BetaDistribution {
    pub fn new(name: &str) -> Self {
        Self { alpha: 1.0, beta: 1.0, name: name.to_string() }
    }

    pub fn record_win(&mut self) { self.alpha += 1.0; }
    pub fn record_loss(&mut self) { self.beta += 1.0; }

    pub fn mean(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }

    /// Thompson sampling: draw from Beta distribution
    pub fn sample(&self) -> f64 {
        // Simple approximation: mean + variance-based noise
        let mean = self.mean();
        let variance = (self.alpha * self.beta) / ((self.alpha + self.beta).powi(2) * (self.alpha + self.beta + 1.0));
        let noise = (variance.sqrt() * ((self.alpha + self.beta) / 10.0).min(1.0));
        // Use a simple pseudo-random approach
        let r = (self.alpha * 7.3 + self.beta * 13.7) % 1.0;  // deterministic "random"
        mean + noise * (r - 0.5) * 2.0
    }
}

/// Ratchet Gate: only progress, never regress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatchetGate {
    pub baseline_score: f64,
    pub noise_tolerance: f64,    // default 0.02 (2%)
    pub regressions: usize,
    pub promotions: usize,
}

impl RatchetGate {
    pub fn new() -> Self {
        Self { baseline_score: 0.0, noise_tolerance: 0.02, regressions: 0, promotions: 0 }
    }

    /// Check if candidate passes the ratchet gate
    /// PASS iff: candidate >= baseline - noise_tolerance AND no regressions
    pub fn check(&mut self, candidate_score: f64, regression_count: usize) -> bool {
        self.regressions = regression_count;
        let passes_score = candidate_score >= (self.baseline_score - self.noise_tolerance);
        let passes_no_regression = regression_count == 0;
        let passed = passes_score && passes_no_regression;
        if passed {
            self.promotions += 1;
        }
        passed
    }

    pub fn promote(&mut self, new_score: f64) {
        self.baseline_score = new_score;
    }
}

/// Fast Ring: experience accumulation (per-conversation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastRing {
    pub tactics: Vec<BetaDistribution>,
    pub inject_count: usize,
    pub max_tactics: usize,
}

impl FastRing {
    pub fn new() -> Self {
        Self { tactics: Vec::new(), inject_count: 0, max_tactics: 20 }
    }

    /// Record an outcome for a tactic
    pub fn record(&mut self, tactic_name: &str, success: bool) {
        if let Some(t) = self.tactics.iter_mut().find(|t| t.name == tactic_name) {
            if success { t.record_win(); } else { t.record_loss(); }
        } else if self.tactics.len() < self.max_tactics {
            let mut t = BetaDistribution::new(tactic_name);
            if success { t.record_win(); } else { t.record_loss(); }
            self.tactics.push(t);
        }
    }

    /// Thompson sampling: select top-N tactics for injection
    pub fn select_tactics(&self, n: usize) -> Vec<String> {
        let mut sampled: Vec<(f64, String)> = self.tactics.iter()
            .map(|t| (t.sample(), t.name.clone()))
            .collect();
        sampled.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        sampled.into_iter().take(n).map(|(_, name)| name).collect()
    }

    pub fn summary(&self) -> String {
        format!("FastRing: {} tactics, {} injections", self.tactics.len(), self.inject_count)
    }
}

/// Slow Ring: periodic capability upgrade (training pipeline)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowRing {
    pub ratchet: RatchetGate,
    pub training_rounds: usize,
    pub last_training: String,
    pub curriculum: Vec<String>,  // gold + replay + hard-negative entries
}

impl SlowRing {
    pub fn new() -> Self {
        Self {
            ratchet: RatchetGate::new(),
            training_rounds: 0,
            last_training: String::new(),
            curriculum: Vec::new(),
        }
    }

    /// Add experience to curriculum for next training round
    pub fn add_to_curriculum(&mut self, entry: &str) {
        if self.curriculum.len() < 200 {
            self.curriculum.push(entry.to_string());
        }
    }

    /// Check if training should trigger (every N experiences)
    pub fn should_train(&self, threshold: usize) -> bool {
        self.curriculum.len() >= threshold
    }

    pub fn summary(&self) -> String {
        format!("SlowRing: {} rounds, {} curriculum entries, baseline={:.3}",
            self.training_rounds, self.curriculum.len(), self.ratchet.baseline_score)
    }
}

/// Dual-Speed Evolution Engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualSpeedEvolution {
    pub fast: FastRing,
    pub slow: SlowRing,
    pub total_experiences: usize,
}

impl DualSpeedEvolution {
    pub fn new() -> Self {
        Self {
            fast: FastRing::new(),
            slow: SlowRing::new(),
            total_experiences: 0,
        }
    }

    /// Process a conversation outcome (fast ring)
    pub fn process_outcome(&mut self, tactic: &str, success: bool) {
        self.fast.record(tactic, success);
        self.slow.add_to_curriculum(&format!("{}:{}", tactic, if success { "win" } else { "loss" }));
        self.total_experiences += 1;
    }

    /// Select tactics for injection into next prompt
    pub fn inject_strategies(&mut self) -> Vec<String> {
        let selected = self.fast.select_tactics(6);
        self.fast.inject_count += 1;
        selected
    }

    /// Check if slow ring training should trigger
    pub fn check_training(&self) -> bool {
        self.slow.should_train(10)  // every 10 experiences
    }

    pub fn describe(&self) -> String {
        format!("{}\n{}\nTotal experiences: {}",
            self.fast.summary(), self.slow.summary(), self.total_experiences)
    }
}


// ═══════════════════════════════════════════════════════════════════════════
