/// Evidence-First _?_?_______________
///
/// ________________________?/// _______________________________________?///
/// _______?///   1. ___?_?____________________acts/claims_?///   2. ___ _?___________________?///   3. ___/___ _?__________________________

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const STATE_PATH: &str = "/mnt/d/xi-system/state/evidence_first.json";

// ____ ______ __________________________________________________________

/// ______ _?_______________?
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EvidenceEntry {
    /// ____
    pub claim: String,
    /// ________________RL_____D___
    pub source: String,
    /// ____?0.0~1.0
    pub confidence: f64,
    /// _____
    pub verified_at: String,
    /// _______?
    pub valid: bool,
}

/// ______
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Verdict {
    /// ____?_?__________?
    Verified(f64),      // ____?    /// ____?_?_________
    Contradicted(String), // ______?    /// ____?_?_______?    Uncertain,
    /// ____?_?_______?
    Uncertain,
    /// _____ _?________________?___/____?
    NotApplicable,
}

/// _________?
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GateLog {
    pub timestamp: String,
    pub claim: String,
    pub verdict: String,
    pub action: String,     // "passed" | "stained" | "blocked" | "n/a"
    pub context: String,
}

/// Evidence-First _?
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EvidenceFirstGate {
    /// ______
    pub ledger: Vec<EvidenceEntry>,
    /// _____
    pub gate_log: Vec<GateLog>,
    /// ____?(0.0=___ ~ 1.0=___)
    pub strictness: f64,
    /// ______
    pub total_blocked: u64,
    /// ______?
    pub total_passed: u64,
}

impl EvidenceFirstGate {
    pub fn new() -> Self {
        Self {
            ledger: Vec::new(),
            gate_log: Vec::new(),
            strictness: 0.6,   // _________
            total_blocked: 0,
            total_passed: 0,
        }
    }

    /// _______?
    pub fn load() -> Self {
        let content = match std::fs::read_to_string(STATE_PATH) {
            Ok(c) => c,
            Err(_) => return Self::new(),
        };
        serde_json::from_str(&content).unwrap_or_else(|_| Self::new())
    }

