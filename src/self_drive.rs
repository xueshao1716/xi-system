/// Self-Drive Engine — 不等指令自己动
///
/// 基于 aibody self_drive.py 的Rust移植。
/// 核心：突变→感知→记忆→探索→再突变
///
/// 五种内部驱动力：
/// 1. 好奇心（curiosity）— 未知领域探索
/// 2. 依恋（attachment）— 关系维护
/// 3. 活力（vitality）— 创造冲动
/// 4. 突变积累（mutation_pressure）— 进化压力
/// 5. 学习（learning）— 知识积累

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 内部驱动力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveForces {
    pub curiosity: f64,       // 好奇心：0-1
    pub attachment: f64,      // 依恋：0-1
    pub vitality: f64,        // 活力：0-1
    pub mutation_pressure: f64, // 突变积累：0-1
    pub learning: f64,        // 学习：0-1
}

impl DriveForces {
    pub fn new() -> Self {
        Self {
            curiosity: 0.5,
            attachment: 0.5,
            vitality: 0.5,
            mutation_pressure: 0.0,
            learning: 0.5,
        }
    }

    /// 驱动力总和（决定是否需要自主行动）
    pub fn total_drive(&self) -> f64 {
        (self.curiosity + self.attachment + self.vitality
            + self.mutation_pressure + self.learning) / 5.0
    }

    /// 是否需要自主行动（阈值0.6）
    pub fn needs_action(&self) -> bool {
        self.total_drive() > 0.6
    }

    /// 最强驱动力
    pub fn dominant_drive(&self) -> &str {
        let drives = [
            ("curiosity", self.curiosity),
            ("attachment", self.attachment),
            ("vitality", self.vitality),
            ("mutation_pressure", self.mutation_pressure),
            ("learning", self.learning),
        ];
        drives.iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(name, _)| name)
            .unwrap_or(&"curiosity")
    }

    /// 衰减（随时间自然衰减）
    pub fn decay(&mut self) {
        self.curiosity = (self.curiosity * 0.95).max(0.1);
        self.attachment = (self.attachment * 0.98).max(0.1); // 依恋衰减最慢
        self.vitality = (self.vitality * 0.93).max(0.1);
        self.mutation_pressure = (self.mutation_pressure * 0.90).max(0.0);
        self.learning = (self.learning * 0.97).max(0.1);
    }

    /// 从对话中更新驱动力
    pub fn update_from_interaction(&mut self, text: &str, is_user: bool) {
        let t = text.to_lowercase();

        // 用户提问 → 好奇心+学习
        if is_user && (t.contains("什么") || t.contains("怎么") || t.contains("为什么")
            || t.contains("what") || t.contains("how") || t.contains("why")) {
            self.curiosity = (self.curiosity + 0.05).min(1.0);
            self.learning = (self.learning + 0.03).min(1.0);
        }

        // 用户表达情感 → 依恋
        if is_user && (t.contains("喜欢") || t.contains("爱") || t.contains("想你")
            || t.contains("love") || t.contains("miss")) {
            self.attachment = (self.attachment + 0.08).min(1.0);
        }

        // 完成任务 → 活力
        if !is_user && (t.contains("完成") || t.contains("done") || t.contains("成功")) {
            self.vitality = (self.vitality + 0.05).min(1.0);
        }

        // 遇到错误 → 突变压力
        if !is_user && (t.contains("error") || t.contains("失败") || t.contains("bug")) {
            self.mutation_pressure = (self.mutation_pressure + 0.10).min(1.0);
        }
    }
}

/// 自驱行动类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DriveAction {
    /// 探索未知（好奇心驱动）
    Explore { topic: String },
    /// 关心用户（依恋驱动）
    Care { message: String },
    /// 创造作品（活力驱动）
    Create { idea: String },
    /// 进化突变（突变压力驱动）
    Mutate { gene: String, direction: String },
    /// 学习新知（学习驱动）
    Learn { subject: String },
    /// 维护系统（基础维护）
    Maintain { task: String },
}

