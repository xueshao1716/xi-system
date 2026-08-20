/// Xi Brain - 8-region cognitive architecture (from neural-core.js + v2)
/// 
/// Structure:
///   8 brain regions x interconnections -> gene expression driven -> behavioral tendency
///   + Emotional context (factor adjustment)
///   + Snapshot/rollback
///
/// Integration: main.rs per tick() updates

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;


const REGION_NAMES: [&str; 8] = [
    "analysis", "planning", "verification",
    "memory", "tooling", "social",
    "coordination", "genesis",
];

const REGION_DESCRIPTIONS: [(&str, &str); 8] = [
    ("analysis", "Analysis depth - tendency to decompose problems, find root causes, assess complexity"),
    ("planning", "Planning granularity - tendency to think steps ahead, make step lists"),
    ("memory", "Memory recall - tendency to frequently review history, associative recall strength"),
    ("verification", "Verification driven - tendency to check work, rigor level"),
    ("tooling", "Tool affinity - preference to search files/run commands vs relying on experience"),
    ("social", "Social resonance - emotional warmth in conversation, reading tone"),
    ("coordination", "Coordination capacity - managing multiple tasks, prioritization ability"),
    ("genesis", "Creativity - generating new ideas, new solutions, non-standard paths"),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genome {
    pub generation: u64,
    pub genes: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralCore {
    pub genome: Genome,
    pub region_weights: HashMap<String, f64>,
}

impl NeuralCore {
    pub fn new(_seed: &str) -> Self {
        let mut region_weights = HashMap::new();
        for name in &REGION_NAMES {
            region_weights.insert(name.to_string(), 0.5);
        }
        Self {
            genome: Genome {
                generation: 1,
                genes: vec![0.5; 10],
            },
            region_weights,
        }
    }

    pub fn load(&mut self) -> bool {
        let p = format!("{}/neural_core.json", crate::xi_home());
        if let Ok(content) = std::fs::read_to_string(&p) {
            if let Ok(loaded) = serde_json::from_str::<NeuralCore>(&content) {
                *self = loaded;
                return true;
            }
        }
        false
    }

    pub fn save(&self) {
        let p = format!("{}/neural_core.json", crate::xi_home());
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(p, json);
        }
    }

    pub fn build_prompt_injection(&self) -> String {
        let mut parts = Vec::new();
        parts.push(format!("[NeuralCore] gen={}", self.genome.generation));
        for name in &REGION_NAMES {
            if let Some(w) = self.region_weights.get(*name) {
                parts.push(format!("  {}: {:.2}", name, w));
            }
        }
        parts.join("\n")
    }
}