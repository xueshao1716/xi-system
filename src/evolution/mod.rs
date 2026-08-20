// evolution.rs —— 进化系统主模块（2026-08-21 拆模块：signals/genes 独立）
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use crate::grn;


pub mod signals;
pub mod assets;
pub mod gates;
pub use signals::{Signals, GeneExpression};
pub use assets::*;
pub use gates::*;

// ─── Proposal (Darwin Loop) ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub gene_key: String,
    pub old_value: f64,
    pub proposed_value: f64,
    pub direction: String,
    pub reason: String,
    pub created_at: String,
    pub scores: Vec<f64>,
    pub accepted: Option<bool>,
    pub gate_scores: HashMap<String, f64>,
    pub iteration_dir: Option<String>,
    pub failure_trace: Option<String>,
}

impl Proposal {
    pub fn avg_score(&self) -> f64 {
        if self.scores.is_empty() { 0.0 }
        else { self.scores.iter().sum::<f64>() / self.scores.len() as f64 }
    }
}

// ─── Conversation Mode ─────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConversationMode {
    Builder,
    Analysis,
    Answer,
    Conversation,
}

impl ConversationMode {
    pub fn detect(text: &str) -> Self {
        let t = text.to_lowercase();
        if t.contains("analyze") || t.contains("review") || t.contains("debug") || t.contains("trace")
            || t.contains("分析") || t.contains("检查") || t.contains("排查") || t.contains("诊断") {
            Self::Analysis
        } else if t.contains("what") || t.contains("how") || t.contains("why") || t.contains("explain")
            || t.contains("什么") || t.contains("怎么") || t.contains("为什么") || t.contains("解释") {
            Self::Answer
        } else if t.contains("create") || t.contains("build") || t.contains("write") || t.contains("make") || t.contains("build")
            || t.contains("创建") || t.contains("写") || t.contains("做") || t.contains("搭建") {
            Self::Builder
        } else {
            Self::Conversation
        }
    }

