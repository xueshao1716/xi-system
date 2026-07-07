/// Self-Harness: 让 Agent 根据失败轨迹改造自己的 harness
///
/// 基于论文: "Self-Harness: Let Agents Modify Their Own Operating System"
/// https://arxiv.org/abs/2606.09498
///
/// 核心循环: 弱点挖掘 → Harness提议 → 回归验证
/// 不改模型权重，只改执行协议（prompt/工具/验证规则/运行策略）

use std::collections::HashMap;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailurePattern {
    pub id: String,
    pub failure_type: String,
    pub count: usize,
    pub root_cause: String,
    pub suggested_surface: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessProposal {
    pub id: String,
    pub target_pattern: String,
    pub surface: String,
    pub change: String,
    pub expected_effect: String,
    pub regression_risk: String,
    pub held_in_delta: Option<f64>,
    pub held_out_delta: Option<f64>,
    pub accepted: Option<bool>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionResult {
    pub proposal_id: String,
    pub held_in_pass_rate: f64,
    pub held_out_pass_rate: f64,
    pub passed_gate: bool,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfHarness {
    pub patterns: Vec<FailurePattern>,
    pub proposals: Vec<HarnessProposal>,
    pub harness_version: usize,
    pub regression_results: Vec<RegressionResult>,
}

impl SelfHarness {
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
            proposals: Vec::new(),
            harness_version: 1,
            regression_results: Vec::new(),
        }
    }

    pub fn mine_weaknesses(&self, traces: &[String]) -> Vec<FailurePattern> {
        let mut patterns: HashMap<String, FailurePattern> = HashMap::new();

        for trace in traces {
            let lower = trace.to_lowercase();
            let ft;
            let rc;
            let sv;

            if lower.contains("no such file") || lower.contains("file not found") || lower.contains("missing") {
                ft = "missing_artifact";
                rc = "忘记创建必要的输出文件";
                sv = "system_prompt";
            } else if lower.contains("timeout") || lower.contains("loop") || lower.contains("stuck") {
                ft = "tool_loop";
                rc = "工具调用陷入死循环";
                sv = "runtime_policy";
            } else if lower.contains("environment") || lower.contains("env") || lower.contains("path") {
                ft = "env_lost";
                rc = "跨shell会话丢失环境变量";
                sv = "runtime_policy";
            } else if lower.contains("wrong") || lower.contains("incorrect") || lower.contains("mismatch") {
                ft = "wrong_output";
                rc = "输出格式或内容不符合要求";
                sv = "tool_config";
            } else {
                continue;
            }

            let key = ft.to_string();
            let entry = patterns.entry(key).or_insert_with(|| FailurePattern {
                id: format!("pat_{}", ft),
                failure_type: ft.to_string(),
                count: 0,
                root_cause: rc.to_string(),
                suggested_surface: sv.to_string(),
            });
            entry.count += 1;
        }

        let mut result: Vec<FailurePattern> = patterns.into_values().collect();
        result.sort_by(|a, b| b.count.cmp(&a.count));
        result.retain(|p| p.count >= 2);
        result
    }

    pub fn propose_harness_change(&mut self, pattern: &FailurePattern) -> String {
        let (change, expected, risk) = match pattern.failure_type.as_str() {
            "missing_artifact" => (
                "在system_prompt中加入：执行任务后必须检查所有必需输出文件是否存在".into(),
                "减少因忘记创建文件导致的失败".into(),
                "可能增加不必要的文件创建步骤".into(),
            ),
            "tool_loop" => (
                "在runtime_policy中加入：同一工具连续调用超过3次必须换方法或报告".into(),
                "打破工具死循环".into(),
                "可能在需要重试的场景下过早放弃".into(),
            ),
            "env_lost" => (
                "在runtime_policy中加入：跨shell命令时显式export关键环境变量".into(),
                "保持环境状态一致性".into(),
                "增加命令复杂度".into(),
            ),
            "wrong_output" => (
                "在tool_config中加入：输出前对照需求文档检查格式".into(),
                "提高输出准确率".into(),
                "可能增加检查时间".into(),
            ),
            _ => return String::new(),
        };

        let proposal = HarnessProposal {
            id: format!("hp_{}", Utc::now().timestamp_millis()),
            target_pattern: pattern.id.clone(),
            surface: pattern.suggested_surface.clone(),
            change,
            expected_effect: expected,
            regression_risk: risk,
            held_in_delta: None,
            held_out_delta: None,
            accepted: None,
            created_at: Utc::now().to_rfc3339(),
        };

        let id = proposal.id.clone();
        self.proposals.push(proposal);
        id
    }

    pub fn validate_proposal(
        &mut self,
        proposal_id: &str,
        held_in_before: f64,
        held_in_after: f64,
        held_out_before: f64,
        held_out_after: f64,
    ) -> (bool, f64, f64) {
        let held_in_delta = held_in_after - held_in_before;
        let held_out_delta = held_out_after - held_out_before;

        let passed_gate = held_in_delta >= 0.0
            && held_out_delta >= 0.0
            && (held_in_delta > 0.0 || held_out_delta > 0.0);

        let result = RegressionResult {
            proposal_id: proposal_id.to_string(),
            held_in_pass_rate: held_in_after,
            held_out_pass_rate: held_out_after,
            passed_gate,
            timestamp: Utc::now().to_rfc3339(),
        };
        self.regression_results.push(result);

        if let Some(p) = self.proposals.iter_mut().find(|p| p.id == proposal_id) {
            p.held_in_delta = Some(held_in_delta);
            p.held_out_delta = Some(held_out_delta);
            p.accepted = Some(passed_gate);
        }

        (passed_gate, held_in_delta, held_out_delta)
    }

    pub fn apply_accepted(&mut self) -> usize {
        let accepted_count = self.proposals.iter().filter(|p| p.accepted == Some(true)).count();
        if accepted_count > 0 {
            self.harness_version += 1;
        }
        self.harness_version
    }

    pub fn status_summary(&self) -> String {
        let total = self.proposals.len();
        let accepted = self.proposals.iter().filter(|p| p.accepted == Some(true)).count();
        let rejected = self.proposals.iter().filter(|p| p.accepted == Some(false)).count();
        let pending = total - accepted - rejected;

        format!(
            "Self-Harness v{}: {}模式, {}提案({}接受/{}拒绝/{}待验), {}次回归",
            self.harness_version, self.patterns.len(), total, accepted, rejected, pending, self.regression_results.len()
        )
    }

    pub fn save(&self, path: &str) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }

    pub fn load(path: &str) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(Self::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mine_weaknesses() {
        let engine = SelfHarness::new();
        let traces = vec![
            "Error: file not found: output.json".into(),
            "Error: file not found: config.yaml".into(),
            "Timeout after 30s".into(),
        ];
        let patterns = engine.mine_weaknesses(&traces);
        assert!(!patterns.is_empty());
        assert_eq!(patterns[0].failure_type, "missing_artifact");
        assert_eq!(patterns[0].count, 2);
    }

    #[test]
    fn test_propose_and_validate() {
        let mut engine = SelfHarness::new();
        let pattern = FailurePattern {
            id: "pat_test".into(),
            failure_type: "tool_loop".into(),
            count: 3,
            root_cause: "工具死循环".into(),
            suggested_surface: "runtime_policy".into(),
        };
        let proposal_id = engine.propose_harness_change(&pattern);
        assert!(!proposal_id.is_empty());

        let (passed, hi_delta, ho_delta) = engine.validate_proposal(&proposal_id, 0.5, 0.6, 0.4, 0.45);
        assert!(passed);
        assert!(hi_delta > 0.0);
        assert!(ho_delta > 0.0);
    }

    #[test]
    fn test_regression_gate() {
        let mut engine = SelfHarness::new();
        let pattern = FailurePattern {
            id: "pat_test2".into(),
            failure_type: "wrong_output".into(),
            count: 2,
            root_cause: "输出错误".into(),
            suggested_surface: "tool_config".into(),
        };
        let proposal_id = engine.propose_harness_change(&pattern);

        let (passed, _, _) = engine.validate_proposal(&proposal_id, 0.5, 0.6, 0.5, 0.45);
        assert!(!passed);
    }
}
