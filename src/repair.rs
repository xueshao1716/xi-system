/// AgentCo-op Repair Engine — error classification and retry logic

use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType {
    ToolError,
    SchemaMismatch,
    Timeout,
    BudgetExceeded,
    ValidationFailed,
    Unknown,
}

impl ErrorType {
    pub fn from_message(msg: &str) -> Self {
        let lower = msg.to_lowercase();
        if lower.contains("timeout") { Self::Timeout }
        else if lower.contains("schema") || lower.contains("format") || lower.contains("missing") { Self::SchemaMismatch }
        else if lower.contains("budget") || lower.contains("token") || lower.contains("limit") { Self::BudgetExceeded }
        else if lower.contains("validation") || lower.contains("verify") { Self::ValidationFailed }
        else if lower.contains("tool") || lower.contains("exec") || lower.contains("api") || lower.contains("exit code") { Self::ToolError }
        else { Self::Unknown }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::ToolError => "tool_error",
            Self::SchemaMismatch => "schema_mismatch",
            Self::Timeout => "timeout",
            Self::BudgetExceeded => "budget_exceeded",
            Self::ValidationFailed => "validation_failed",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RepairAction {
    RetrySame,
    RetryWithUpdatedPrompt,
    SwapComponent,
    AddPreprocessor,
    AdjustParams,
}

impl RepairAction {
    pub fn as_str(&self) -> &str {
        match self {
            Self::RetrySame => "retry_same",
            Self::RetryWithUpdatedPrompt => "retry_updated_prompt",
            Self::SwapComponent => "swap_component",
            Self::AddPreprocessor => "add_preprocessor",
            Self::AdjustParams => "adjust_params",
        }
    }
}

pub struct RepairPolicy {
    error_type: ErrorType,
    action: RepairAction,
    priority: u32,
    description: &'static str,
}

fn default_policies() -> Vec<RepairPolicy> {
    vec![
        RepairPolicy { error_type: ErrorType::ToolError, action: RepairAction::RetrySame, priority: 10, description: "retry same tool" },
        RepairPolicy { error_type: ErrorType::ToolError, action: RepairAction::SwapComponent, priority: 20, description: "swap tool" },
        RepairPolicy { error_type: ErrorType::SchemaMismatch, action: RepairAction::AdjustParams, priority: 10, description: "fix schema" },
        RepairPolicy { error_type: ErrorType::SchemaMismatch, action: RepairAction::AddPreprocessor, priority: 20, description: "add preprocessor" },
        RepairPolicy { error_type: ErrorType::Timeout, action: RepairAction::RetrySame, priority: 10, description: "retry timeout" },
        RepairPolicy { error_type: ErrorType::Timeout, action: RepairAction::AdjustParams, priority: 20, description: "adjust timeout" },
        RepairPolicy { error_type: ErrorType::BudgetExceeded, action: RepairAction::SwapComponent, priority: 10, description: "swap cheaper" },
        RepairPolicy { error_type: ErrorType::ValidationFailed, action: RepairAction::RetryWithUpdatedPrompt, priority: 10, description: "fix prompt" },
        RepairPolicy { error_type: ErrorType::ValidationFailed, action: RepairAction::SwapComponent, priority: 20, description: "swap prompt" },
        RepairPolicy { error_type: ErrorType::Unknown, action: RepairAction::RetrySame, priority: 5, description: "retry unknown" },
    ]
}

#[derive(Debug, Clone)]
pub struct ExecutionTrace {
    pub trace_id: String,
    pub node_id: String,
    pub tool_name: String,
    pub input_hash: String,
    pub status: String,
    pub output_hash: Option<String>,
    pub error_type: Option<String>,
    pub error_message: Option<String>,
    pub token_cost: u64,
    pub duration_ms: u64,
    pub retry_count: u32,
    pub timestamp: u64,
}

impl ExecutionTrace {
    pub fn new(node_id: &str, tool_name: &str, input_preview: &str) -> Self {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let input_hash = simple_hash(input_preview);
        let trace_id = format!("{}_{:x}", &input_hash[..8.min(input_hash.len())], ts);
        Self {
            trace_id,
            node_id: node_id.into(),
            tool_name: tool_name.into(),
            input_hash,
            status: "running".into(),
            output_hash: None,
            error_type: None,
            error_message: None,
            token_cost: 0,
            duration_ms: 0,
            retry_count: 0,
            timestamp: ts,
        }
    }

    pub fn mark_success(&mut self, output_preview: &str, token_cost: u64, duration_ms: u64) {
        self.status = "success".into();
        self.output_hash = Some(simple_hash(output_preview));
        self.token_cost = token_cost;
        self.duration_ms = duration_ms;
    }

    pub fn mark_failed(&mut self, error_msg: &str) {
        self.status = "failed".into();
        self.error_type = Some(ErrorType::from_message(error_msg).as_str().into());
        self.error_message = Some(error_msg.into());
    }

    pub fn to_value(&self) -> Value {
        json!({
            "trace_id": self.trace_id,
            "node_id": self.node_id,
            "tool_name": self.tool_name,
            "status": self.status,
            "error_type": self.error_type,
            "retry_count": self.retry_count,
            "duration_ms": self.duration_ms,
        })
    }
}

pub struct RepairEngine {
    policies: Vec<RepairPolicy>,
    traces: Mutex<Vec<ExecutionTrace>>,
    max_retries: u32,
    max_repair_budget: u64,
    total_repair_cost: Mutex<u64>,
}

impl RepairEngine {
    pub fn new(max_retries: u32) -> Self {
        Self {
            policies: default_policies(),
            traces: Mutex::new(Vec::new()),
            max_retries,
            max_repair_budget: 50000,
            total_repair_cost: Mutex::new(0),
        }
    }

    pub fn create_trace(&self, node_id: &str, tool_name: &str, input_preview: &str) -> ExecutionTrace {
        ExecutionTrace::new(node_id, tool_name, input_preview)
    }

    pub fn record_trace(&self, trace: ExecutionTrace) {
        if let Ok(mut traces) = self.traces.lock() {
            traces.push(trace);
            let len = traces.len();
            if len > 200 {
                traces.drain(0..len - 200);
            }
        }
    }

    fn check_stop(&self, retry_count: u32) -> Option<String> {
        if retry_count >= self.max_retries {
            return Some(format!("max_retries ({}/{})", retry_count, self.max_retries));
        }
        if let Ok(cost) = self.total_repair_cost.lock() {
            if *cost >= self.max_repair_budget {
                return Some(format!("budget_exceeded ({}/{})", cost, self.max_repair_budget));
            }
        }
        None
    }

    pub fn match_policy(&self, error_type: &ErrorType, retry_count: u32) -> Option<&RepairPolicy> {
        let candidates: Vec<&RepairPolicy> = self.policies.iter()
            .filter(|p| p.error_type == *error_type)
            .collect();
        for policy in candidates {
            if retry_count == 0 && policy.priority <= 10 {
                return Some(policy);
            }
            if retry_count == 1 && policy.priority > 10 && policy.priority <= 20 {
                return Some(policy);
            }
        }
        self.policies.iter().find(|p| p.error_type == *error_type)
    }

    pub fn attempt_repair(&self, trace: &mut ExecutionTrace) -> Value {
        if let Some(reason) = self.check_stop(trace.retry_count) {
            return json!({ "should_retry": false, "stop_reason": reason });
        }
        let error_type = trace.error_type.as_deref()
            .map(ErrorType::from_message)
            .unwrap_or(ErrorType::Unknown);
        let policy = self.match_policy(&error_type, trace.retry_count);
        trace.retry_count += 1;
        if let Some(p) = policy {
            json!({
                "should_retry": true,
                "action": p.action.as_str(),
                "policy": p.description,
                "error_type": error_type.as_str(),
                "retry_count": trace.retry_count,
            })
        } else {
            json!({ "should_retry": false, "stop_reason": "no_matching_policy" })
        }
    }

    pub fn stats(&self) -> Value {
        self.traces.lock().map(|traces| {
            let total = traces.len();
            let failed = traces.iter().filter(|t| t.status == "failed").count();
            let repaired = traces.iter().filter(|t| t.status == "success" && t.retry_count > 0).count();
            let avg_retries: f64 = if total > 0 {
                traces.iter().map(|t| t.retry_count as f64).sum::<f64>() / total as f64
            } else { 0.0 };
            json!({
                "total_traces": total,
                "failed": failed,
                "repaired": repaired,
                "repair_rate": if failed > 0 { (repaired as f64 / failed as f64 * 100.0).round() } else { 0.0 },
                "avg_retries": (avg_retries * 100.0).round() / 100.0,
            })
        }).unwrap_or(json!({}))
    }

    pub fn recent_traces(&self, limit: usize) -> Vec<Value> {
        self.traces.lock().map(|traces| {
            traces.iter().rev().take(limit).map(|t| t.to_value()).collect()
        }).unwrap_or_default()
    }
}

fn simple_hash(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
