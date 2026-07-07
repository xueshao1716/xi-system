/// Evolution State — Gene Expression + Signals + Darwin Loop + Gate Evaluation
///
/// Signals: 4 dimensions (intimacy/novelty/stress/trust) with 10 decay rates
/// Genes: 10 personality traits expressed from signals
/// Darwin Loop: propose mutations, gate evaluation, iteration tracking
/// v3: Meta-Harness — iteration dirs, 5 gate AND evaluation, trace filesystem

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use crate::grn;

// ─── Signals ───────────────────────────────────────────────

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

    fn express(signals: &Signals) -> Self {
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

    fn to_map(&self) -> HashMap<String, f64> {
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

// ─── GEP Evolution Assets (Evolver pattern, 2026-06-24) ────
// Based on Evolver's GEP: Gene/Capsule as evolution assets, not just memory.
// Gene = compact core evolution unit (heritable, mutable, verifiable)
// Capsule = full context evolution asset (audit trail included)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetType {
    Gene,     // Compact: rule + trigger + evidence
    Capsule,  // Full context: gene + history + variants
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionAsset {
    pub id: String,
    pub asset_type: AssetType,
    pub rule: String,           // The generalized rule/pattern
    pub trigger: String,        // When to apply this asset
    pub evidence: Vec<String>,  // Supporting evidence (conversations, errors)
    pub score: f64,             // Quality score (0-10)
    pub created_at: String,
    pub last_validated: String,
    pub validation_count: u32,
    pub parent_id: Option<String>,  // For mutation tracking
}

impl EvolutionAsset {
    pub fn new_gene(rule: &str, trigger: &str) -> Self {
        Self {
            id: format!("gene_{}", Utc::now().timestamp_millis()),
            asset_type: AssetType::Gene,
            rule: rule.to_string(),
            trigger: trigger.to_string(),
            evidence: Vec::new(),
            score: 5.0,
            created_at: Utc::now().to_rfc3339(),
            last_validated: Utc::now().to_rfc3339(),
            validation_count: 0,
            parent_id: None,
        }
    }
}

// ─── Five-Dimension Evolution Evaluation (Evolver GEP) ─────
// 五维度评估：自动化/泛化/验证/固化/迭代闭环
// 只有五维度全通过才算"真进化"

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiveDimensionEval {
    pub automation: bool,      // 无需人工干预完成进化循环
    pub generalization: bool,  // 能从具体经验提炼通用规则
    pub verification: bool,    // 系统性验证进化结果有效性
    pub solidification: bool,  // 沉淀成可重用资产
    pub iteration_loop: bool,  // 进化结果还能被继续优化
}

impl FiveDimensionEval {
    pub fn new() -> Self {
        Self {
            automation: false,
            generalization: false,
            verification: false,
            solidification: false,
            iteration_loop: false,
        }
    }

    /// 五维度全通过才算真进化
    pub fn is_real_evolution(&self) -> bool {
        self.automation && self.generalization && self.verification
            && self.solidification && self.iteration_loop
    }

    /// 通过的维度数
    pub fn passed_count(&self) -> u8 {
        [self.automation, self.generalization, self.verification,
         self.solidification, self.iteration_loop]
            .iter().filter(|&&x| x).count() as u8
    }

    /// 评估等级
    pub fn grade(&self) -> &'static str {
        let n = self.passed_count();
        if n >= 5 { "真进化" }
        else if n >= 3 { "半进化" }
        else { "假进化" }
    }
}

// ─── GEP Pipeline: SCAN → MUTATE → VALIDATE → SOLIDIFY ────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GepStep {
    pub name: String,        // SCAN/MUTATE/VALIDATE/SOLIDIFY
    pub status: String,      // pending/running/done/failed
    pub input: String,
    pub output: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GepPipeline {
    pub steps: Vec<GepStep>,
    pub candidate: Option<EvolutionAsset>,
    pub evaluation: FiveDimensionEval,
    pub accepted: bool,
}

impl GepPipeline {
    pub fn new() -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            steps: vec![
                GepStep { name: "SCAN".into(), status: "pending".into(), input: String::new(), output: String::new(), timestamp: now.clone() },
                GepStep { name: "MUTATE".into(), status: "pending".into(), input: String::new(), output: String::new(), timestamp: now.clone() },
                GepStep { name: "VALIDATE".into(), status: "pending".into(), input: String::new(), output: String::new(), timestamp: now.clone() },
                GepStep { name: "SOLIDIFY".into(), status: "pending".into(), input: String::new(), output: String::new(), timestamp: now },
            ],
            candidate: None,
            evaluation: FiveDimensionEval::new(),
            accepted: false,
        }
    }

    /// 当前步骤
    pub fn current_step(&self) -> Option<&GepStep> {
        self.steps.iter().find(|s| s.status == "pending")
    }

    /// 标记步骤完成
    pub fn complete_step(&mut self, output: &str) {
        if let Some(step) = self.steps.iter_mut().find(|s| s.status == "pending") {
            step.status = "done".to_string();
            step.output = output.to_string();
        }
    }
}