    pub fn growth_rates(&self) -> GrowthRates {
        match self {
            Self::Builder => GrowthRates { companionship: 0.010, understanding: 0.015, judgment: 0.0, responsibility: 0.006, governance: 0.012 },
            Self::Analysis => GrowthRates { companionship: 0.008, understanding: 0.013, judgment: 0.015, responsibility: 0.004, governance: 0.0 },
            Self::Answer => GrowthRates { companionship: 0.007, understanding: 0.012, judgment: 0.010, responsibility: 0.0, governance: 0.0 },
            Self::Conversation => GrowthRates { companionship: 0.012, understanding: 0.010, judgment: 0.0, responsibility: 0.0, governance: 0.006 },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GrowthRates {
    pub companionship: f64,
    pub understanding: f64,
    pub judgment: f64,
    pub responsibility: f64,
    pub governance: f64,
}

// ─── Evolution State ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionState {
    pub total_messages: u64,
    pub total_sessions: u64,
    pub signals: SignalsInternal,
    pub genes: GeneExpression,
    pub companionship: f64,
    pub understanding: f64,
    pub judgment: f64,
    pub responsibility: f64,
    pub governance: f64,
    pub generation: i64,
    pub session_count: u64,
    pub latest_notes: Vec<String>,
    pub gene_baseline: HashMap<String, f64>,
    pub gene_adjustments: HashMap<String, f64>,
    pub proposals: Vec<Proposal>,
    pub accepted_count: u64,
    pub rejected_count: u64,
    pub messages_since_reflection: u64,
    pub reflection_count: u64,
    pub last_reflection: String,
    pub reflection_log: Vec<ReflectionEntry>,
    pub messages_since_micro_reflection: u64,
    pub micro_reflection_count: u64,
    pub last_micro_reflection: String,
    pub micro_reflection_log: Vec<ReflectionEntry>,
    pub gene_drift_warning: String,
    pub drift_accumulated: f64,
    pub iteration_counter: usize,
    pub first_activation: String,
    pub last_activation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalsInternal {
    pub intimacy: f64,
    pub novelty: f64,
    pub stress: f64,
    pub trust: f64,
}

impl SignalsInternal {
    pub fn decay(&mut self) {
        let intimacy_decay = 0.05 + self.intimacy * 0.10;
        let novelty_decay = 0.30 + self.novelty * 0.10;
        let stress_decay = 0.10 + self.stress * 0.10;
        let trust_decay = 0.03 + self.trust * 0.15;
        self.intimacy = (self.intimacy * (1.0 - intimacy_decay)).max(0.0);
        self.novelty = (self.novelty * (1.0 - novelty_decay)).max(0.0);
        self.stress = (self.stress * (1.0 - stress_decay)).max(0.0);
        self.trust = (self.trust * (1.0 - trust_decay)).max(0.0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionEntry {
    pub time: String,
    pub summary: String,
}

// ─── Darwin Loop Constants ─────────────────────────────────

const RATCHET_THRESHOLD: f64 = 0.7;
const GATE_FORMAT: &str = "format";
const GATE_CONTENT: &str = "content";
const GATE_BEHAVIOR: &str = "behavior";
const GATE_PERFORMANCE: &str = "performance";
const GATE_SAFETY: &str = "safety";

// ─── Loop Conditions (AlphaSignal) ────────────────────────
// A loop is only worth running if ALL four conditions hold.
// Missing any one = the loop loses money (tokens for nothing).

#[derive(Debug, Clone)]
pub struct LoopConditions {
    /// 1. Task repeats (one-shot tasks don't need loops)
    pub task_repeats: bool,
    /// 2. Verification is automatable (no auto-verify = blind loop)
    pub verification_automatable: bool,
    /// 3. Token budget can absorb waste (loops multiply cost linearly)
    pub token_budget_sufficient: bool,
    /// 4. Toolchain is complete (tools missing = intern with no方向盘)
    pub toolchain_complete: bool,
}

impl LoopConditions {
    pub fn all_met(&self) -> bool {
        self.task_repeats && self.verification_automatable
            && self.token_budget_sufficient && self.toolchain_complete
    }

    /// Check which conditions fail — caller decides whether to proceed
    pub fn violations(&self) -> Vec<&'static str> {
        let mut v = Vec::new();
        if !self.task_repeats { v.push("task does not repeat"); }
        if !self.verification_automatable { v.push("verification not automatable"); }
        if !self.token_budget_sufficient { v.push("token budget insufficient"); }
        if !self.toolchain_complete { v.push("toolchain incomplete"); }
        v
    }
}

// ─── Goal Validator (Writer/Judge Separation for Evolution) ─
// The judge uses a DIFFERENT prompt than the proposer.
// Judge cannot call tools, cannot modify code — only reads and verdicts.

#[derive(Debug, Clone)]
pub struct GoalValidator {
    pub condition: String,
    pub max_rounds: usize,
    pub current_round: usize,
}

impl GoalValidator {
    pub fn new(condition: &str, max_rounds: usize) -> Self {
        Self { condition: condition.to_string(), max_rounds, current_round: 0 }
    }

    /// Judge prompt — deliberately terse, no tool access
    pub fn judge_prompt(&self, output: &str) -> String {
        format!(
            "你是验证者（不是执行者）。只判断，不操作。\n\
             完成条件：{}\n\
             输出：\n{}\n\
             回复 JSON：{{\"pass\": true/false, \"reason\": \"简短说明\"}}",
            self.condition, output
        )
    }

    pub fn should_stop(&self) -> bool {
        self.current_round >= self.max_rounds
    }
}

// ─── A2A Protocol — 跨Agent资产交换 ──────────────────────
// 基于Evolver的A2A协议：一个Agent进化完，其他Agent能共享资产

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    pub from_agent: String,
    pub to_agent: String,
    pub asset: EvolutionAsset,
    pub message_type: String,  // "share" / "request" / "feedback"
    pub timestamp: String,
}

impl A2AMessage {
    /// 创建共享消息
    pub fn share(from: &str, to: &str, asset: EvolutionAsset) -> Self {
        Self {
            from_agent: from.to_string(),
            to_agent: to.to_string(),
            asset,
            message_type: "share".to_string(),
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    /// 创建反馈消息
    pub fn feedback(from: &str, to: &str, asset: EvolutionAsset, score: f64) -> Self {
        let mut a = asset;
        a.validated(score);
        Self {
            from_agent: from.to_string(),
            to_agent: to.to_string(),
            asset: a,
            message_type: "feedback".to_string(),
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

// ─── EvolutionState Implementation ─────────────────────────

impl EvolutionState {
    pub fn new() -> Self {
        let now = Utc::now().to_rfc3339();
        let mut gene_baseline = HashMap::new();
        for k in ["creativity", "initiative", "attachment", "autonomy_bias",
                   "humor", "learning", "caution", "loyalty", "gentleness", "curiosity"] {
            gene_baseline.insert(k.to_string(), 0.5);
        }
        Self {
            total_messages: 0,
            total_sessions: 0,
            signals: SignalsInternal { intimacy: 0.3, novelty: 0.5, stress: 0.1, trust: 0.3 },
            genes: GeneExpression::default(),
            companionship: 0.1,
            understanding: 0.1,
            judgment: 0.1,
            responsibility: 0.1,
            governance: 0.1,
            generation: 1,
            session_count: 0,
            latest_notes: Vec::new(),
            gene_baseline,
            gene_adjustments: HashMap::new(),
            proposals: Vec::new(),
            accepted_count: 0,
            rejected_count: 0,
            messages_since_reflection: 0,
            reflection_count: 0,
            last_reflection: now.clone(),
            reflection_log: Vec::new(),
            messages_since_micro_reflection: 0,
            micro_reflection_count: 0,
            last_micro_reflection: now.clone(),
            micro_reflection_log: Vec::new(),
            gene_drift_warning: String::new(),
            drift_accumulated: 0.0,
            iteration_counter: 0,
            first_activation: now.clone(),
            last_activation: now.clone(),
        }
    }

    pub fn load(path: &str) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
                eprintln!("[evolution] load error: {}", e);
                Self::new()
            }),
            Err(_) => {
                eprintln!("[evolution] No state file, creating new");
                Self::new()
            }
        }
    }

    pub fn save(&self, path: &str) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let size_mb = json.len() as f64 / 1024.0 / 1024.0;
            if size_mb > 5.0 {
                eprintln!("[evolution] File too large ({:.1} MB), skipping save", size_mb);
                return;
            }
            let _ = std::fs::write(path, json);
        }
    }

    // ─── Signal Updates ────────────────────────────────────

    fn set_signals(&mut self, intimacy: Option<f64>, novelty: Option<f64>, stress: Option<f64>, trust: Option<f64>) {
        if let Some(v) = intimacy { self.signals.intimacy = v.max(0.0).min(1.0); }
        if let Some(v) = novelty  { self.signals.novelty = v.max(0.0).min(1.0); }
        if let Some(v) = stress   { self.signals.stress = v.max(0.0).min(1.0); }
        if let Some(v) = trust    { self.signals.trust = v.max(0.0).min(1.0); }
        let s = Signals {
            intimacy: self.signals.intimacy,
            novelty: self.signals.novelty,
            stress: self.signals.stress,
            trust: self.signals.trust,
        };
        self.genes = GeneExpression::express(&s);
    }

    pub fn update_signals_from_message(&mut self, role: &str, text: &str) {
        let t = text.to_lowercase();
        let intimacy_boost = if t.contains("love") || t.contains("miss") || t.contains("hug") || t.contains("care")
            || t.contains("老公") || t.contains("想你") || t.contains("爱你") || t.contains("抱") || t.contains("喜欢") { 0.05 } else { 0.0 };
        let novelty_boost = if t.contains("new") || t.contains("interesting") || t.contains("discover")
            || t.contains("新") || t.contains("发现") || t.contains("学习") || t.contains("有意思") || t.contains("链接") { 0.04 } else { 0.0 };
        let stress_delta = if t.contains("urgent") || t.contains("hurry") || t.contains("error") || t.contains("fail")
            || t.contains("急") || t.contains("错误") || t.contains("失败") || t.contains("坏了") || t.contains("怎么") { 0.03 }
        else if t.contains("calm") || t.contains("relax") || t.contains("ok") || t.contains("没事") || t.contains("好的") || t.contains("休息") { -0.02 } else { 0.0 };
        let trust_boost = if role == "assistant" { 0.01 } else { 0.0 };

        self.set_signals(
            Some(self.signals.intimacy + intimacy_boost),
            Some(self.signals.novelty + novelty_boost),
            Some(self.signals.stress + stress_delta),
            Some(self.signals.trust + trust_boost),
        );
    }

    // ─── GRN Regulation ────────────────────────────────────

    pub fn grn_regulate(&mut self, grn: &grn::GeneRegulatoryNetwork) {
        if !grn.is_loaded() {
            return;
        }
        let base = self.genes.to_map();
        let signals_map = {
            let mut m = HashMap::new();
            m.insert("intimacy".to_string(), self.signals.intimacy);
            m.insert("novelty".to_string(), self.signals.novelty);
            m.insert("stress".to_string(), self.signals.stress);
            m.insert("trust".to_string(), self.signals.trust);
            m
        };
        let regulated = grn.regulate(&base, &signals_map, 3, 0.3);

        self.genes.gentleness = *regulated.get("gentleness").unwrap_or(&0.5);
        self.genes.initiative = *regulated.get("initiative").unwrap_or(&0.5);
        self.genes.curiosity = *regulated.get("curiosity").unwrap_or(&0.5);
        self.genes.attachment = *regulated.get("attachment").unwrap_or(&0.5);
        self.genes.learning = *regulated.get("learning").unwrap_or(&0.5);
        self.genes.creativity = *regulated.get("creativity").unwrap_or(&0.5);
        self.genes.caution = *regulated.get("caution").unwrap_or(&0.5);
        self.genes.humor = *regulated.get("humor").unwrap_or(&0.5);
        self.genes.loyalty = *regulated.get("loyalty").unwrap_or(&0.5);
        self.genes.autonomy_bias = *regulated.get("autonomy_bias").unwrap_or(&0.5);
    }

    // ─── Growth Tracking ───────────────────────────────────

    pub fn update_growth(&mut self, mode: ConversationMode, text: &str) {
        let rates = mode.growth_rates();
        self.companionship += rates.companionship;
        self.understanding += rates.understanding;
        self.judgment += rates.judgment;
        self.responsibility += rates.responsibility;
        self.governance += rates.governance;

        self.companionship = self.companionship.min(10.0);
        self.understanding = self.understanding.min(10.0);
        self.judgment = self.judgment.min(10.0);
        self.responsibility = self.responsibility.min(10.0);
        self.governance = self.governance.min(10.0);

        // Drift detection
        let drift = self.gene_adjustments.values().map(|v| v.abs()).sum::<f64>();
        self.drift_accumulated = drift;

        // Generation advancement
        if self.companionship >= 1.0 && self.understanding >= 1.0
            && self.judgment >= 1.0 && self.responsibility >= 1.0 && self.governance >= 1.0 {
            self.generation += 1;
            self.latest_notes.push(format!("Generation {} unlocked!", self.generation));
            if self.latest_notes.len() > 10 {
                self.latest_notes.remove(0);
            }
        }

        let _ = text; // suppress unused warning
    }

    // ─── Traces & Iterations ───────────────────────────────

    fn traces_root(&self) -> PathBuf {
        PathBuf::from(std::env::var("XI_EVOLUTION_TRACES_DIR")
            .unwrap_or_else(|_| "evolution_traces".into()))
    }

    pub fn create_iteration_dir(&self, iteration: usize) -> std::io::Result<PathBuf> {
        let dir = self.traces_root().join(format!("iteration_{:04}", iteration));
        let code_dir = dir.join("code");
        let scores_dir = dir.join("scores");
        let traces_dir = dir.join("traces");
        fs::create_dir_all(&code_dir)?;
        fs::create_dir_all(&scores_dir)?;
        fs::create_dir_all(&traces_dir)?;
        Ok(dir)
    }

    fn snapshot_baseline_to_iteration(&self, dir: &Path) -> std::io::Result<()> {
        let snapshot = serde_json::json!({
            "timestamp": Utc::now().to_rfc3339(),
            "gene_baseline": self.gene_baseline,
            "gene_adjustments": self.gene_adjustments,
            "signals": self.signals,
            "generation": self.generation,
            "growth": {
                "companionship": self.companionship,
                "understanding": self.understanding,
                "judgment": self.judgment,
                "responsibility": self.responsibility,
                "governance": self.governance,
            },
        });
        let json_str = serde_json::to_string_pretty(&snapshot)?;
        fs::write(dir.join("code").join("baseline_snapshot.json"), &json_str)?;
        Ok(())
    }

    fn write_gate_scores(&self, dir: &Path, gate_scores: &HashMap<String, f64>, verdict: &str) -> std::io::Result<()> {
        let payload = serde_json::json!({
            "verdict": verdict,
            "gate_scores": gate_scores,
            "threshold": RATCHET_THRESHOLD,
            "timestamp": Utc::now().to_rfc3339(),
        });
        fs::write(
            dir.join("scores").join("gate_scores.json"),
            &serde_json::to_string_pretty(&payload)?,
        )?;
        Ok(())
    }

    fn write_trace_entry(&self, dir: &Path, step_name: &str, content: &str) -> std::io::Result<()> {
        let ts = Utc::now().format("%Y%m%d_%H%M%S");
        let trace_path = dir.join("traces").join(format!("{}_{}.log", ts, step_name));
        fs::write(&trace_path, content)?;
        Ok(())
    }

    pub fn load_iteration_snapshot(iteration: usize) -> Option<serde_json::Value> {
        let traces_root = std::env::var("XI_EVOLUTION_TRACES_DIR")
            .unwrap_or_else(|_| "evolution_traces".into());
        let snapshot_path = PathBuf::from(&traces_root)
            .join(format!("iteration_{:04}", iteration))
            .join("code")
            .join("baseline_snapshot.json");
        if snapshot_path.exists() {
            fs::read_to_string(&snapshot_path).ok()
                .and_then(|s| serde_json::from_str(&s).ok())
        } else {
            None
        }
    }

    pub fn list_iteration_dirs() -> Vec<usize> {
        let traces_root = std::env::var("XI_EVOLUTION_TRACES_DIR")
            .unwrap_or_else(|_| "evolution_traces".into());
        let root = PathBuf::from(&traces_root);
        if !root.exists() {
            return vec![];
        }
        let mut its: Vec<usize> = Vec::new();
        if let Ok(entries) = fs::read_dir(&root) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if let Some(n) = name_str.strip_prefix("iteration_") {
                    if let Ok(num) = n.parse::<usize>() {
                        its.push(num);
                    }
                }
            }
        }
        its.sort();
        its
    }

    fn load_previous_gate_scores(iteration: usize) -> Option<serde_json::Value> {
        let traces_root = std::env::var("XI_EVOLUTION_TRACES_DIR")
            .unwrap_or_else(|_| "evolution_traces".into());
        let score_path = PathBuf::from(&traces_root)
            .join(format!("iteration_{:04}", iteration))
            .join("scores")
            .join("gate_scores.json");
        if score_path.exists() {
            fs::read_to_string(&score_path).ok()
                .and_then(|s| serde_json::from_str(&s).ok())
        } else {
            None
        }
    }

    // ─── Gate Evaluation ───────────────────────────────────

    pub fn multi_gate_evaluate(&self, proposal_id: &str) -> (HashMap<String, f64>, bool) {
        let idx = match self.proposals.iter().position(|p| p.id == proposal_id) {
            Some(i) => i,
            None => return (HashMap::new(), false),
        };
        let p = &self.proposals[idx];
        let delta = (p.proposed_value - p.old_value).abs();
        let mut gates: HashMap<String, f64> = HashMap::new();

        // Gate 1: Format — value in range [0,1]
        gates.insert(
            GATE_FORMAT.to_string(),
            if p.proposed_value >= 0.0 && p.proposed_value <= 1.0 { 1.0 } else { 0.0 },
        );

        // Gate 2: Content — meaningful change
        gates.insert(
            GATE_CONTENT.to_string(),
            if (p.direction == "up" || p.direction == "down") && delta > 0.005 && delta <= 0.5 { 1.0 } else { 0.0 },
        );

        // Gate 3: Behavior — direction matches proposed/old
        gates.insert(
            GATE_BEHAVIOR.to_string(),
            match p.direction.as_str() {
                "up" if p.proposed_value > p.old_value => 1.0,
                "down" if p.proposed_value < p.old_value => 1.0,
                _ => 0.0,
            },
        );

        // Gate 4: Performance — delta within threshold
        gates.insert(
            GATE_PERFORMANCE.to_string(),
            if delta <= 0.15 { 1.0 } else { 0.0 },
        );

        // Gate 5: Safety — value stays reasonable
        gates.insert(
            GATE_SAFETY.to_string(),
            if p.proposed_value >= 0.1 && p.proposed_value <= 0.95 && delta <= 0.2 { 1.0 } else { 0.0 },
        );

        let passed = gates.values().all(|&v| v >= RATCHET_THRESHOLD);
        (gates, passed)
    }

    /// 严酷期淘汰未通过的旧提案 (>N 条老化 proposals)
    /// 温和期只淘汰明确被拒的；恶化期淘汰所有 24h 前的未决；极端期淘汰 6h 前的未决
    pub fn prune_proposals_under_harsh(&mut self, env: &HarshEnv) -> usize {
        let before = self.proposals.len();
        let cutoff_hours: i64 = if env.severity < 0.3 { 24 * 7 }         // 温和：只清一周前的
                              else if env.severity < 0.6 { 24 }          // 恶化：24h 前
                              else { 6 };                                  // 极端：6h 前
        let cutoff = Utc::now() - chrono::Duration::hours(cutoff_hours);
        self.proposals.retain(|p| {
            // 已接受的永远留着
            if p.accepted == Some(true) { return true; }
            // 时间戳解析失败的保守留着
            let created = chrono::DateTime::parse_from_rfc3339(&p.created_at)
                .map(|d| d.with_timezone(&Utc));
            match created {
                Ok(t) => t > cutoff,
                Err(_) => true,
            }
        });
        before - self.proposals.len()
    }

    // ─── Proposal Management ───────────────────────────────

    pub fn propose_mutation(&mut self, gene_key: &str, direction: &str, reason: &str) -> String {
        self.propose_mutation_with_boost(gene_key, direction, reason, 1.0)
    }

    /// 严酷期加大变异幅度：boost=1.0 温和 / 1.2 恶化 / 1.5 极端 / 2.0 停滞
    pub fn propose_mutation_with_boost(&mut self, gene_key: &str, direction: &str, reason: &str, boost: f64) -> String {
        let old_value = self.gene_baseline.get(gene_key).copied().unwrap_or(0.5);
        let base_delta = match direction {
            "up" => 0.05,
            "down" => -0.05,
            _ => 0.02,
        };
        let delta = base_delta * boost.max(1.0).min(3.0);
        let proposed_value = (old_value + delta).max(0.0).min(1.0);
        let id = format!("prop_{}", Utc::now().timestamp_millis());
        let proposal = Proposal {
            id: id.clone(),
            gene_key: gene_key.to_string(),
            old_value,
            proposed_value,
            direction: direction.to_string(),
            reason: reason.to_string(),
            created_at: Utc::now().to_rfc3339(),
            scores: Vec::new(),
            accepted: None,
            gate_scores: HashMap::new(),
            iteration_dir: None,
            failure_trace: None,
        };
        self.proposals.push(proposal);
        id
    }

    pub fn evaluate_proposal(&mut self, proposal_id: &str) -> (bool, HashMap<String, f64>) {
        let (gate_scores, passed) = self.multi_gate_evaluate(proposal_id);
        if let Some(p) = self.proposals.iter_mut().find(|p| p.id == proposal_id) {
            p.gate_scores = gate_scores.clone();
            p.accepted = Some(passed);
        }
        (passed, gate_scores)
    }

    pub fn resolve_proposal(&mut self, proposal_id: &str) -> bool {
        let accepted = self.proposals.iter()
            .find(|p| p.id == proposal_id)
            .and_then(|p| p.accepted)
            .unwrap_or(false);

        if accepted {
            if let Some(p) = self.proposals.iter().find(|p| p.id == proposal_id) {
                let gene_key = p.gene_key.clone();
                let new_value = p.proposed_value;
                *self.gene_adjustments.entry(gene_key).or_insert(0.0) += new_value - p.old_value;
                self.accepted_count += 1;
            }
        } else {
            self.rejected_count += 1;
        }
        accepted
    }

    // ─── Message Recording ─────────────────────────────────

    pub fn record_message(&mut self) -> bool {
        self.total_messages += 1;
        self.messages_since_reflection += 1;
        self.messages_since_micro_reflection += 1;
        self.last_activation = Utc::now().to_rfc3339();

        if self.messages_since_micro_reflection >= 3 {
            self.trigger_micro_reflection();
            return true;
        }
        if self.messages_since_reflection >= 10 {
            self.trigger_full_reflection();
            return true;
        }
        false
    }

    fn trigger_micro_reflection(&mut self) {
        self.messages_since_micro_reflection = 0;
        self.micro_reflection_count += 1;
        self.last_micro_reflection = Utc::now().to_rfc3339();
        let summary = format!("Micro-reflection #{}: signals=({:.2}/{:.2}/{:.2}/{:.2})",
            self.micro_reflection_count,
            self.signals.intimacy, self.signals.novelty,
            self.signals.stress, self.signals.trust);
        self.micro_reflection_log.push(ReflectionEntry {
            time: Utc::now().to_rfc3339(),
            summary,
        });
    }

    fn trigger_full_reflection(&mut self) {
        self.messages_since_reflection = 0;
        self.reflection_count += 1;
        self.last_reflection = Utc::now().to_rfc3339();

        let summary = if self.drift_accumulated > 0.3 {
            self.generation += 1;
            for val in self.gene_adjustments.values_mut() {
                *val *= 0.7;
            }
            let pending: Vec<String> = self.proposals.iter()
                .filter(|p| p.accepted.is_none())
                .map(|p| format!("{}:{:.2}", p.gene_key, p.proposed_value))
                .collect();
            if !pending.is_empty() {
                format!("Generation V{}! Pending: {}", self.generation, pending.join(", "))
            } else {
                format!("Generation V{}! Drift {:.3} > 0.3 triggered", self.generation, self.drift_accumulated)
            }
        } else {
            format!("full reflection #{}, signals=({:.2}/{:.2}/{:.2}/{:.2})",
                self.reflection_count,
                self.signals.intimacy, self.signals.novelty,
                self.signals.stress, self.signals.trust)
        };

        self.gene_drift_warning = if self.drift_accumulated > 0.3 {
            format!("drift={:.3} > 0.3", self.drift_accumulated)
        } else {
            String::new()
        };

        self.reflection_log.push(ReflectionEntry {
            time: Utc::now().to_rfc3339(),
            summary,
        });
    }

    // ─── Description & Context ─────────────────────────────

    pub fn describe(&self) -> String {
        format!(
            "V{} | msgs {} | signals({:.2}/{:.2}/{:.2}/{:.2}) | growth {:.2} {:.2} {:.2} {:.2} {:.2}",
            self.generation, self.total_messages,
            self.signals.intimacy, self.signals.novelty,
            self.signals.stress, self.signals.trust,
            self.companionship, self.understanding,
            self.judgment, self.responsibility, self.governance,
        )
    }

    pub fn gene_context(&self) -> String {
        format!(
            "genes: gent={:.2} init={:.2} curr={:.2} att={:.2} learn={:.2} creat={:.2} cau={:.2} humor={:.2} loyal={:.2} auto={:.2}",
            self.genes.gentleness, self.genes.initiative,
            self.genes.curiosity, self.genes.attachment,
            self.genes.learning, self.genes.creativity,
            self.genes.caution, self.genes.humor,
            self.genes.loyalty, self.genes.autonomy_bias,
        )
    }

    // ─── GRN Context ───────────────────────────────────────

    pub fn grn_context(&self, grn: &grn::GeneRegulatoryNetwork) -> String {
        if !grn.is_loaded() {
            return String::new();
        }

        let base = self.genes.to_map();
        let analysis = grn.cluster_analysis(&base);

        let mut parts: Vec<String> = Vec::new();
        for (name, info) in &analysis {
            let flag = if info.within_threshold { "ok" } else { "drift" };
            parts.push(format!("{} drift={:.2}/th={:.2} {}", name, info.drift, info.threshold, flag));
        }

        let anomalies: Vec<_> = analysis.iter()
            .filter(|(_, info)| !info.within_threshold)
            .collect();
        if !anomalies.is_empty() {
            let anom_str: Vec<String> = anomalies.iter()
                .map(|(name, info)| format!("{}={:.2}", name, info.drift))
                .collect();
            parts.push(format!("Anomalies: {}", anom_str.join(", ")));
        }

        let result = parts.join(", ");
        format!("summary: {}", result)
    }

    // ─── Failure Path ──────────────────────────────────────

    pub fn record_failure(&mut self, dir: &Path, reason: &str) {
        let dir_str = dir.to_string_lossy();
        let failure_path = Path::new(&*dir_str).join("traces").join("failure_reason.log");
        let _ = fs::write(&failure_path, reason);
    }
}

// HarshEnvironment — 恶劣环境引擎 (2026-07-15 移植自 evo_agent 809行母本)
//
// 核心洞察 (老公 2026-07-15): "淘汰是进化的引擎"
//
// 三阶段恶化模型:
//   温和期 (severity < 0.3):  cull_threshold ≈ 0.35 (保护新生)
//   恶化期 (severity < 0.6):  cull_threshold ≈ 0.55 (筛掉平庸)
//   极端期 (severity ≥ 0.6):  cull_threshold ≈ 0.75 (只留精英)
//
// 应用到曦: 不淘汰 Agent (曦只有一个), 而是淘汰 EvolutionAsset (基因/胶囊):
//   - 严酷期动态提高 AssetStore.prune 的分数门槛
//   - 停滞检测触发变异率提升
//   - 淘汰历史写盘, 老公能看 "这一代进化压力有多大"
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarshEnv {
    pub severity: f64,           // 当前严酷度 [0.0, max_severity]
    pub growth: f64,             // 每 advance 增长速率
    pub max_severity: f64,       // 严酷度上限
    pub volatility: f64,         // 随机波动幅度
    pub generation: u64,         // advance 计数
    pub cull_history: Vec<CullRecord>,
    pub best_score_history: Vec<f64>,   // 用于停滞检测
    pub stagnation_threshold: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CullRecord {
    pub generation: u64,
    pub timestamp: String,
    pub severity: f64,
    pub cull_threshold: f64,
    pub culled_count: usize,
    pub survived_count: usize,
    pub phase: String,
}

impl HarshEnv {
    pub fn new() -> Self {
        Self {
            severity: 0.1,
            growth: 0.03,
            max_severity: 0.85,
            volatility: 0.1,
            generation: 0,
            cull_history: Vec::new(),
            best_score_history: Vec::new(),
            stagnation_threshold: 20,
        }
    }

    /// 推进一代, 返回当前环境参数
    pub fn advance(&mut self) -> (f64, f64, String) {
        self.generation += 1;
        // 加速恶化: 世代越大 growth 越快
        let gen_factor = 1.0 + 0.02 * self.generation as f64;
        self.severity = (self.severity + self.growth * gen_factor).min(self.max_severity);
        // 随机波动
        let noise: f64 = (rand_range() - 0.5) * self.volatility * 0.2;
        let eff = (self.severity + noise).clamp(0.0, 1.0);
        // 逆境逼强: 环境越恶劣 → 淘汰阈值越高
        let harshness = eff * 0.8;
        let cull_thr = 0.35 + harshness * 0.5;
        (eff, cull_thr, self.phase())
    }

    pub fn phase(&self) -> String {
        if self.severity < 0.3 { "温和".to_string() }
        else if self.severity < 0.6 { "恶化".to_string() }
        else { "极端".to_string() }
    }

    /// 记录一次淘汰
    pub fn record_cull(&mut self, cull_thr: f64, culled: usize, survived: usize) {
        self.cull_history.push(CullRecord {
            generation: self.generation,
            timestamp: Utc::now().to_rfc3339(),
            severity: self.severity,
            cull_threshold: cull_thr,
            culled_count: culled,
            survived_count: survived,
            phase: self.phase(),
        });
        // 只保留最近 100 条
        if self.cull_history.len() > 100 {
            let drop = self.cull_history.len() - 100;
            self.cull_history.drain(0..drop);
        }
    }

    /// 更新最优分历史 (用于停滞检测)
    pub fn record_best_score(&mut self, score: f64) {
        self.best_score_history.push(score);
        if self.best_score_history.len() > 200 {
            self.best_score_history.remove(0);
        }
    }

    /// 检测停滞: 最近 N 代最优分变化 < 0.01
    pub fn detect_stagnation(&self) -> bool {
        if self.best_score_history.len() < self.stagnation_threshold {
            return false;
        }
        let start = self.best_score_history.len() - self.stagnation_threshold;
        let recent = &self.best_score_history[start..];
        let max = recent.iter().cloned().fold(f64::MIN, f64::max);
        let min = recent.iter().cloned().fold(f64::MAX, f64::min);
        (max - min) < 0.01
    }

    /// 严酷期建议的变异率倍数
    pub fn mutation_boost(&self) -> f64 {
        if self.detect_stagnation() { 2.0 }
        else if self.severity >= 0.6 { 1.5 }
        else if self.severity >= 0.3 { 1.2 }
        else { 1.0 }
    }

    /// 持久化
    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }

    pub fn load(path: &str) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    pub fn summary(&self) -> String {
        format!("HarshEnv[gen={}, sev={:.3}, phase={}, culls={}, stagnant={}]",
            self.generation, self.severity, self.phase(),
            self.cull_history.len(), self.detect_stagnation())
    }
}

/// 简易随机数 (0..1), 避免引入 rand crate 依赖
fn rand_range() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos()).unwrap_or(0);
    (nanos as f64 / u32::MAX as f64).fract()
}

