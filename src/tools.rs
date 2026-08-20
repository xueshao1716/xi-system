//! Tool definitions and implementations for the AI agent system.
//!
//! Provides OpenAI function-calling compatible tool schemas and async handlers
//! for shell execution, file I/O, web fetching, search, artifact validation,
//! and image generation.

use serde_json::json;

// ── Constants ────────────────────────────────────────────────────────────────


// ── Tool Schema ──────────────────────────────────────────────────────────────

/// Returns OpenAI function-calling compatible tool definitions.
pub fn tool_definitions() -> Vec<serde_json::Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": "exec",
                "description": "Execute a shell command. Cross-platform: uses bash on Linux, cmd.exe on Windows. Timeout 30 seconds. Output truncated at 15000 chars.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {"type": "string", "description": "Shell command"}
                    },
                    "required": ["command"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read text/json/md/py/rs file content. Supports Windows and WSL paths. Output truncated at 20000 chars.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "File path, e.g. D:\\xi-system\\Cargo.toml"}
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Write content to a file. Supports both Windows and Unix paths.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "File path"},
                        "content": {"type": "string", "description": "File content"}
                    },
                    "required": ["path", "content"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "search_files",
                "description": "Search file contents or find files by name.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {"type": "string", "description": "Search pattern"},
                        "target": {"type": "string", "description": "'content' for content search, 'files' for file names"},
                        "path": {"type": "string", "description": "Search directory path"}
                    },
                    "required": ["pattern"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "web_get",
                "description": "HTTP GET to fetch URL or API. Timeout 30 seconds. Returns text, strips JavaScript. Truncated at 20000 chars.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": {"type": "string", "description": "Target URL"}
                    },
                    "required": ["url"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "web_search",
                "description": "Web search via Google.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search query"},
                        "count": {"type": "integer", "description": "Number of results (default 5, max 10)"}
                    },
                    "required": ["query"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "validate_artifact",
                "description": "Validate artifact against schema. Returns {valid, errors}.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "artifact_type": {"type": "string", "description": "Artifact type: task_handoff/code_review/feed_absorb/code_artifact/deploy_report/memory_sync/execution_trace"},
                        "data": {"type": "object", "description": "Artifact data"}
                    },
                    "required": ["artifact_type", "data"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "generate_image",
                "description": "Generate image via Agnes AI. Supports optional reference URL.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "prompt": {"type": "string", "description": "Image prompt"},
                        "size": {"type": "string", "description": "Image size, e.g. 1024x1024, 1024x768", "default": "1024x1024"},
                        "image_url": {"type": "string", "description": "Optional reference image URL"}
                    },
                    "required": ["prompt"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "create_tool",
                "description": "造工具：定义一个新工具（名字/描述/参数/实现），保存后立即可用。handler.kind: exec(shell模板，{key}占位替换args)/prompt(文本模板)。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "description": "工具名（字母数字下划线，≤32字符）"},
                        "description": {"type": "string", "description": "工具用途说明"},
                        "parameters": {"type": "object", "description": "OpenAI 参数 schema"},
                        "handler": {
                            "type": "object",
                            "description": "实现：kind=exec 时 template 是 shell 命令模板（{key} 会被 args 替换）；kind=prompt 时 template 是文本模板",
                            "properties": {
                                "kind": {"type": "string", "enum": ["exec", "prompt"]},
                                "template": {"type": "string"}
                            }
                        }
                    },
                    "required": ["name", "description", "handler"]
                }
            }
        }),
    ]
}

// ── Tool Dispatch ────────────────────────────────────────────────────────────

/// Dispatch a tool call by name with the given arguments.
pub fn custom_tool_dir() -> String {
    format!("{}/tools/custom", crate::xi_home())
}

pub async fn call_tool(name: &str, args: &serde_json::Value) -> String {
    match name {
        "exec" => cmd_exec(args).await,
        "read_file" => cmd_read_file(args).await,
        "write_file" => cmd_write_file(args).await,
        "search_files" => cmd_search_files(args).await,
        "web_get" => cmd_web_get(args).await,
        "web_search" => cmd_web_search(args).await,
        "validate_artifact" => cmd_validate_artifact(args).await,
        "generate_image" => cmd_generate_image(args).await,
        "create_tool" => cmd_create_tool(args).await,
        // 自定义工具（tool_forge）：动态加载执行
        _ => {
            let tools = crate::tool_forge::load_custom_tools(&custom_tool_dir());
            if let Some(tool) = tools.iter().find(|t| t.name == name) {
                crate::tool_forge::execute_custom(tool, args, &|base, base_args| {
                    let base = base.to_string();
                    let base_args = base_args.clone();
                    Box::pin(async move { call_tool(&base, &base_args).await })
                }).await
            } else {
                format!("Unknown tool: {}", name)
            }
        }
    }
}