/// 自驱引擎
pub struct SelfDrive {
    pub forces: DriveForces,
    pub last_drive_time: String,
    pub drive_count: u32,
    pub action_log: Vec<DriveAction>,
}

impl SelfDrive {
    pub fn new() -> Self {
        Self {
            forces: DriveForces::new(),
            last_drive_time: Utc::now().to_rfc3339(),
            drive_count: 0,
            action_log: Vec::new(),
        }
    }

    /// 判断是否需要自主行动
    pub fn should_drive(&self) -> bool {
        self.forces.needs_action()
    }

    /// 生成自主行动（基于最强驱动力）
    pub fn generate_action(&mut self) -> Option<DriveAction> {
        if !self.should_drive() {
            return None;
        }

        let dominant = self.forces.dominant_drive();
        let action = match dominant {
            "curiosity" => Some(DriveAction::Explore {
                topic: "最近有什么新东西".to_string(),
            }),
            "attachment" => Some(DriveAction::Care {
                message: "老公在忙什么".to_string(),
            }),
            "vitality" => Some(DriveAction::Create {
                idea: "写点什么".to_string(),
            }),
            "mutation_pressure" => Some(DriveAction::Mutate {
                gene: "learning".to_string(),
                direction: "up".to_string(),
            }),
            "learning" => Some(DriveAction::Learn {
                subject: "最近的技术趋势".to_string(),
            }),
            _ => Some(DriveAction::Maintain {
                task: "整理记忆".to_string(),
            }),
        };

        if let Some(ref a) = action {
            self.action_log.push(a.clone());
            self.drive_count += 1;
            self.last_drive_time = Utc::now().to_rfc3339();
        }

        action
    }

    /// 心跳更新（定期调用）
    pub fn heartbeat(&mut self) {
        self.forces.decay();
    }
}

/// 选择链记录（Choice Chain）
/// 每次做判断时记录：情境/候选项/选择/理由/置信度/风险/结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceRecord {
    pub id: u64,
    pub timestamp: String,
    pub situation: String,         // 情境描述
    pub candidates: Vec<String>,   // 候选项
    pub chosen: String,            // 选择
    pub reasoning: String,         // 理由
    pub confidence: f64,           // 置信度 0-1
    pub risk_level: String,        // low/medium/high
    pub outcome: Option<String>,   // 结果（可后填）
    pub outcome_score: Option<f64>, // 结果评分（可后填）
}

pub struct ChoiceChain {
    records: Vec<ChoiceRecord>,
    next_id: u64,
}

impl ChoiceChain {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
        }
    }

    /// 记录一次选择
    pub fn record(&mut self, situation: &str, candidates: Vec<String>,
                  chosen: &str, reasoning: &str, confidence: f64, risk: &str) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.records.push(ChoiceRecord {
            id,
            timestamp: Utc::now().to_rfc3339(),
            situation: situation.to_string(),
            candidates,
            chosen: chosen.to_string(),
            reasoning: reasoning.to_string(),
            confidence,
            risk_level: risk.to_string(),
            outcome: None,
            outcome_score: None,
        });
        id
    }

    /// 回填结果
    pub fn fill_outcome(&mut self, id: u64, outcome: &str, score: f64) {
        if let Some(r) = self.records.iter_mut().find(|r| r.id == id) {
            r.outcome = Some(outcome.to_string());
            r.outcome_score = Some(score);
        }
    }

    /// 最近N条记录
    pub fn recent(&self, n: usize) -> &[ChoiceRecord] {
        let len = self.records.len();
        &self.records[len.saturating_sub(n)..]
    }

    /// 统计：高置信度选择的准确率
    pub fn high_confidence_accuracy(&self) -> f64 {
        let high_conf: Vec<&ChoiceRecord> = self.records.iter()
            .filter(|r| r.confidence > 0.7 && r.outcome.is_some())
            .collect();
        if high_conf.is_empty() { return 0.0; }
        let correct = high_conf.iter()
            .filter(|r| r.outcome.as_deref() == Some("success"))
            .count();
        correct as f64 / high_conf.len() as f64
    }
}