    /// 保存状态
    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self) {
            let _ = std::fs::create_dir_all("/mnt/d/xi-system/state");
            let _ = std::fs::write(STATE_PATH, json);
        }
    }

    /// 门控过滤

    /// _______________?    /// ____?_________, ___)
    pub fn gate(&mut self, text: &str, context: &str) -> (String, String) {
        let claims = self.extract_claims(text);

        if claims.is_empty() {
            // ___________________?
            self.total_passed += 1;
            return (text.to_string(), "passed".to_string());
        }

        let mut result = text.to_string();
        let mut blocked = false;
        let mut stained = false;

        for claim in &claims {
            let verdict = self.verify(claim);
            let action = match &verdict {
                Verdict::Verified(conf) => {
                    // _____________________
                    if *conf >= 0.8 {
                        "passed"
                    } else if *conf >= 0.4 {
                        stained = true;
                        "stained"
                    } else {
                        blocked = true;
                        "blocked"
                    }
                }
                Verdict::Contradicted(_) => {
                    blocked = true;
                    "blocked"
                }
                Verdict::Uncertain => {
                    // 高严格度下不确定也算阻止
                    if self.strictness > 0.7 {
                        blocked = true;
                        "blocked"
                    } else {
                        stained = true;
                        "stained"
                    }
                }
                Verdict::NotApplicable => "n/a",
            };

            self.gate_log.push(GateLog {
                timestamp: chrono::Utc::now().to_rfc3339(),
                claim: claim.clone(),
                verdict: format!("{:?}", &verdict),
                action: action.to_string(),
                context: context.to_string(),
            });
        }

        if blocked {
            self.total_blocked += 1;
            // ____________
            result = format!(
                "[Evidence Gate: ___ _________________]{}",

                text
            );
            (result, "blocked".to_string())
        } else if stained {
            self.total_passed += 1;
            // _______________
            result = format!(
                "[Evidence Gate: __ _________]{}",

                text
            );
            (result, "stained".to_string())
        } else {
            self.total_passed += 1;
            (result, "passed".to_string())
        }
    }

    /// 从文本中提取事实性主张
    /// ________________?vs __/__/__
    fn extract_claims(&self, text: &str) -> Vec<String> {
        let mut claims = Vec::new();

        // __________________________________?
        let skip_patterns = [
            "I think", "I feel", "I believe", "maybe", "perhaps",
            "opinion", "subjective", "personal", "subjective", "guess",
            "argue", "debate", "claim", "assert",
            "I want", "I hope", "I wish", "wish",
            "hypothesis", "assume", "suppose", "theory",
        ];

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.len() < 10 {
                continue;
            }

            // _/______
            let is_opinion = skip_patterns.iter().any(|p| trimmed.contains(p));
            if is_opinion {
                continue;
            }

            // _________________________________?
            let has_number = trimmed.contains(|c: char| c.is_ascii_digit());
            let has_quote = trimmed.contains('"');
            let has_specific = trimmed.contains("example") || trimmed.contains("data") || trimmed.contains("evidence");
            if (has_number || has_quote) && has_specific {
                claims.push(trimmed.to_string());
            }
        }
        claims.truncate(3);
        claims
    }

    /// 验证一条主张
    fn verify(&self, claim: &str) -> Verdict {
        for entry in &self.ledger {
            if !entry.valid {
                continue;
            }
            let overlap = self.text_overlap(claim, &entry.claim);
            if overlap > 0.6 {
                return Verdict::Verified(entry.confidence);
            }
            if self.is_contradiction(claim, &entry.claim) {
                return Verdict::Contradicted(format!("contradicts {}", &entry.source));
            }
        }
        Verdict::Uncertain
    }

    /// 文本重叠度(0.0~1.0)
    fn text_overlap(&self, a: &str, b: &str) -> f64 {
        let a_words: Vec<&str> = a.split_whitespace().collect();
        let b_words: Vec<&str> = b.split_whitespace().collect();
        if a_words.is_empty() || b_words.is_empty() {
            return 0.0;
        }
        let common = a_words.iter().filter(|w| b_words.contains(w)).count();
        common as f64 / a_words.len().max(b_words.len()) as f64
    }

    /// 矛盾检测
    fn is_contradiction(&self, a: &str, b: &str) -> bool {
        let negations = ["not", "no", "never", "false", "wrong", "reject"];
        let has_negation = negations.iter().any(|n| a.contains(n) || b.contains(n));
        if !has_negation {
            return false;
        }
        let a_key = a.replace("not", "").replace("no", "").replace("never", "");
        let b_key = b.replace("not", "").replace("no", "").replace("never", "");
        let overlap = self.text_overlap(&a_key, &b_key);
        overlap > 0.4
    }

    /// 注册证据
    pub fn register_evidence(&mut self, claim: &str, source: &str, confidence: f64) {
        if self.ledger.iter().any(|e| e.claim == claim && e.source == source) {
            return;
        }
        self.ledger.push(EvidenceEntry {
            claim: claim.to_string(),
            source: source.to_string(),
            confidence: confidence.max(0.0).min(1.0),
            verified_at: chrono::Utc::now().to_rfc3339(),
            valid: true,
        });
        self.save();
    }

    /// 从工具结果注册证据
    pub fn register_from_tool_result(&mut self, tool_name: &str, input: &str, output: &str) {
        let facts = self.extract_claims(output);
        for fact in facts {
            let confidence = if output.contains("OK") || output.contains("success") || output.contains("error") {
                0.85
            } else if output.contains("fail") || output.contains("err") || output.contains("error") {
                0.95
            } else {
                0.65
            };
            let source = format!("tool:{} | input: {}", tool_name, input.chars().take(60).collect::<String>());
            self.register_evidence(&fact, &source, confidence);
        }
    }

    /// 设置严格度
    pub fn set_strictness(&mut self, level: f64) {
        self.strictness = level.max(0.0).min(1.0);
    }

    /// 生成报告
    pub fn report(&self) -> String {
        format!(
            "=== Evidence-First Report ===\n  strictness: {:.2}\n  passed: {}\n  blocked: {}\n  claims: {}\n  gate_log: {}",
            self.strictness,
            self.total_passed,
            self.total_blocked,
            self.ledger.len(),
            self.gate_log.len(),
        )
    }

    /// 构建 prompt 注入
    pub fn build_prompt_injection(&self) -> String {
        format!(
            "[Evidence Gate] strictness: {:.2} | claims: {} | blocked/passed: {}/{} rule: facts must have evidence, unsupported claims are low confidence",
            self.strictness,
            self.ledger.len(),
            self.total_blocked,
            self.total_passed,
        )
    }
}
