// evolution/assets.rs —— GEP 进化资产与资产库（2026-08-21 从 evolution.rs 拆出）
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::fs;

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