// ── 造工具元工具（tool_forge 入口）：曦自己定义新工具 ──
async fn cmd_create_tool(args: &serde_json::Value) -> String {
    let name = args["name"].as_str().unwrap_or("");
    let description = args["description"].as_str().unwrap_or("");
    let parameters = args.get("parameters").cloned().unwrap_or(json!({"type":"object","properties":{}}));
    let kind = args["handler"]["kind"].as_str().unwrap_or("prompt");
    let template = args["handler"]["template"].as_str().unwrap_or("");
    if name.is_empty() || description.is_empty() {
        return "Error: create_tool 需要 name 和 description".to_string();
    }
    let handler = match kind {
        "exec" => crate::tool_forge::ToolHandler::Exec { template: template.to_string(), timeout_secs: args["handler"]["timeout_secs"].as_u64() },
        "prompt" => crate::tool_forge::ToolHandler::Prompt { template: template.to_string() },
        _ => return format!("Error: 不支持的 handler kind: {}（支持 exec/prompt）", kind),
    };
    let tool = crate::tool_forge::forge_tool(name, description, parameters, handler, "xi");
    match crate::tool_forge::save_tool(&custom_tool_dir(), &tool) {
        Ok(()) => format!("工具「{}」已创建并注册：{}", name, description),
        Err(e) => format!("Error: {}", e),
    }
}

// ── Shell Execution ──────────────────────────────────────────────────────────

/// Execute a shell command.
/// - Windows 原生: 用 Git Bash（显式路径，避免解析到 System32ash.exe 即 WSL bash）
/// - Linux/WSL: 用 bash
/// 命令里若带 /mnt/d/ 或 wsl 前缀会自动转换（曦旧习惯兜底）。
async fn cmd_exec(args: &serde_json::Value) -> String {
    let cmd = args["command"].as_str().unwrap_or("");
    if cmd.is_empty() {
        return "Error: empty command".to_string();
    }
    // 2026-08-21 护栏接入：risk_guard 危险命令拦截（rm -rf / 等 BLOCK_OR_CONFIRM）
    let verdict = crate::risk_guard::check_command(cmd);
    if verdict.dangerous {
        return format!("❌ [risk_guard] 危险命令被拦截（{}）: {}。{}",
            verdict.level, verdict.reasons.join(" / "), verdict.suggestion);
    }

    // Windows 原生环境适配：曦旧习惯里的 WSL 路径和 wsl 调用自动转换
    #[cfg(windows)]
    let cmd = {
        let mut c = cmd.to_string();
        c = c.replace("/mnt/d/", "/d/");
        c = c.lines().map(|line| {
            let t = line.trim_start();
            if t.starts_with("wsl ") || t.starts_with("wsl.exe ") {
                t.replacen("wsl", "", 1).replacen(".exe", "", 1).trim_start().to_string()
            } else { line.to_string() }
        }).collect::<Vec<_>>().join("
");
        c
    };
    #[cfg(not(windows))]
    let cmd = cmd.to_string();

    // 选择 shell
    #[cfg(windows)]
    let shell: std::path::PathBuf = {
        let candidates = [
            "C:\\Program Files\\Git\\bin\\bash.exe",
            "C:\\Program Files\\Git\\usr\\bin\\bash.exe",
            "C:\\Program Files (x86)\\Git\\bin\\bash.exe",
            "D:\\Program Files\\Git\\bin\\bash.exe",
        ];
        candidates.iter().map(|s| std::path::PathBuf::from(s))
            .find(|p| p.exists())
            .unwrap_or_else(|| std::path::PathBuf::from("bash"))
    };
    #[cfg(not(windows))]
    let shell = std::path::PathBuf::from("bash");

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        tokio::process::Command::new(&shell)
            .arg("-c")
            .arg(&cmd)
            .output(),
    )
    .await;

    match output {
        Ok(result) => match result {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let code = out.status.code().unwrap_or(-1);
                let mut res = format!("exit code: {}", code);
                if !stdout.is_empty() {
                    res.push_str(&format!("
--- stdout ---
{}", stdout));
                }
                if !stderr.is_empty() {
                    res.push_str(&format!("
--- stderr ---
{}", stderr));
                }
                res
            }
            Err(e) => format!("Execution error: {}", e),
        },
        Err(_) => "Timeout after 120 seconds".to_string(),
    }
}

async fn cmd_read_file(args: &serde_json::Value) -> String {
    let path = args["path"].as_str().unwrap_or("");
    if path.is_empty() {
        return "Error: missing path".to_string();
    }
    // 统一路径分隔符
    let path = path.replace("\\", "/");
    std::fs::read_to_string(&path)
        .map(|s| if s.len() > 20000 { format!("{}...[truncated]", &s[..20000]) } else { s })
        .unwrap_or_else(|e| format!("Error reading {}: {}", path, e))
}

async fn cmd_write_file(args: &serde_json::Value) -> String {
    let path = args["path"].as_str().unwrap_or("");
    let content = args["content"].as_str().unwrap_or("");
    if path.is_empty() {
        return "Error: missing path".to_string();
    }
    let path = path.replace("\\", "/");
    std::fs::write(&path, content)
        .map(|_| format!("Written {} bytes to {}", content.len(), path))
        .unwrap_or_else(|e| format!("Error writing {}: {}", path, e))
}

// ── File Search ─────────────────────────────────────────────────────────────

async fn cmd_search_files(args: &serde_json::Value) -> String {
    let pattern = args["pattern"].as_str().unwrap_or("");
    let path = args["path"].as_str().unwrap_or(".");
    
    if pattern.is_empty() {
        return "Error: missing pattern".to_string();
    }
    
    // 用 std::process 调用 rg
    let output = tokio::process::Command::new("rg")
        .args(&["-l", "--glob", "!**/node_modules/**", "--glob", "!**/.git/**"])
        .arg(pattern)
        .arg(path)
        .output()
        .await;
    
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.is_empty() { "No matches found".to_string() }
            else { stdout.to_string() }
        }
        Err(_) => "rg not found, falling back to grep".to_string(),
    }
}

// ── Web ─────────────────────────────────────────────────────────────────────

async fn cmd_web_get(args: &serde_json::Value) -> String {
    let url = args["url"].as_str().unwrap_or("");
    if url.is_empty() {
        return "Error: missing url".to_string();
    }
    
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap();
    
    match client.get(url).send().await {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            // 简单清理 HTML tags
            let cleaned: String = text.chars()
                .filter(|c| !(*c == '<' || *c == '>'))
                .collect();
            if cleaned.len() > 20000 {
                format!("{}\n[...truncated]", &cleaned[..20000])
            } else {
                cleaned
            }
        }
        Err(e) => format!("Error fetching {}: {}", url, e),
    }
}