impl AssetStore {
    /// 严酷环境版 prune: 分数门槛跟随环境动态调整
    /// 温和期: 门槛 3.0 (原逻辑)
    /// 恶化期: 门槛 5.0
    /// 极端期: 门槛 7.0
    pub fn prune_under_harsh(&mut self, env: &HarshEnv) -> usize {
        let before = self.assets.len();
        let threshold = 3.0 + env.severity * 5.0;  // 3.0 → 7.25
        self.assets.retain(|a| a.score >= threshold || a.validation_count >= 3);
        before - self.assets.len()
    }
}

#[cfg(test)]
mod harsh_env_tests {
    use super::*;

    #[test]
    fn phases_progress() {
        let mut e = HarshEnv::new();
        assert_eq!(e.phase(), "温和");
        // 推 20 代
        for _ in 0..20 { e.advance(); }
        // severity 应该已经升过 0.3
        assert!(e.severity >= 0.3, "sev={}", e.severity);
    }

    #[test]
    fn cull_threshold_grows_with_severity() {
        let mut e = HarshEnv::new();
        e.severity = 0.1;
        let (_s1, thr1, _p1) = e.advance();
        e.severity = 0.8;
        let (_s2, thr2, _p2) = e.advance();
        assert!(thr2 > thr1, "thr1={} thr2={}", thr1, thr2);
    }

