// working_memory.rs — 工作记忆（从旧版 python working_memory.py 移植 2026-08-20）
//
// 核心模式：
//   - 新对话 → 保留 conversation_history，只清任务上下文
//   - 每步执行 → update context / 记录 tool_calls
//   - 任务完成 → 归档到长期记忆（L2），清 working 但保留 history
//
// 这是"人"的工作记忆：不是永久存储，是当前任务的"桌面"。
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub role: String, // user / assistant
    pub content: String,
    pub ts: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub tool: String,
    pub args: String,
    pub result_summary: String,
    pub ok: bool,
    pub ts: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkingMemory {
    pub task: String,
    pub user_id: String,
    pub started_at: String,
    pub intent: String,
    pub context: HashMap<String, serde_json::Value>,
    pub tool_calls: Vec<ToolCallRecord>,
    pub intermediate: HashMap<String, serde_json::Value>,
    pub conversation_history: Vec<ConversationTurn>,
    pub paused: bool,
    pub n_turns: u64,
    pub completed: bool,
}

impl WorkingMemory {
    pub fn new() -> Self {
        WorkingMemory {
            started_at: Utc::now().to_rfc3339(),
            ..Default::default()
        }
    }

    pub fn load(path: &str) -> Self {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(wm) = serde_json::from_str::<WorkingMemory>(&content) {
                return wm;
            }
        }
        WorkingMemory::new()
    }

    pub fn save(&self, path: &str) {
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }

    /// 新任务：保留对话历史，只清任务上下文
    pub fn start_task(&mut self, task: &str, user_id: &str, intent: &str) {
        let history = std::mem::take(&mut self.conversation_history);
        *self = WorkingMemory {
            task: task.to_string(),
            user_id: user_id.to_string(),
            started_at: Utc::now().to_rfc3339(),
            intent: intent.to_string(),
            conversation_history: history,
            n_turns: 0,
            ..Default::default()
        };
    }

    pub fn append_turn(&mut self, role: &str, content: &str) {
        self.conversation_history.push(ConversationTurn {
            role: role.to_string(),
            content: content.to_string(),
            ts: Utc::now().to_rfc3339(),
        });
        self.n_turns += 1;
    }

    pub fn record_tool(&mut self, tool: &str, args: &str, result_summary: &str, ok: bool) {
        self.tool_calls.push(ToolCallRecord {
            tool: tool.to_string(),
            args: args.to_string(),
            result_summary: result_summary.to_string(),
            ok,
            ts: Utc::now().to_rfc3339(),
        });
        // 只留最近 50 条工具记录（防止无界增长）
        if self.tool_calls.len() > 50 {
            let excess = self.tool_calls.len() - 50;
            self.tool_calls.drain(0..excess);
        }
    }

    pub fn set_context(&mut self, key: &str, value: serde_json::Value) {
        self.context.insert(key.to_string(), value);
    }

    /// 任务完成：归档标记（长期记忆由调用方写入 memory.rs）
    pub fn complete(&mut self) {
        self.completed = true;
    }

    /// 工作记忆摘要（进 system prompt，让模型感知当前任务状态）
    pub fn injection(&self) -> String {
        let mut parts = Vec::new();
        if !self.task.is_empty() {
            parts.push(format!("当前任务: {}", self.task));
            if !self.intent.is_empty() {
                parts.push(format!("意图: {}", self.intent));
            }
        }
        if !self.tool_calls.is_empty() {
            let last = self.tool_calls.last().unwrap();
            parts.push(format!(
                "最近工具: {}({}) → {}{}",
                last.tool,
                last.args.chars().take(40).collect::<String>(),
                if last.ok { "✓ " } else { "✗ " },
                last.result_summary.chars().take(60).collect::<String>(),
            ));
        }
        if self.n_turns > 0 {
            parts.push(format!("本轮 {} 轮对话", self.n_turns));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("【工作记忆】\n{}", parts.join("\n"))
        }
    }

    /// 最近对话（供 memory.rs 归档/检索）
    pub fn recent_history(&self, n: usize) -> Vec<ConversationTurn> {
        let skip = self.conversation_history.len().saturating_sub(n);
        self.conversation_history[skip..].to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_task_keeps_history() {
        let mut wm = WorkingMemory::new();
        wm.append_turn("user", "你好");
        wm.start_task("写报告", "老公", "文档");
        assert_eq!(wm.conversation_history.len(), 1); // 历史保留
        assert_eq!(wm.n_turns, 0); // 新任务计数清零
        assert_eq!(wm.task, "写报告");
    }

    #[test]
    fn tool_records_bounded() {
        let mut wm = WorkingMemory::new();
        for i in 0..60 {
            wm.record_tool("bash", &format!("cmd {}", i), "ok", true);
        }
        assert!(wm.tool_calls.len() <= 50);
        assert_eq!(wm.tool_calls.last().unwrap().args, "cmd 59");
    }

    #[test]
    fn save_load_roundtrip() {
        let p = std::env::temp_dir().join("wm_test.json");
        let p = p.to_str().unwrap();
        let mut wm = WorkingMemory::new();
        wm.start_task("任务A", "老公", "测试");
        wm.append_turn("user", "做它");
        wm.record_tool("read", "a.md", "读到内容", true);
        wm.save(p);
        let loaded = WorkingMemory::load(p);
        assert_eq!(loaded.task, "任务A");
        assert_eq!(loaded.conversation_history.len(), 1);
        assert_eq!(loaded.tool_calls.len(), 1);
        let _ = fs::remove_file(p);
    }

    #[test]
    fn injection_shape() {
        let mut wm = WorkingMemory::new();
        assert!(wm.injection().is_empty());
        wm.start_task("修 bug", "老公", "");
        assert!(wm.injection().contains("修 bug"));
    }
}