// ─── EvolutionAsset Persistence ───────────────────────────
// Gene/Capsule 资产的持久化存储和检索

impl EvolutionAsset {
    /// 保存资产到JSON文件
    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }

    /// 从JSON文件加载资产
    pub fn load(path: &str) -> std::io::Result<Self> {
        let data = std::fs::read_to_string(path)?;
        serde_json::from_str(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    /// 添加证据
    pub fn add_evidence(&mut self, evidence: &str) {
        if !self.evidence.contains(&evidence.to_string()) {
            self.evidence.push(evidence.to_string());
        }
    }

    /// 验证后更新分数和时间
    pub fn validated(&mut self, score: f64) {
        self.score = score;
        self.validation_count += 1;
        self.last_validated = Utc::now().to_rfc3339();
    }

    /// 变异：从当前资产生成新变体
    pub fn mutate(&self, new_rule: &str, new_trigger: &str) -> Self {
        let mut child = self.clone();
        child.id = format!("gene_{}", Utc::now().timestamp_millis());
        child.rule = new_rule.to_string();
        child.trigger = new_trigger.to_string();
        child.parent_id = Some(self.id.clone());
        child.score = 5.0; // 新变体从5分开始
        child.created_at = Utc::now().to_rfc3339();
        child.validation_count = 0;
        child
    }
}

// ─── Asset Store — 资产仓库 ──────────────────────────────
// 管理所有进化资产的存储、检索和淘汰

pub struct AssetStore {
    pub assets: Vec<EvolutionAsset>,
    pub store_path: String,
}

impl AssetStore {
    pub fn new(store_path: &str) -> Self {
        let mut store = Self {
            assets: Vec::new(),
            store_path: store_path.to_string(),
        };
        store.load_all();
        store
    }

    /// 从目录加载所有资产
    fn load_all(&mut self) {
        let path = std::path::Path::new(&self.store_path);
        if !path.exists() {
            let _ = std::fs::create_dir_all(path);
            return;
        }
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(asset) = EvolutionAsset::load(p.to_str().unwrap_or("")) {
                        self.assets.push(asset);
                    }
                }
            }
        }
    }

    /// 添加新资产
    pub fn add(&mut self, asset: EvolutionAsset) {
        let path = format!("{}/{}.json", self.store_path, asset.id);
        let _ = asset.save(&path);
        self.assets.push(asset);
    }

    /// 按触发条件检索资产
    pub fn search_by_trigger(&self, context: &str) -> Vec<&EvolutionAsset> {
        self.assets.iter()
            .filter(|a| context.contains(&a.trigger) || a.trigger.contains(context))
            .collect()
    }

    /// 按分数排序，返回Top N
    pub fn top_n(&self, n: usize) -> Vec<&EvolutionAsset> {
        let mut sorted: Vec<&EvolutionAsset> = self.assets.iter().collect();
        sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        sorted.into_iter().take(n).collect()
    }

    /// 淘汰低分资产（分数 < 3.0 且验证次数 < 2）
    pub fn prune(&mut self) -> usize {
        let before = self.assets.len();
        self.assets.retain(|a| a.score >= 3.0 || a.validation_count >= 2);
        before - self.assets.len()
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

    // ─── Proposal Management ───────────────────────────────

    pub fn propose_mutation(&mut self, gene_key: &str, direction: &str, reason: &str) -> String {
        let old_value = self.gene_baseline.get(gene_key).copied().unwrap_or(0.5);
        let delta = match direction {
            "up" => 0.05,
            "down" => -0.05,
            _ => 0.02,
        };
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
