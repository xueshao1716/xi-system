/// AgentCo-op Broker — typed artifact validation and handoff

use std::collections::HashMap;
use std::sync::Mutex;
use serde_json::{json, Value};

fn builtin_schemas() -> HashMap<String, Value> {
    let mut m = HashMap::new();
    m.insert("task_handoff".into(), json!({
        "required": ["task_id", "source", "target", "task_type", "payload"],
        "optional": ["context", "constraints"]
    }));
    m.insert("code_review".into(), json!({
        "required": ["file_path", "issues", "verdict"],
        "optional": ["suggestions"]
    }));
    m.insert("feed_absorb".into(), json!({
        "required": ["url", "title", "judgment", "one_liner"],
        "optional": ["key_insights", "system_relevance"]
    }));
    m.insert("code_artifact".into(), json!({
        "required": ["file_path", "action", "language", "content_hash"],
        "optional": ["content", "dependencies"]
    }));
    m.insert("deploy_report".into(), json!({
        "required": ["service", "status", "action_taken"],
        "optional": ["logs", "rollback_plan"]
    }));
    m.insert("memory_sync".into(), json!({
        "required": ["sync_type", "entries"],
        "optional": ["conflict_resolution"]
    }));
    m.insert("execution_trace".into(), json!({
        "required": ["trace_id", "node_id", "status", "input_hash"],
        "optional": ["output_hash", "error", "token_cost", "duration_ms", "retry_count"]
    }));
    m
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub data: Value,
}

impl ValidationResult {
    pub fn ok(data: Value) -> Self {
        Self { valid: true, errors: vec![], warnings: vec![], data }
    }
    pub fn fail(errors: Vec<String>) -> Self {
        Self { valid: false, errors, warnings: vec![], data: Value::Null }
    }
    pub fn summary(&self) -> String {
        if self.valid {
            "valid".into()
        } else {
            format!("{} errors: {}", self.errors.len(), self.errors.join("; "))
        }
    }
}

pub struct Broker {
    schemas: HashMap<String, Value>,
    trace_log: Mutex<Vec<Value>>,
}

impl Broker {
    pub fn new() -> Self {
        Self {
            schemas: builtin_schemas(),
            trace_log: Mutex::new(Vec::new()),
        }
    }

    pub fn validate(&self, artifact_type: &str, data: &Value) -> ValidationResult {
        let schema = match self.schemas.get(artifact_type) {
            Some(s) => s,
            None => return ValidationResult::fail(vec![format!("Unknown type: {}", artifact_type)]),
        };
        let mut errors = Vec::new();
        if let Some(required) = schema["required"].as_array() {
            for field in required {
                if let Some(name) = field.as_str() {
                    if data.get(name).is_none() || data[name].is_null() {
                        errors.push(format!("Missing: {}", name));
                    }
                }
            }
        }
        if errors.is_empty() {
            ValidationResult::ok(data.clone())
        } else {
            ValidationResult::fail(errors)
        }
    }

    pub fn handoff(&self, artifact_type: &str, data: Value, source: &str, target: &str) -> ValidationResult {
        let result = self.validate(artifact_type, &data);
        let trace = json!({
            "timestamp": chrono_now(),
            "artifact_type": artifact_type,
            "source": source,
            "target": target,
            "valid": result.valid,
            "errors": result.errors,
        });
        if let Ok(mut log) = self.trace_log.lock() {
            log.push(trace);
            let len = log.len();
            if len > 100 {
                log.drain(0..len - 100);
            }
        }
        result
    }

    pub fn recent_traces(&self, limit: usize) -> Vec<Value> {
        self.trace_log.lock().map(|log| {
            log.iter().rev().take(limit).cloned().collect()
        }).unwrap_or_default()
    }

    pub fn stats(&self) -> Value {
        self.trace_log.lock().map(|log| {
            let total = log.len();
            let success = log.iter().filter(|t| t["valid"].as_bool() == Some(true)).count();
            json!({
                "total": total,
                "success": success,
                "failed": total - success,
                "success_rate": if total > 0 { (success as f64 / total as f64 * 100.0).round() } else { 0.0 },
            })
        }).unwrap_or(json!({}))
    }
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    format!("{}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        1970 + (secs / 31536000) as u32,
        ((secs % 31536000) / 2592000 % 12) + 1,
        ((secs % 2592000) / 86400) + 1,
        (secs % 86400) / 3600,
        (secs % 3600) / 60,
        secs % 60,
    )
}
