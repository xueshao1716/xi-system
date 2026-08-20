// evolution/signals.rs —— 信号与基因表达（2026-08-21 从 evolution.rs 拆出）
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signals {
    pub intimacy: f64,
    pub novelty: f64,
    pub stress: f64,
    pub trust: f64,
}

impl Signals {
    pub fn default() -> Self {
        Self { intimacy: 0.5, novelty: 0.5, stress: 0.15, trust: 0.5 }
    }

    fn decay(&mut self) {
        // [Fix] Level-proportional decay: high signals decay faster → equilibrium instead of saturation
        // At 1.0: effective decay per tick ≈ 0.05 (intimacy), 0.30 (novelty), 0.10 (stress), 0.03 (trust)
        // At 0.5: effective decay per tick ≈ 0.025, 0.15, 0.05, 0.015
        // This prevents saturation while preserving low-signal sensitivity
        let intimacy_decay = 0.05 + self.intimacy * 0.10;   // stronger decay at high levels
        let novelty_decay = 0.30 + self.novelty * 0.10;
        let stress_decay = 0.10 + self.stress * 0.10;
        let trust_decay = 0.03 + self.trust * 0.15;        // trust decays faster when high (earned not locked)
        self.intimacy = (self.intimacy * (1.0 - intimacy_decay)).max(0.0);
        self.novelty = (self.novelty * (1.0 - novelty_decay)).max(0.0);
        self.stress = (self.stress * (1.0 - stress_decay)).max(0.0);
        self.trust = (self.trust * (1.0 - trust_decay)).max(0.0);
    }
}

// ─── Gene Expression ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneExpression {
    pub gentleness: f64,
    pub initiative: f64,
    pub curiosity: f64,
    pub attachment: f64,
    pub learning: f64,
    pub creativity: f64,
    pub caution: f64,
    pub humor: f64,
    pub loyalty: f64,
    pub autonomy_bias: f64,
}

impl GeneExpression {
    pub fn default() -> Self {
        Self {
            gentleness: 0.5, initiative: 0.5, curiosity: 0.5,
            attachment: 0.5, learning: 0.5, creativity: 0.5,
            caution: 0.5, humor: 0.5, loyalty: 0.5, autonomy_bias: 0.5,
        }
    }

    pub(crate) fn express(signals: &Signals) -> Self {
        let s = |v: f64| v.max(0.0).min(1.0);
        let b = 0.5;
        Self {
            gentleness:    s(b + signals.intimacy * 0.15 + signals.trust * 0.10 - signals.stress * 0.10),
            initiative:    s(b + signals.novelty * 0.12 - signals.stress * 0.18),
            curiosity:     s(b + signals.novelty * 0.15),
            attachment:    s(b + signals.intimacy * 0.12 + signals.trust * 0.10),
            learning:      s(b + signals.novelty * 0.10 + (1.0 - signals.stress) * 0.08),
            creativity:    s(b + signals.novelty * 0.15),
            caution:       s(b + signals.stress * 0.20),
            humor:         s(b + signals.intimacy * 0.08 - signals.stress * 0.08),
            loyalty:       b,
            autonomy_bias: s(b + signals.novelty * 0.10 - signals.stress * 0.12),
        }
    }

    pub(crate) fn to_map(&self) -> HashMap<String, f64> {
        let mut m = HashMap::new();
        m.insert("gentleness".into(), self.gentleness);
        m.insert("initiative".into(), self.initiative);
        m.insert("curiosity".into(), self.curiosity);
        m.insert("attachment".into(), self.attachment);
        m.insert("learning".into(), self.learning);
        m.insert("creativity".into(), self.creativity);
        m.insert("caution".into(), self.caution);
        m.insert("humor".into(), self.humor);
        m.insert("loyalty".into(), self.loyalty);
        m.insert("autonomy_bias".into(), self.autonomy_bias);
        m
    }
}
