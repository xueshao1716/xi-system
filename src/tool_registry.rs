// 工具注册表模块
// 定义工具的结构和注册机制

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// 工具参数的定义
#[derive(Debug, Clone)]
pub struct ToolParam {
    pub name: String,
    pub description: String,
    pub param_type: String, // e.g., "string", "number", "object"
    pub required: bool,
}

/// 工具的定义
#[derive(Debug, Clone)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub params: Vec<ToolParam>,
    // 在实际实现中，这里会是一个函数指针或异步闭包
    // 为了简化，我们先只存储元数据
}

/// 工具注册表
pub struct ToolRegistry {
    tools: Arc<Mutex<HashMap<String, ToolDef>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        ToolRegistry {
            tools: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 注册一个新工具
    pub fn register(&self, tool: ToolDef) -> Result<(), String> {
        let mut tools = self.tools.lock().map_err(|e| e.to_string())?;
        if tools.contains_key(&tool.name) {
            return Err(format!("Tool {} already exists", tool.name));
        }
        tools.insert(tool.name.clone(), tool);
        Ok(())
    }

    /// 获取工具定义
    pub fn get(&self, name: &str) -> Option<ToolDef> {
        let tools = self.tools.lock().ok()?;
        tools.get(name).cloned()
    }

    /// 列出所有工具
    pub fn list(&self) -> Vec<String> {
        let tools = self.tools.lock().ok()?;
        tools.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get_tool() {
        let registry = ToolRegistry::new();
        let tool = ToolDef {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            params: vec![],
        };

        assert!(registry.register(tool).is_ok());
        assert!(registry.get("test_tool").is_some());
        assert!(registry.get("non_existent").is_none());
    }

    #[test]
    fn test_duplicate_registration() {
        let registry = ToolRegistry::new();
        let tool = ToolDef {
            name: "dup_tool".to_string(),
            description: "Duplicate tool".to_string(),
            params: vec![],
        };

        assert!(registry.register(tool.clone()).is_ok());
        assert!(registry.register(tool).is_err());
    }

    #[test]
    fn test_list_tools() {
        let registry = ToolRegistry::new();
        let tool1 = ToolDef { name: "tool1".into(), description: "1".into(), params: vec![] };
        let tool2 = ToolDef { name: "tool2".into(), description: "2".into(), params: vec![] };

        registry.register(tool1).unwrap();
        registry.register(tool2).unwrap();

        let mut tools = registry.list();
        tools.sort();
        assert_eq!(tools, vec!["tool1", "tool2"]);
    }
}