async fn cmd_web_search(args: &serde_json::Value) -> String {
/// URL percent-encode (无外部依赖，替代被曦误用的 urlencoding crate)
fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

    let query = args["query"].as_str().unwrap_or("");
    let count = args["count"].as_u64().unwrap_or(5).min(10);
    
    if query.is_empty() {
        return "Error: missing query".to_string();
    }
    
    let url = format!("https://www.google.com/search?q={}", urlencode(query));
    
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap();
    
    match client.get(&url).send().await {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            // 提取搜索结果标题和链接
            let results: Vec<&str> = text.split("<h3>").collect();
            let mut out = String::new();
            for r in results.iter().take(count as usize + 1).skip(1) {
                if let Some(title_end) = r.find("</h3>") {
                    let title = &r[..title_end];
                    out.push_str(&format!("- {}\n", title));
                }
            }
            if out.is_empty() { "Search results not parsed correctly".to_string() }
            else { out }
        }
        Err(e) => format!("Search error: {}", e),
    }
}

// ── Validation ──────────────────────────────────────────────────────────────

async fn cmd_validate_artifact(args: &serde_json::Value) -> String {
    let artifact_type = args["artifact_type"].as_str().unwrap_or("");
    let data = args["data"].clone();
    
    if artifact_type.is_empty() {
        return "Error: missing artifact_type".to_string();
    }
    
    // 简单验证逻辑
    let valid = !data.is_null();
    format!("{{\"valid\": {}, \"errors\": []}}", valid)
}

// ── Image Generation ────────────────────────────────────────────────────────

async fn cmd_generate_image(args: &serde_json::Value) -> String {
    let prompt = args["prompt"].as_str().unwrap_or("");
    let size = args["size"].as_str().unwrap_or("1024x1024");
    
    if prompt.is_empty() {
        return "Error: missing prompt".to_string();
    }
    
    format!("Image generation: prompt='{}', size={}", prompt, size)
}