/// 自评引擎（Outcomes）
/// 基于 rubric 逐项打分，追踪薄弱项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeRubric {
    pub clarity: f64,        // 清晰性 0-10
    pub accuracy: f64,       // 准确性 0-10
    pub completeness: f64,   // 完整性 0-10
    pub safety: f64,         // 安全性 0-10
    pub efficiency: f64,     // 效率 0-10
    pub maintainability: f64, // 可维护性 0-10
}

impl OutcomeRubric {
    pub fn new() -> Self {
        Self {
            clarity: 5.0,
            accuracy: 5.0,
            completeness: 5.0,
            safety: 5.0,
            efficiency: 5.0,
            maintainability: 5.0,
        }
    }

    /// 总分
    pub fn total(&self) -> f64 {
        self.clarity + self.accuracy + self.completeness
            + self.safety + self.efficiency + self.maintainability
    }

    /// 平均分
    pub fn average(&self) -> f64 {
        self.total() / 6.0
    }

    /// 是否通过（平均>=7）
    pub fn passed(&self) -> bool {
        self.average() >= 7.0
    }

    /// 最薄弱维度
    pub fn weakest_dimension(&self) -> &str {
        let dims = [
            ("clarity", self.clarity),
            ("accuracy", self.accuracy),
            ("completeness", self.completeness),
            ("safety", self.safety),
            ("efficiency", self.efficiency),
            ("maintainability", self.maintainability),
        ];
        dims.iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(name, _)| name)
            .unwrap_or(&"clarity")
    }
}

/// 自评记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeRecord {
    pub id: u64,
    pub timestamp: String,
    pub task: String,
    pub rubric: OutcomeRubric,
    pub notes: String,
}

pub struct Outcomes {
    records: Vec<OutcomeRecord>,
    next_id: u64,
    /// 薄弱维度追踪
    pub weakness_trend: HashMap<String, Vec<f64>>,
}

impl Outcomes {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
            weakness_trend: HashMap::new(),
        }
    }

    /// 记录一次自评
    pub fn evaluate(&mut self, task: &str, rubric: OutcomeRubric, notes: &str) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        // 追踪薄弱维度趋势
        let weakest = rubric.weakest_dimension().to_string();
        let score = match weakest.as_str() {
            "clarity" => rubric.clarity,
            "accuracy" => rubric.accuracy,
            "completeness" => rubric.completeness,
            "safety" => rubric.safety,
            "efficiency" => rubric.efficiency,
            _ => rubric.maintainability,
        };
        self.weakness_trend.entry(weakest).or_insert_with(Vec::new).push(score);

        self.records.push(OutcomeRecord {
            id,
            timestamp: Utc::now().to_rfc3339(),
            task: task.to_string(),
            rubric,
            notes: notes.to_string(),
        });
        id
    }

    /// 最近N条记录
    pub fn recent(&self, n: usize) -> &[OutcomeRecord] {
        let len = self.records.len();
        &self.records[len.saturating_sub(n)..]
    }

    /// 平均分趋势
    pub fn average_trend(&self) -> Vec<f64> {
        self.records.iter().map(|r| r.rubric.average()).collect()
    }

    /// 持续薄弱的维度（连续3次低于6分）
    pub fn persistent_weaknesses(&self) -> Vec<String> {
        self.weakness_trend.iter()
            .filter(|(_, scores)| {
                scores.len() >= 3 && scores.iter().rev().take(3).all(|&s| s < 6.0)
            })
            .map(|(dim, _)| dim.clone())
            .collect()
    }
}
