// tool_forge.rs — 造工具能力（2026-08-20）
//
// 让曦能自己定义新工具：把"工具"作为一等公民，可创建/注册/执行。
// 工具定义存 tools/custom/*.json（JSON），create_tool 元工具负责写入。
//
// 工具实现三种形态：
//   exec       shell 命令模板（{args} 占位展开）——最通用
//   composite  组合现有基础工具（按序调用 read_file/write_file/exec 等）
//   prompt     提示词模板工具（返回展开后的文本，供 agent 继续用）
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTool {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub parameters: serde_json::Value, // OpenAI function schema 的 parameters
    pub handler: ToolHandler,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub created_by: String, // 谁造的（曦/用户/系统）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolHandler {
    /// shell 模板：如 "python {script_path}" / "node {file}"，用 args 替换 {key}
    Exec { template: String, timeout_secs: Option<u64> },
    /// 组合：按序调用基础工具，args 透传
    Composite { steps: Vec<CompositeStep> },
    /// 提示词模板：返回展开文本
    Prompt { template: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeStep {
    pub tool: String, // 基础工具名（exec/read_file/write_file/...）
    pub args: HashMap<String, serde_json::Value>,
}

impl CustomTool {
    pub fn schema(&self) -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters,
            }
        })
    }
}

/// 造一个新工具（返回工具对象，由调用方保存）
pub fn forge_tool(
    name: &str,
    description: &str,
    parameters: serde_json::Value,
    handler: ToolHandler,
    created_by: &str,
) -> CustomTool {
    CustomTool {
        name: name.to_string(),
        description: description.to_string(),
        parameters,
        handler,
        created_at: Utc::now().to_rfc3339(),
        created_by: created_by.to_string(),
    }
}

/// 保存工具定义到 tools/custom/
pub fn save_tool(dir: &str, tool: &CustomTool) -> Result<(), String> {
    // 名字合法性：字母/数字/下划线（工具名会被 agent 直接调用）
    if !tool.name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(format!("工具名非法（只允许字母数字下划线）: {}", tool.name));
    }
    if tool.name.len() > 32 {
        return Err("工具名过长（≤32 字符）".to_string());
    }
    fs::create_dir_all(dir).map_err(|e| format!("创建工具目录失败: {}", e))?;
    let path = Path::new(dir).join(format!("{}.json", tool.name));
    let json = serde_json::to_string_pretty(tool).map_err(|e| format!("序列化失败: {}", e))?;
    fs::write(path, json).map_err(|e| format!("写入失败: {}", e))?;
    Ok(())
}

/// 加载全部自定义工具
pub fn load_custom_tools(dir: &str) -> Vec<CustomTool> {
    let mut tools = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else { return tools };
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("json") { continue; }
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(t) = serde_json::from_str::<CustomTool>(&content) {
                tools.push(t);
            } else {
                eprintln!("[tool_forge] 工具定义解析失败: {}", p.display());
            }
        }
    }
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    tools
}

/// 执行自定义工具
/// base_call: 基础工具调用器（tools::call_tool），composite 用它组合
// 单线程 runtime 用：base_call 不要求 Send（call_tool 内部 future 非 Send，agent_loop 是单线程 tokio）
pub async fn execute_custom(
    tool: &CustomTool,
    args: &serde_json::Value,
    base_call: &(dyn Fn(&str, &serde_json::Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = String>>>),
) -> String {
    match &tool.handler {
        ToolHandler::Exec { template, timeout_secs } => {
            // {key} 占位展开
            let mut cmd = template.clone();
            if let Some(obj) = args.as_object() {
                for (k, v) in obj {
                    let val = match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    cmd = cmd.replace(&format!("{{{}}}", k), &val);
                }
            }
            // 执行（复用 tools::call_tool 的 exec）
            let exec_args = json!({ "command": cmd });
            let _ = timeout_secs;
            base_call("exec", &exec_args).await
        }
        ToolHandler::Composite { steps } => {
            let mut out = Vec::new();
            for step in steps {
                let result = base_call(&step.tool, &json!(step.args)).await;
                out.push(format!("[{}.{}] {}", tool.name, step.tool, result));
            }
            out.join("\n")
        }
        ToolHandler::Prompt { template } => {
            let mut text = template.clone();
            if let Some(obj) = args.as_object() {
                for (k, v) in obj {
                    let val = match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    text = text.replace(&format!("{{{}}}", k), &val);
                }
            }
            text
        }
    }
}

/// 自定义工具的工具表（追加进 agent 的 tool_definitions）
pub fn custom_definitions(tools: &[CustomTool]) -> Vec<serde_json::Value> {
    tools.iter().map(|t| t.schema()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forge_and_save_load() {
        let dir = std::env::temp_dir().join("tool_forge_test");
        let dir = dir.to_str().unwrap();
        let _ = fs::remove_dir_all(dir);
        let tool = forge_tool(
            "greet",
            "问候用户",
            json!({"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}),
            ToolHandler::Prompt { template: "你好，{name}！".to_string() },
            "xi",
        );
        save_tool(dir, &tool).unwrap();
        let loaded = load_custom_tools(dir);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "greet");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn invalid_name_rejected() {
        let dir = std::env::temp_dir().join("tool_forge_test2");
        let dir = dir.to_str().unwrap();
        let tool = forge_tool("bad name!", "x", json!({}), ToolHandler::Prompt { template: "".into() }, "xi");
        assert!(save_tool(dir, &tool).is_err());
    }

    #[test]
    fn exec_template_expands() {
        // 用 fake base_call 验证展开逻辑
        let tool = forge_tool(
            "run_py",
            "跑 python",
            json!({}),
            ToolHandler::Exec { template: "python {script}".to_string(), timeout_secs: None },
            "xi",
        );
        let result = tokio_test_block_on(async {
            execute_custom(&tool, &json!({"script": "test.py"}), &|name, args| {
                let name = name.to_string();
                let args = args.clone();
                Box::pin(async move { format!("CALLED {} {:?}", name, args) })
            }).await
        });
        assert!(result.contains("python test.py"));
    }

    #[test]
    fn prompt_tool_returns_text() {
        let tool = forge_tool("t1", "d", json!({}), ToolHandler::Prompt { template: "分析 {topic}".into() }, "xi");
        let result = tokio_test_block_on(async {
            execute_custom(&tool, &json!({"topic": "股市"}), &|_, _| Box::pin(async { String::new() })).await
        });
        assert_eq!(result, "分析 股市");
    }

    // 简化 block_on（避免引入 tokio-test 依赖）
    fn tokio_test_block_on<F: std::future::Future>(f: F) -> F::Output {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(f)
    }
}
