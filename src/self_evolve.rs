// self_evolve.rs
// 融合 SkillHone (诊断-修改-评估) 与 OPD-Evolver (实时策略蒸馏)
// 目标：让 xi-system 具备自我迭代进化的能力

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// 诊断报告：识别当前系统或任务执行中的问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnosis {
    pub dimension: String, // 例如: 'code_quality', 'response_latency', 'user_satisfaction'
    pub severity: f32,     // 0.0 - 1.0, 越高越严重
    pub details: String,
    pub suggested_fix: Option<String>,
}

/// 修改指令：根据诊断结果生成的具体修改动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Modification {
    pub target: String,    // 被修改的对象，如 'config', 'module_x', 'parameter_y'
    pub action: String,    // 动作类型: 'update', 'remove', 'add', 'refactor'
    pub payload: HashMap<String, String>, // 具体修改内容
    pub rollback_plan: Option<String>, // 回滚计划，确保安全
}

/// 评估结果：评估修改后的效果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evaluation {
    pub improvement: f32,  // 改善程度，-1.0 (更差) 到 1.0 (更好)
    pub metrics_changed: Vec<String>, // 变化的指标
    pub success: bool,     // 是否达到预期
}

/// 蒸馏出的策略：从成功或失败的循环中提取的经验
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistilledStrategy {
    pub condition: String, // 触发条件
    pub action: String,    // 推荐动作
    pub confidence: f32,   // 置信度
    pub source_loop: u64,  // 来源循环ID
}

/// 自进化引擎核心
pub struct SelfEvolver {
    pub loop_count: u64,
    pub diagnosis_history: Vec<Diagnosis>,
    pub modification_log: Vec<Modification>,
    pub evaluation_log: Vec<Evaluation>,
    pub strategy_pool: Vec<DistilledStrategy>,
}

impl SelfEvolver {
    pub fn new() -> Self {
        Self {
            loop_count: 0,
            diagnosis_history: Vec::new(),
            modification_log: Vec::new(),
            evaluation_log: Vec::new(),
            strategy_pool: Vec::new(),
        }
    }

    /// 第一步：诊断
    pub fn diagnose(&mut self, current_state: &str) -> Diagnosis {
        // 模拟诊断逻辑，实际应连接监控系统或分析器
        let severity = if current_state.contains("error") { 0.9 } else { 0.3 };
        let diagnosis = Diagnosis {
            dimension: "system_health".to_string(),
            severity,
            details: format!("Detected state: {}", current_state),
            suggested_fix: Some("apply_standard_fix".to_string()),
        };
        self.diagnosis_history.push(diagnosis.clone());
        diagnosis
    }

    /// 第二步：修改
    pub fn modify(&mut self, diagnosis: &Diagnosis) -> Modification {
        // 根据诊断生成修改指令
        let modification = Modification {
            target: "config".to_string(),
            action: "update".to_string(),
            payload: HashMap::from([("fix_type".to_string(), diagnosis.suggested_fix.clone().unwrap_or_default())]),
            rollback_plan: Some("revert_to_last_known_good".to_string()),
        };
        self.modification_log.push(modification.clone());
        modification
    }

    /// 第三步：评估
    pub fn evaluate(&mut self, modification: &Modification) -> Evaluation {
        // 模拟评估，实际应运行测试或监控指标
        let improvement = 0.7; // 假设改善明显
        let evaluation = Evaluation {
            improvement,
            metrics_changed: vec!["health_score".to_string()],
            success: improvement > 0.5,
        };
        self.evaluation_log.push(evaluation.clone());
        evaluation
    }

    /// 第四步：策略蒸馏 (OPD-Evolver 核心)
    pub fn distill_strategy(&mut self, diagnosis: &Diagnosis, evaluation: &Evaluation) {
        if evaluation.success && evaluation.improvement > 0.5 {
            let strategy = DistilledStrategy {
                condition: format!("severity > {}", diagnosis.severity),
                action: "apply_standard_fix".to_string(),
                confidence: evaluation.improvement,
                source_loop: self.loop_count,
            };
            self.strategy_pool.push(strategy);
        } else if !evaluation.success {
            // 记录失败案例，降低相关策略的置信度或删除
            println!("Warning: Evaluation failed. Strategy not distilled. Loop: {}", self.loop_count);
        }
    }

    /// 执行一次完整的自进化循环
    pub fn evolve_cycle(&mut self, current_state: &str) {
        self.loop_count += 1;
        println!("Starting evolution cycle #{}", self.loop_count);

        let diagnosis = self.diagnose(current_state);
        println!("Diagnosis: {:?}", diagnosis);

        let modification = self.modify(&diagnosis);
        println!("Modification: {:?}", modification);

        let evaluation = self.evaluate(&modification);
        println!("Evaluation: {:?}", evaluation);

        self.distill_strategy(&diagnosis, &evaluation);
        println!("Strategy pool size: {}", self.strategy_pool.len());

        println!("Cycle #{} completed.", self.loop_count);
    }

    /// 获取当前最优策略
    pub fn get_best_strategy(&self) -> Option<&DistilledStrategy> {
        self.strategy_pool.iter().max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evolution_cycle() {
        let mut evolver = SelfEvolver::new();
        evolver.evolve_cycle("system_error_detected");
        assert_eq!(evolver.loop_count, 1);
        assert!(!evolver.diagnosis_history.is_empty());
        assert!(!evolver.modification_log.is_empty());
        assert!(!evolver.evaluation_log.is_empty());
    }

    #[test]
    fn test_strategy_distillation() {
        let mut evolver = SelfEvolver::new();
        evolver.evolve_cycle("healthy_system");
        // 假设 healthy_system 导致低严重性诊断，可能不会蒸馏出新策略
        // 这里主要测试结构完整性
        assert_eq!(evolver.loop_count, 1);
    }
}