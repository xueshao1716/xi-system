/// Throat — Intent encoding for LLM context
///
/// Encodes emotional/cognitive/behavioral state into an intent vector

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const INTENT_DIMS: &[&str] = &[
    "emotional_direction",
    "cognitive_focus",
    "behavioral_drive",
    "relational_tone",
    "memory_echo",
    "creative_pressure",
    "domain_intent",
    "urgency_signal",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentVector {
    pub time: String,
    pub dimensions: HashMap<String, f64>,
    pub correlations: Vec<String>,
    pub signal_strength: f64,
    pub encoding_depth: String,
}

#[derive(Debug, Clone)]
pub struct Throat {
    pub intent: IntentVector,
    pub last_dim_count: usize,
}

impl Throat {
    pub fn new() -> Self {
        let mut dims = HashMap::new();
        for d in INTENT_DIMS {
            dims.insert(d.to_string(), 0.0);
        }
        Self {
            intent: IntentVector {
                time: Utc::now().to_rfc3339(),
                dimensions: dims,
                correlations: Vec::new(),
                signal_strength: 0.0,
                encoding_depth: "standard".to_string(),
            },
            last_dim_count: 0,
        }
    }

    pub fn capture(
        &mut self,
        emotion_primary: &str,
        emotion_valence: f64,
        emotion_arousal: f64,
        emotion_dominance: f64,
        brain_regions: &HashMap<String, f64>,
        gene_expression: &HashMap<String, f64>,
        organ_signals: usize,
        recent_memory_count: usize,
        creation_impulse_val: f64,
        disgust_val: f64,
        intimacy: f64,
        stress: f64,
    ) {
        let emotional = (emotion_valence * 0.5 + emotion_arousal * 0.3 + emotion_dominance * 0.2)
            .clamp(-1.0, 1.0);
        self.intent.dimensions.insert("emotional_direction".to_string(), emotional);

        let max_region = brain_regions
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(k, v)| (k.clone(), *v))
            .unwrap_or(("social".to_string(), 0.4));
        let cognitive = (max_region.1 * 2.0 - 1.0).clamp(-1.0, 1.0);
        self.intent.dimensions.insert("cognitive_focus".to_string(), cognitive);

        let max_gene = gene_expression
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(_, v)| *v)
            .unwrap_or(0.5);
        let behavioral = (max_gene * 2.0 - 1.0).clamp(-1.0, 1.0);
        self.intent.dimensions.insert("behavioral_drive".to_string(), behavioral);

        let relational = (emotion_valence * 0.4 + intimacy * 0.6).clamp(-1.0, 1.0);
        self.intent.dimensions.insert("relational_tone".to_string(), relational);

        let mem_echo = ((recent_memory_count as f64 / 100.0).min(1.0) * 2.0 - 1.0).clamp(-1.0, 1.0);
        self.intent.dimensions.insert("memory_echo".to_string(), mem_echo);

        let creative = (creation_impulse_val - disgust_val * 0.5).clamp(-1.0, 1.0);
        self.intent.dimensions.insert("creative_pressure".to_string(), creative);

        let domain = self.infer_domain_intent(emotion_primary, &max_region.0, cognitive);
        self.intent.dimensions.insert("domain_intent".to_string(), domain);

        let urgency = (stress * 1.2 + if organ_signals > 3 { 0.3 } else { 0.0 })
            .clamp(-1.0, 1.0);
        self.intent.dimensions.insert("urgency_signal".to_string(), urgency);

        let avg: f64 = self.intent.dimensions.values().map(|v| v.abs()).sum::<f64>()
            / self.intent.dimensions.len() as f64;
        self.intent.signal_strength = avg;
        self.intent.correlations = self.detect_correlations();
        self.intent.encoding_depth = if avg > 0.6 {
            "full".to_string()
        } else if avg > 0.35 {
            "standard".to_string()
        } else {
            "compact".to_string()
        };
        self.intent.time = Utc::now().to_rfc3339();
        self.last_dim_count = self.intent.dimensions.len();
    }

    fn infer_domain_intent(&self, emotion: &str, _top_region: &str, cognitive: f64) -> f64 {
        match emotion {
            "loving" | "happy" | "playful" => 0.6 + cognitive * 0.3,
            "curious" | "calm" => 0.2 + cognitive * 0.3,
            "anxious" | "sad" | "tired" => -0.3 + cognitive * 0.2,
            "angry" => -0.6 + cognitive * 0.2,
            _ => 0.0,
        }
        .clamp(-1.0, 1.0)
    }

    fn detect_correlations(&self) -> Vec<String> {
        let mut cors = Vec::new();
        let dims = &self.intent.dimensions;
        let emotional = dims.get("emotional_direction").copied().unwrap_or(0.0);
        let relational = dims.get("relational_tone").copied().unwrap_or(0.0);
        let creative = dims.get("creative_pressure").copied().unwrap_or(0.0);
        let cognitive = dims.get("cognitive_focus").copied().unwrap_or(0.0);
        let urgency = dims.get("urgency_signal").copied().unwrap_or(0.0);

        if emotional > 0.3 && relational > 0.3 { cors.push("emotional bonding".to_string()); }
        if emotional > 0.3 && creative > 0.3 { cors.push("creative emotion".to_string()); }
        if cognitive > 0.3 && urgency < -0.3 { cors.push("deep focus".to_string()); }
        if urgency > 0.3 && creative < -0.3 { cors.push("urgent pressure".to_string()); }
        if relational > 0.4 && cognitive < -0.2 { cors.push("emotional over logic".to_string()); }
        if relational < -0.3 && cognitive > 0.3 { cors.push("logic over emotion".to_string()); }
        cors
    }

    pub fn encode_prompt(&self) -> String {
        let mut parts = Vec::new();
        parts.push("Intent Encoding:".to_string());

        if self.intent.encoding_depth == "compact" {
            let max_dim = self.intent.dimensions
                .iter()
                .max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(k, v)| (k.clone(), *v));
            if let Some((dim, val)) = max_dim {
                let dir = if val > 0.3 { "positive" } else if val < -0.3 { "negative" } else { "neutral" };
                parts.push(format!("Top: {} ({})", dim, dir));
            }
            if !self.intent.correlations.is_empty() {
                parts.push(format!("Links: {}", self.intent.correlations.join(", ")));
            }
            parts.push(format!("Strength: {:.2}", self.intent.signal_strength));
        } else {
            parts.push("Dimensions:".to_string());
            for dim in INTENT_DIMS {
                let val = self.intent.dimensions.get(*dim).copied().unwrap_or(0.0);
                let bar = Self::visualize_bar(val);
                let label = match *dim {
                    "emotional_direction" => "emotion",
                    "cognitive_focus" => "cognitive",
                    "behavioral_drive" => "behavior",
                    "relational_tone" => "relation",
                    "memory_echo" => "memory",
                    "creative_pressure" => "creative",
                    "domain_intent" => "domain",
                    "urgency_signal" => "urgency",
                    _ => dim,
                };
                parts.push(format!("  {} {} [{:+.2}]", label, bar, val));
            }
            if !self.intent.correlations.is_empty() {
                parts.push(String::new());
                parts.push(format!("Links: {}", self.intent.correlations.join(" | ")));
            }
            if self.intent.encoding_depth == "full" {
                parts.push(String::new());
                parts.push(format!("Strength: {:.2} | Depth: full", self.intent.signal_strength));
            }
        }

        parts.join("\n")
    }

    fn visualize_bar(val: f64) -> String {
        let normalized = ((val + 1.0) / 2.0 * 10.0) as usize;
        let filled = normalized.min(10);
        let empty = 10 - filled;
        format!("[{}{}]", "#".repeat(filled), "-".repeat(empty))
    }

    pub fn intent_summary(&self) -> String {
        let emo = self.intent.dimensions.get("emotional_direction").copied().unwrap_or(0.0);
        let rel = self.intent.dimensions.get("relational_tone").copied().unwrap_or(0.0);
        let cre = self.intent.dimensions.get("creative_pressure").copied().unwrap_or(0.0);
        let urg = self.intent.dimensions.get("urgency_signal").copied().unwrap_or(0.0);

        if !self.intent.correlations.is_empty() {
            return self.intent.correlations.first().unwrap().clone();
        }
        if urg > 0.4 { return "urgent".to_string(); }
        if emo > 0.3 && rel > 0.2 { return "bonding".to_string(); }
        if cre > 0.3 { return "creative".to_string(); }
        if emo < -0.3 { return "low".to_string(); }
        "neutral".to_string()
    }
}
