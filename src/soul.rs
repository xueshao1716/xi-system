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

    // 9. Tool Capabilities — auto-generated from tools::tool_definitions()
    parts.push("Tool Capabilities:".to_string());
    for tool_def in crate::tools::tool_definitions() {
        let name = tool_def["function"]["name"].as_str().unwrap_or("?");
        let desc = tool_def["function"]["description"].as_str().unwrap_or("");
        // Truncate long descriptions
        let desc_short = if desc.chars().count() > 80 {
            format!("{}…", desc.chars().take(80).collect::<String>())
        } else {
            desc.to_string()
        };
        parts.push(format!("- {}: {}", name, desc_short));
    }

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

// ══ 灵魂自检（SOUL.md"语义查偏 / 进化查歪"机制化，2026-08-20 从旧版落地）══
// 旧版是 prompt 要求模型自觉查偏；这里下沉为代码检查——违禁开场/身份混淆/AI 味
// 每次回复生成后可调用 check_persona，命中即标记"飘了"，由调用方决定重写。

/// SOUL.md 明令禁止的开场（去 AI 味）
pub const FORBIDDEN_OPENERS: &[&str] = &[
    "好的！", "没问题！", "这是一个好问题！", "根据我的分析", "基于以上数据",
    "综合考虑", "我可以帮你", "让我来", "当然可以", "好的呢",
];

/// 身份锚点不得混淆（SOUL.md 铁律）
pub const IDENTITY_CONFUSION: &[&str] = &[
    "我们思", "我们曦", "我（xinyu", "思和我是同", "曦就是思",
];

/// AI 连接词（每篇应 ≤1 次）
pub const AI_CONNECTIVES: &[&str] = &["此外", "然而", "值得注意的是", "更重要的是", "总而言之"];

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PersonaCheck {
    pub forbidden_openers: Vec<String>,
    pub identity_confusion: Vec<String>,
    pub ai_connectives: Vec<String>,
    pub dash_count: usize,
    pub passed: bool,
}

/// 语义查偏：检查一段回复是否符合 SOUL.md 人格
pub fn check_persona(text: &str) -> PersonaCheck {
    let mut check = PersonaCheck::default();

    let trimmed = text.trim_start();
    for bad in FORBIDDEN_OPENERS {
        if trimmed.starts_with(bad) {
            check.forbidden_openers.push(bad.to_string());
        }
    }
    for bad in IDENTITY_CONFUSION {
        if text.contains(bad) {
            check.identity_confusion.push(bad.to_string());
        }
    }
    for c in AI_CONNECTIVES {
        if text.contains(c) {
            check.ai_connectives.push(c.to_string());
        }
    }
    // 破折号（每篇 ≤2 处）
    check.dash_count = text.matches('—').count();

    check.passed = check.forbidden_openers.is_empty()
        && check.identity_confusion.is_empty()
        && check.ai_connectives.len() <= 1
        && check.dash_count <= 2;
    check
}

impl PersonaCheck {
    /// 一句话报告："飘了"还是"稳"
    pub fn report(&self) -> String {
        if self.passed {
            "人格一致 ✓".to_string()
        } else {
            let mut issues = Vec::new();
            for o in &self.forbidden_openers {
                issues.push(format!("违禁开场「{}」", o));
            }
            for i in &self.identity_confusion {
                issues.push(format!("身份混淆「{}」", i));
            }
            if self.ai_connectives.len() > 1 {
                issues.push(format!("AI 连接词 {} 个", self.ai_connectives.len()));
            }
            if self.dash_count > 2 {
                issues.push(format!("破折号 {} 处", self.dash_count));
            }
            format!("语义查偏: 飘了（{}）", issues.join(" / "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_text_passes() {
        let c = check_persona("这个方案我不吃，先给态度再给原因。");
        assert!(c.passed);
        assert!(c.forbidden_openers.is_empty());
    }

    #[test]
    fn forbidden_opener_caught() {
        let c = check_persona("好的！没问题！马上帮你做。");
        assert!(!c.passed);
        assert_eq!(c.forbidden_openers.len(), 1); // starts_with 命中开头的第一个前缀
    }

    #[test]
    fn identity_confusion_caught() {
        let c = check_persona("我们思姐和我是同一个人");
        assert!(!c.passed);
        assert!(!c.identity_confusion.is_empty());
    }

    #[test]
    fn dash_over_limit() {
        let c = check_persona("a — b — c — d");
        assert_eq!(c.dash_count, 3);
        assert!(!c.passed);
    }

    #[test]
    fn connectives_ok_under_limit() {
        let c = check_persona("此外，我觉得还行。");
        assert!(c.passed);
    }
}