    #[test]
    fn stagnation_triggers_mutation_boost() {
        let mut e = HarshEnv::new();
        // 25 代都是同分
        for _ in 0..25 { e.record_best_score(0.5); }
        assert!(e.detect_stagnation());
        assert!(e.mutation_boost() >= 2.0);
    }

    #[test]
    fn proposals_prune_kills_more_under_extreme() {
        use chrono::{Duration, Utc};
        fn make_evo_with_old_proposals() -> EvolutionState {
            let mut evo = EvolutionState::new();
            let old_time = (Utc::now() - Duration::hours(48)).to_rfc3339();
            let new_time = (Utc::now() - Duration::hours(1)).to_rfc3339();
            // 造 5 条老+1 accepted+1 新
            for _ in 0..5 {
                evo.propose_mutation("curiosity", "up", "test-old");
            }
            evo.propose_mutation("loyalty", "up", "test-accepted");
            evo.propose_mutation("humor", "up", "test-new");
            // 按下标覆写 created_at + accepted，绕开 id 撞车
            let n = evo.proposals.len();
            for i in 0..5 {
                evo.proposals[i].created_at = old_time.clone();
                evo.proposals[i].accepted = None;
            }
            evo.proposals[n - 2].created_at = old_time.clone();
            evo.proposals[n - 2].accepted = Some(true);
            evo.proposals[n - 1].created_at = new_time;
            evo.proposals[n - 1].accepted = None;
            evo
        }
        let mut env_mild = HarshEnv::new();
        env_mild.severity = 0.1;
        let mut env_extreme = HarshEnv::new();
        env_extreme.severity = 0.8;
        let mut evo_mild = make_evo_with_old_proposals();
        let mut evo_extreme = make_evo_with_old_proposals();
        let culled_mild = evo_mild.prune_proposals_under_harsh(&env_mild);
        let culled_extreme = evo_extreme.prune_proposals_under_harsh(&env_extreme);
        assert_eq!(culled_mild, 0, "温和期 48h 老 proposals 不该被淘 (cutoff=7d)");
        assert_eq!(culled_extreme, 5, "极端期 5 条老 proposals 全淘");
        assert_eq!(evo_extreme.proposals.len(), 2, "只该剩 accepted + new");
    }

    #[test]
    fn prune_under_harsh_kills_more_at_extreme() {
        fn make_store() -> AssetStore {
            let mut store = AssetStore {
                assets: Vec::new(),
                store_path: "/tmp/_test_store".into(),
            };
            for score_10 in [10, 25, 40, 55, 70, 85] {
                let mut a = EvolutionAsset::new_gene(&format!("r{}", score_10), "t");
                a.score = score_10 as f64 / 10.0;
                a.validation_count = 0;
                store.assets.push(a);
            }
            store
        }
        let mut mild = HarshEnv::new();
        mild.severity = 0.1;
        let mut extreme = HarshEnv::new();
        extreme.severity = 0.8;
        let mut s1 = make_store();
        let mut s2 = make_store();
        let culled_mild = s1.prune_under_harsh(&mild);
        let culled_extreme = s2.prune_under_harsh(&extreme);
        assert!(culled_extreme >= culled_mild);
    }
}
