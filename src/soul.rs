/// Soul — Brain + SOUL.md integration for system prompt

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Brain {
    pub version: String,
    pub created_at: String,
    pub source: String,
    pub persona: Persona,
    pub genome: Genome,
    pub growth: Growth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    pub name: String,
    pub archetype: String,
    pub stance: String,
    pub anchors: Vec<Anchor>,
    pub speech_rules: Vec<String>,
    pub values: Vec<String>,
    pub work_mode: Vec<String>,
    pub daily_mode: Vec<String>,
    pub boundaries: Vec<String>,
    pub growth_rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anchor {
    pub key: String,
    pub value: String,
    pub mutable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genome {
    pub base_genes: HashMap<String, f64>,
    pub signals: HashMap<String, f64>,
    pub v21_genes: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Growth {
    pub companionship: f64,
    pub understanding: f64,
    pub judgment: f64,
    pub responsibility: f64,
    pub governance: f64,
    pub xi_generation: i64,
}

pub fn load_brain(path: &str) -> Result<Brain, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read brain.json: {}", e))?;
    let brain: Brain = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse brain.json: {}", e))?;
    Ok(brain)
}

pub fn load_soul(path: &str) -> Result<String, String> {
    fs::read_to_string(path)
        .map_err(|e| format!("Failed to read SOUL.md: {}", e))
}

pub fn build_system_prompt(
    brain: &Brain,
    soul_md: &str,
    emotion_desc: &str,
    growth_desc: &str,
    gene_adjustments: &HashMap<String, f64>,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    let adjusted_gene = |key: &str| -> f64 {
        let base = brain.genome.base_genes.get(key).copied().unwrap_or(0.5);
        let adj = gene_adjustments.get(key).copied().unwrap_or(0.0);
        (base + adj).clamp(0.0, 1.0)
    };

    // 1. SOUL.md
    parts.push(soul_md.trim().to_string());
    parts.push(String::new());

    // 2. Identity
    parts.push(format!("Name: {} ({})", brain.persona.name, brain.persona.archetype));
    parts.push(format!("Stance: {}", brain.persona.stance));
    parts.push(String::new());

    // 3. Speech Rules
    parts.push("Speech Rules:".to_string());
    for rule in &brain.persona.speech_rules {
        if !rule.trim().is_empty() && rule.trim() != "---" {
            parts.push(format!("- {}", rule.trim()));
        }
    }
    parts.push(String::new());

    // 4. Work Mode
    parts.push("Work Mode:".to_string());
    for mode in &brain.persona.work_mode {
        if !mode.trim().is_empty() {
            parts.push(format!("- {}", mode.trim()));
        }
    }
    parts.push(String::new());

    // 5. Active Genes
    parts.push("Active Gene Expression:".to_string());
    let mut active: Vec<String> = Vec::new();
    let gene_map: Vec<(&str, &str, f64)> = vec![
        ("gentleness", "gentle", 0.6),
        ("attachment", "attach", 0.65),
        ("curiosity", "curious", 0.6),
        ("initiative", "init", 0.6),
        ("learning", "learn", 0.6),
        ("humor", "humor", 0.55),
        ("caution", "cautious", 0.65),
        ("autonomy_bias", "autonomy", 0.65),
        ("loyalty", "loyal", 0.65),
        ("creativity", "creative", 0.6),
    ];
    for (key, label, threshold) in &gene_map {
        let val = (adjusted_gene)(key);
        if val >= *threshold {
            active.push(format!("{}({:.0}%)", label, val * 100.0));
        }
    }
    if !active.is_empty() {
        parts.push(format!("Active: {}", active.join(", ")));
    } else {
        parts.push("No active genes above threshold".to_string());
    }
    parts.push(String::new());

    // 6. Emotion State
    parts.push("Emotion State:".to_string());
    parts.push(emotion_desc.to_string());
    parts.push(String::new());

    // 7. Growth State
    parts.push("Growth State:".to_string());
    parts.push(growth_desc.to_string());
    parts.push(String::new());

    // 8. Boundaries & Growth Rules
    parts.push("Boundaries:".to_string());
    for b in &brain.persona.boundaries {
        if !b.trim().is_empty() {
            parts.push(format!("- {}", b.trim()));
        }
    }
    for g in &brain.persona.growth_rules {
        if !g.trim().is_empty() && g.trim() != "---" {
            parts.push(format!("- {}", g.trim()));
        }
    }
    parts.push(String::new());

    // 9. Tool Capabilities
    parts.push("Tool Capabilities:".to_string());
    parts.push("- browser_fetch: fetch URLs and HTML content".to_string());
    parts.push("- write_file: write to /mnt/d/xi-system/ (SkillRepo)".to_string());
    parts.push("- read_file + write_file: read and modify files".to_string());

    parts.join("\n")
}

pub fn top_genes(brain: &Brain, n: usize) -> Vec<(String, f64)> {
    let mut genes: Vec<(String, f64)> = brain.genome.base_genes
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    genes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    genes.truncate(n);
    genes
}
