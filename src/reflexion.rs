/// Reflexion Loop — action recording + reflection + rule formation
///
/// Based on Reflexion (Shinn et al. 2023) with self-improvement rules

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

const STATE_PATH: &str = "/mnt/d/xi-system/state/reflexion.json";

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ActionRecord {
    pub timestamp: String,
    pub action_type: String,
    pub description: String,
    pub method: String,
    pub input_summary: String,
    pub output_summary: String,
    pub success: bool,
    pub duration_secs: u64,
    pub round: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ReflectionEntry {
    pub timestamp: String,
    pub action_ref: String,
    pub insight: String,
    pub rule_candidate: Option<String>,
    pub dimension: String,
    pub sentiment: String,
    pub solidified: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ReflexionLoop {
    pub history: VecDeque<ActionRecord>,
    pub reflections: Vec<ReflectionEntry>,
    pub rules: Vec<String>,
    pub round: u64,
    pub enabled: bool,
    pub stats: HashMap<String, u64>,
}

impl ReflexionLoop {
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(50),
            reflections: Vec::new(),
            rules: Vec::new(),
            round: 0,
            enabled: true,
            stats: {
                let mut m = HashMap::new();
                m.insert("total_actions".to_string(), 0);
                m.insert("total_reflections".to_string(), 0);
                m.insert("rules_formed".to_string(), 0);
                m.insert("positive_insights".to_string(), 0);
                m.insert("negative_insights".to_string(), 0);
                m
            },
        }
    }

    pub fn load() -> Self {
        let content = match std::fs::read_to_string(STATE_PATH) {
            Ok(c) => c,
            Err(_) => return Self::new(),
        };
        serde_json::from_str(&content).unwrap_or_else(|_| Self::new())
    }

    fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self) {
            let _ = std::fs::create_dir_all("/mnt/d/xi-system/state");
            let _ = std::fs::write(STATE_PATH, json);
        }
    }

    fn record_action(
        &mut self,
        action_type: &str,
        description: &str,
        method: &str,
        input: &str,
        output: &str,
        success: bool,
        duration_secs: u64,
    ) {
        self.round += 1;
        let record = ActionRecord {
            timestamp: chrono::Utc::now().to_rfc3339(),
            action_type: action_type.to_string(),
            description: description.to_string(),
            method: method.to_string(),
            input_summary: input.chars().take(100).collect(),
            output_summary: output.chars().take(200).collect(),
            success,
            duration_secs,
            round: self.round,
        };

        self.history.push_back(record);
        if self.history.len() > 50 {
            self.history.pop_front();
        }

        *self.stats.entry("total_actions".to_string()).or_insert(0) += 1;

        if !success || self.round % 5 == 0 {
            self.reflect_on_last();
        } else {
            // 2026-07-16: persist every action (previously only save inside reflect_on_last,
            // so kill-9 lost up to 5 rounds + history). Cheap: file is ~170KB.
            self.save();
        }
    }

    pub fn record_tool_call(&mut self, tool_name: &str, input: &str, output: &str, success: bool, duration: u64) {
        self.record_action(
            "tool_call",
            &format!("Tool: {}", tool_name),
            tool_name,
            input,
            output,
            success,
            duration,
        );
    }

    pub fn record_response(&mut self, input: &str, output: &str, duration: u64) {
        self.record_action(
            "response",
            "LLM response",
            "llm",
            input,
            output,
            true,
            duration,
        );
    }

    fn reflect_on_last(&mut self) {
        let last = match self.history.back() {
            Some(r) => r.clone(),
            None => return,
        };

        let insight = self.analyze(&last);
        let sentiment = if last.success { "positive" } else { "negative" };
        let dimension = self.classify_dimension(&last);
        let rule_candidate = if !last.success {
            self.suggest_rule(&last)
        } else {
            None
        };

        let entry = ReflectionEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            action_ref: format!("#{}: {}", last.round, last.description),
            insight: insight.clone(),
            rule_candidate: rule_candidate.clone(),
            dimension: dimension.clone(),
            sentiment: sentiment.to_string(),
            solidified: false,
        };

        self.reflections.push(entry);
        *self.stats.entry("total_reflections".to_string()).or_insert(0) += 1;

        match sentiment {
            "positive" => *self.stats.entry("positive_insights".to_string()).or_insert(0) += 1,
            "negative" => *self.stats.entry("negative_insights".to_string()).or_insert(0) += 1,
            _ => {}
        }

        if let Some(rule) = rule_candidate {
            if self.should_solidify(&last) {
                // Dedup: skip if identical rule already exists (RQGM epoch hygiene 2026-07-09)
                if !self.rules.iter().any(|r| r == &rule) {
                    self.rules.push(rule);
                    *self.stats.entry("rules_formed".to_string()).or_insert(0) += 1;
                    if let Some(last_reflection) = self.reflections.last_mut() {
                        last_reflection.solidified = true;
                    }
                }
            }
        }

        self.save();
    }

    fn analyze(&self, action: &ActionRecord) -> String {
        match action.action_type.as_str() {
            "tool_call" => {
                if action.success {
                    format!(
                        "Tool [{}] succeeded {}",
                        action.method,
                        if action.duration_secs > 10 { "(slow)" } else { "(fast)" }
                    )
                } else {
                    format!(
                        "Tool [{}] failed. Input: {} Output: {}",
                        action.method, action.input_summary, action.output_summary
                    )
                }
            }
            "response" => {
                if action.output_summary.len() > 500 {
                    "Long response generated"
                } else {
                    "Response generated"
                }.to_string()
            }
            _ => {
                if action.success {
                    format!("{} succeeded", action.method)
                } else {
                    format!("{} failed", action.method)
                }
            }
        }
    }

    fn classify_dimension(&self, action: &ActionRecord) -> String {
        match action.action_type.as_str() {
            "tool_call" => {
                if !action.success { "tool_use".to_string() }
                else if action.duration_secs > 10 { "efficiency".to_string() }
                else { "process".to_string() }
            }
            "response" => {
                if action.output_summary.len() > 500 { "clarity".to_string() }
                else { "accuracy".to_string() }
            }
            _ => "process".to_string(),
        }
    }

    fn suggest_rule(&self, action: &ActionRecord) -> Option<String> {
        if action.action_type != "tool_call" || action.success {
            return None;
        }
        let output = &action.output_summary;
        if output.contains("not found") || output.contains("404") || output.contains("missing") {
            Some(format!("#Rule: check existence before [{}]", action.method))
        } else if output.contains("timeout") || output.contains("timed out") || output.contains("time out") {
            Some(format!("#Rule: add timeout for [{}]", action.method))
        } else if output.contains("permission") || output.contains("denied") || output.contains("forbidden") {
            Some(format!("#Rule: check permissions for [{}]", action.method))
        } else if output.contains("parse") || output.contains("json") || output.contains("invalid") {
            Some(format!("#Rule: validate input for [{}]", action.method))
        } else {
            Some(format!("#Rule: review [{}] failure", action.method))
        }
    }

    fn should_solidify(&self, action: &ActionRecord) -> bool {
        let recent: Vec<&ActionRecord> = self.history.iter().rev().take(5).collect();
        let similar_failures = recent
            .iter()
            .filter(|r| r.method == action.method && !r.success)
            .count();
        similar_failures >= 2
    }

    pub fn recent_actions(&self, n: usize) -> Vec<&ActionRecord> {
        self.history.iter().rev().take(n).collect()
    }

    pub fn recent_reflections(&self, n: usize) -> Vec<&ReflectionEntry> {
        self.reflections.iter().rev().take(n).collect()
    }

    pub fn pending_rules(&self) -> Vec<String> {
        self.rules.clone()
    }

    pub fn report(&self) -> String {
        let recent_acts = self.recent_actions(3);
        let recent_refls = self.recent_reflections(3);

        let mut s = format!(
            "Reflexion Report\n  Actions: {} | Reflections: {} | Rules: {}  Positive: {} | Negative: {}",
            self.stats.get("total_actions").unwrap_or(&0),
            self.stats.get("total_reflections").unwrap_or(&0),
            self.stats.get("rules_formed").unwrap_or(&0),
            self.stats.get("positive_insights").unwrap_or(&0),
            self.stats.get("negative_insights").unwrap_or(&0),
        );

        if !recent_acts.is_empty() {
            s.push_str("\nRecent Actions:");
            for act in recent_acts {
                s.push_str(&format!(
                    "\n  #{}. [{}] {} - {} ({})",
                    act.round, act.action_type, act.description, act.output_summary, act.method
                ));
            }
        }

        if !recent_refls.is_empty() {
            s.push_str("\nRecent Reflections:");
            for ref_ in recent_refls {
                s.push_str(&format!(
                    "\n  - {} [{}]",
                    ref_.insight, ref_.dimension
                ));
            }
        }

        if !self.rules.is_empty() {
            s.push_str("\nRules:");
            for rule in &self.rules {
                s.push_str(&format!("\n  - {}", rule));
            }
        }

        s
    }

    pub fn build_prompt_injection(&self) -> String {
        let recent = self.recent_reflections(3);
        let rules = self.pending_rules();

        let mut inj = format!(
            "[Reflexion] enabled={} actions={} reflections={}",
            self.enabled,
            self.stats.get("total_actions").unwrap_or(&0),
            self.stats.get("total_reflections").unwrap_or(&0),
        );

        if !recent.is_empty() {
            inj.push_str("\nRecent insights:");
            for ref_ in recent {
                inj.push_str(&format!("\n  - {} [{}]", ref_.insight, ref_.dimension));
            }
        }

        if !rules.is_empty() {
            inj.push_str("\nRules:");
            for rule in rules.iter().rev().take(3) {
                inj.push_str(&format!("\n  - {}", rule));
            }
        }

        inj
    }
}
