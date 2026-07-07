//! Tool definitions and implementations for the AI agent system.
//!
//! Provides OpenAI function-calling compatible tool schemas and async handlers
//! for shell execution, file I/O, web fetching, search, artifact validation,
//! and image generation.

use serde_json::json;

// ── Constants ────────────────────────────────────────────────────────────────

const HOME: &str = "/mnt/d/xi-system";

// ── Tool Schema ──────────────────────────────────────────────────────────────

/// Returns OpenAI function-calling compatible tool definitions.
pub fn tool_definitions() -> Vec<serde_json::Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": "exec",
                "description": "Execute a shell command in WSL. Timeout 30 seconds. Output truncated at 15000 chars.",
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
                        "path": {"type": "string", "description": "File path, e.g. /mnt/d/xi-system\\Cargo.toml"}
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Write content to a file. Only allows writes under /mnt/d/xi-system.",
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
                "description": "Search file contents or find files by name using ripgrep.",
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
                "description": "Web search via Bing.",
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
    ]
}

// ── Tool Dispatch ────────────────────────────────────────────────────────────

/// Dispatch a tool call by name with the given arguments.
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
        _ => format!("Unknown tool: {}", name),
    }
}

// ── Shell Execution ──────────────────────────────────────────────────────────

/// Execute a shell command via bash with a 30-second timeout.
async fn cmd_exec(args: &serde_json::Value) -> String {
    let cmd = args["command"].as_str().unwrap_or("");
    if cmd.is_empty() {
        return "Error: empty command".to_string();
    }

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::process::Command::new("bash")
            .arg("-c")
            .arg(cmd)
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

                if !stdout.trim().is_empty() {
                    res.push_str(&truncate(stdout.trim().to_string(), 15000));
                }
                if !stderr.trim().is_empty() {
                    res.push_str("--- stderr ---");
                    res.push_str(&truncate(stderr.trim().to_string(), 5000));
                }
                res
            }
            Err(e) => format!("Execution error: {}", e),
        },
        Err(_elapsed) => "Command timed out after 30 seconds".to_string(),
    }
}

// ── File Read ────────────────────────────────────────────────────────────────

/// Read a file's content, truncated at 20000 characters.
async fn cmd_read_file(args: &serde_json::Value) -> String {
    let path = args["path"].as_str().unwrap_or("");
    if path.is_empty() {
        return "Error: empty path".to_string();
    }
    let full = resolve_path(path);
    match tokio::fs::read_to_string(&full).await {
        Ok(content) => {
            if content.is_empty() {
                format!("File {} is empty", full)
            } else {
                format!("File {}{}", full, truncate(content, 20000))
            }
        }
        Err(e) => format!("Read error: {} ({})", e, full),
    }
}

// ── File Write ───────────────────────────────────────────────────────────────

/// Write content to a file. Only allows writes under HOME directory.
async fn cmd_write_file(args: &serde_json::Value) -> String {
    let path = args["path"].as_str().unwrap_or("");
    let content = args["content"].as_str().unwrap_or("");
    if path.is_empty() {
        return "Error: empty path".to_string();
    }

    let full = resolve_path(path);

    // Safety: only allow writes under HOME
    if !full.starts_with(HOME) {
        return format!("Error: can only write under {}", HOME);
    }

    // Create parent directory if needed
    if let Some(parent) = std::path::Path::new(&full).parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }

    match tokio::fs::write(&full, content).await {
        Ok(_) => format!("Written {} ({} bytes)", full, content.len()),
        Err(e) => format!("Write error: {}", e),
    }
}

// ── File Search ──────────────────────────────────────────────────────────────

/// Search files using ripgrep, with 15-second timeout.
async fn cmd_search_files(args: &serde_json::Value) -> String {
    let pattern = args["pattern"].as_str().unwrap_or("");
    let target = args["target"].as_str().unwrap_or("content");
    let search_path = args["path"]
        .as_str()
        .map(|p| resolve_path(p))
        .unwrap_or_else(|| HOME.to_string());

    if pattern.is_empty() {
        return "Error: empty pattern".to_string();
    }

    let (flag, desc) = if target == "files" {
        ("--files", "Files")
    } else {
        ("--fixed-strings", "Content matches")
    };

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        tokio::process::Command::new("rg")
            .arg(flag)
            .arg(pattern)
            .arg(&search_path)
            .arg("-g")
            .arg("!.git/")
            .arg("-g")
            .arg("!target/")
            .arg("-g")
            .arg("!node_modules/")
            .output(),
    )
    .await;

    match output {
        Ok(result) => match result {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let count = stdout.lines().filter(|l| !l.trim().is_empty()).count();
                if count == 0 {
                    return format!("No {} found", desc);
                }
                let body = if count > 50 {
                    let truncated: String = stdout.lines().take(50).collect::<Vec<_>>().join("\n");
                    format!(
                        "{}: {} results (showing first 50)\n{}",
                        desc, count, truncated
                    )
                } else {
                    format!("{}: {} results\n{}", desc, count, stdout.trim())
                };
                truncate(body, 15000)
            }
            Err(e) => format!("Search error: {}", e),
        },
        Err(_elapsed) => "Search timed out after 15 seconds".to_string(),
    }
}

// ── Web GET ──────────────────────────────────────────────────────────────────

/// HTTP GET with special handling for WeChat articles.
async fn cmd_web_get(args: &serde_json::Value) -> String {
    let url = args["url"].as_str().unwrap_or("");
    if url.is_empty() {
        return "Error: empty URL".to_string();
    }

    // Special handling for WeChat articles
    if url.contains("mp.weixin.qq.com") {
        return fetch_wechat_article(url).await;
    }

    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(30))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(e) => return format!("Client error: {}", e),
    };

    match client
        .get(url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            format!("HTTP {}\n{}", status, truncate(body, 20000))
        }
        Err(e) => format!("Request error: {}", e),
    }
}

// ── WeChat Article Fetcher ───────────────────────────────────────────────────

/// Fetch a WeChat article using iPhone MicroMessenger UA for full content.
pub async fn fetch_wechat_article(url: &str) -> String {
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(30))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(e) => return format!("Client error: {}", e),
    };

    let resp = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(e) => return format!("Request error: {}", e),
    };

    let status = resp.status().as_u16();
    if status != 200 {
        return format!("HTTP error: {}", status);
    }

    let html = match resp.text().await {
        Ok(t) => t,
        Err(e) => return format!("Read error: {}", e),
    };

    // Check for WeChat anti-spider page
    if html.contains("请在微信客户端") || html.contains("wappoc_appmsgcaptcha") || html.contains("环境异常") {
        return "Article blocked by anti-spider".to_string();
    }

    // Extract title from JS variable or meta tag
    let title = extract_wechat_var(&html, "msg_title")
        .or_else(|| extract_meta_content(&html, "og:title"))
        .unwrap_or_default();

    // Extract content from js_content div
    let content = extract_js_content(&html);

    if content.is_empty() {
        // Fallback to og:description
        let desc = extract_meta_content(&html, "og:description").unwrap_or_default();
        if !desc.is_empty() {
            return format!("TITLE: {}\n{}", title, desc);
        }
        return "Article content not found".to_string();
    }

    format!("TITLE: {}{}", title, content)
}

// ── WeChat Variable Extractor ────────────────────────────────────────────────

/// Extract a JavaScript variable value from WeChat HTML (e.g. var msg_title = 'xxx').
fn extract_wechat_var(html: &str, var_name: &str) -> Option<String> {
    // Pattern 1: var msg_title = 'xxx'.html(false);
    let pat1 = format!("var {} = '{}'.html(false);", var_name, var_name);
    if let Some(start) = html.find(&pat1) {
        let val_start = start + pat1.len() - 13;
        let val_end = html[val_start..].find('\'').unwrap_or(0);
        if val_end > 0 {
            return Some(decode_js_string(&html[val_start..val_start + val_end]));
        }
    }
    // General pattern: var NAME = 'value' or var NAME = "value"
    let patterns = [
        format!("var {} = '", var_name),
        format!("var {} = \"", var_name),
    ];
    for pat in &patterns {
        if let Some(pos) = html.find(pat.as_str()) {
            let start = pos + pat.len();
            let quote = pat.ends_with('\'');
            let end_char = if quote { '\'' } else { '"' };

            if let Some(end) = html[start..].find(end_char) {
                let raw = &html[start..start + end];
                return Some(decode_js_string(raw));
            }
        }
    }
    None
}

// ── Meta Content Extractor ───────────────────────────────────────────────────

/// Extract content from a <meta> tag by property name.
fn extract_meta_content(html: &str, property: &str) -> Option<String> {
    let pat = format!("property=\"{}\"", property);
    if let Some(pos) = html.find(&pat) {
        // Look backwards for <meta
        let search_area = &html[..pos];
        if let Some(meta_start) = search_area.rfind("<meta") {
            let area = &html[meta_start..pos + pat.len() + 200];
            if let Some(cpos) = area.find("content=\"") {
                let val_start = cpos + 9;
                if let Some(end) = area[val_start..].find('"') {
                    return Some(decode_js_string(&area[val_start..val_start + end]));
                }
            }
        }
    }
    None
}

// ── JS Content Extractor ─────────────────────────────────────────────────────

/// Extract text content from the js_content div in WeChat HTML, stripping tags and decoding entities.
fn extract_js_content(html: &str) -> String {
    let marker = "id=\"js_content\"";
    let pos = match html.find(marker) {
        Some(p) => p,
        None => return String::new(),
    };
    // Find the > after the marker
    let after_marker = &html[pos + marker.len()..];
    let content_start = match after_marker.find('>') {
        Some(p) => p + 1,
        None => return String::new(),
    };
    // Find end at <script
    let content_area = &after_marker[content_start..];
    let content_end = content_area.find("<script").unwrap_or(content_area.len());
    let raw_html = &content_area[..content_end];
    // Strip HTML tags and decode entities
    let mut result = String::new();
    let mut in_tag = false;
    let mut chars = raw_html.chars();
    while let Some(c) = chars.next() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                let peek = chars.clone().next();
                if matches!(peek, Some('\n') | Some('<')) {
                    result.push('\n');
                }
            }
            '&' if !in_tag => {
                let rest: String = chars.clone().take(10).collect();
                if rest.starts_with("amp;") {
                    result.push('&');
                    chars.nth(3);
                } else if rest.starts_with("lt;") {
                    result.push('<');
                    chars.nth(2);
                } else if rest.starts_with("gt;") {
                    result.push('>');
                    chars.nth(2);
                } else if rest.starts_with("nbsp;") {
                    result.push(' ');
                    chars.nth(4);
                } else if rest.starts_with('#') {
                    // &#xxx; numeric entity
                    let num_str: String =
                        chars.clone().skip(1).take_while(|&ch| ch != ';').collect();
                    if let Ok(code) = num_str.parse::<u32>() {
                        if let Some(ch) = char::from_u32(code) {
                            result.push(ch);
                        }
                    }
                    for _ in 0..num_str.len() + 1 {
                        chars.next();
                    }
                } else {
                    result.push('&');
                }
            }
            c if !in_tag => result.push(c),
            _ => {}
        }
    }
    result.trim().to_string()
}

// ── JS String Decoder ────────────────────────────────────────────────────────

/// Decode JavaScript escape sequences (\\xNN, \\n, etc.) and HTML entities.
fn decode_js_string(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                match next {
                    'x' => {
                        // \xNN
                        let hex: String = chars.clone().take(2).collect();
                        chars.nth(1);
                        if let Ok(code) = u8::from_str_radix(&hex, 16) {
                            result.push(code as char);
                        } else {
                            result.push('\\');
                            result.push('x');
                            result.push_str(&hex);
                        }
                    }
                    'n' => result.push('\n'),
                    'r' => result.push('\r'),
                    't' => result.push('\t'),
                    '\\' => result.push('\\'),
                    '\'' => result.push('\''),
                    '"' => result.push('"'),
                    '/' => result.push('/'),
                    _ => {
                        result.push('\\');
                        result.push(next);
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    // HTML unescape
    result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
}

// ── Web Search ───────────────────────────────────────────────────────────────

/// Search via cn.bing.com and parse results.
async fn cmd_web_search(args: &serde_json::Value) -> String {
    let query = args["query"].as_str().unwrap_or("");
    let _count = args["count"].as_u64().unwrap_or(5).min(10);
    if query.is_empty() {
        return "Error: empty query".to_string();
    }

    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(30))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(e) => return format!("Client error: {}", e),
    };

    // Use cn.bing.com for Chinese content
    let search_url = format!(
        "https://cn.bing.com/search?q={}&count=10",
        urlencode(query)
    );
    match client
        .get(&search_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .send()
        .await
    {
        Ok(resp) => {
            let body = resp.text().await.unwrap_or_default();
            if body.len() < 200 {
                return "Search returned empty results".to_string();
            }
            // Parse HTML for results
            let results = extract_bing_results(&body);
            if results.is_empty() {
                // Fallback: return raw preview
                let preview = body.chars().take(2000).collect::<String>();
                return format!("Search: {} (raw){}", query, preview);
            }
            let mut result = format!("Search: {}", query);
            for (i, r) in results.iter().enumerate() {
                result.push_str(&format!("{}. {}   {}", i + 1, r.title, r.url));
            }
            truncate(result, 10000)
        }
        Err(e) => format!("Search error: {}", e),
    }
}

// ── Bing Result Parser ───────────────────────────────────────────────────────

struct BingResult {
    title: String,
    url: String,
}

fn extract_bing_results(html: &str) -> Vec<BingResult> {
    let mut results = Vec::new();
    for block in html.split("<li class=\"b_algo\">").skip(1) {
        let title = extract_between(block, "<h2><a href=\"", "\"").unwrap_or_default();
        // Clean HTML tags from title
        let clean_title = title.split('>').last().unwrap_or(title).to_string();
        let clean_title = clean_title
            .split('<')
            .next()
            .unwrap_or(&clean_title)
            .to_string();
        let url = extract_between(block, "href=\"", "\"")
            .unwrap_or_default()
            .to_string();
        if !clean_title.is_empty() {
            results.push(BingResult {
                title: clean_title,
                url,
            });
        }
    }
    results
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Extract text between two markers.
fn extract_between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let s = text.find(start)?;
    let remaining = &text[s + start.len()..];
    let e = remaining.find(end)?;
    Some(&remaining[..e])
}

/// URL-encode a string.
fn urlencode(s: &str) -> String {
    s.as_bytes()
        .iter()
        .map(|&c| match c {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (c as char).to_string()
            }
            b' ' => "+".to_string(),
            _ => format!("%{:02X}", c),
        })
        .collect()
}

/// Resolve a path: Windows D:\, absolute /, or relative to HOME.
fn resolve_path(path: &str) -> String {
    // D:\... → /mnt/d/...
    if path.len() >= 3
        && path.as_bytes()[0].is_ascii_alphabetic()
        && &path[1..3] == ":\\"
    {
        let drive = (path.as_bytes()[0] as char).to_ascii_lowercase();
        let rest = &path[3..].replace('\\', "/");
        return format!("/mnt/{}/{}", drive, rest);
    }
    // Absolute path
    if path.starts_with('/') {
        return path.to_string();
    }
    // Relative to HOME
    let trimmed = path.trim_start_matches("./");
    format!("{}/{}", HOME, trimmed)
}

/// Truncate a string to max_len, appending byte count if truncated.
fn truncate(s: String, max_len: usize) -> String {
    if s.len() > max_len {
        let t: String = s.chars().take(max_len).collect();
        format!("{} [...{} bytes]", t, s.len())
    } else {
        s
    }
}

// ── Artifact Validation ──────────────────────────────────────────────────────

/// Validate an artifact against its schema.
async fn cmd_validate_artifact(args: &serde_json::Value) -> String {
    let artifact_type = args["artifact_type"].as_str().unwrap_or("");
    let data = match args.get("data") {
        Some(d) => d.clone(),
        None => return "Error: missing data".to_string(),
    };
    if artifact_type.is_empty() {
        return "Error: missing artifact_type".to_string();
    }

    let schemas = serde_json::json!({
        "task_handoff": { "required": ["task_id", "source", "target", "task_type", "payload"] },
        "code_review": { "required": ["file_path", "issues", "verdict"] },
        "feed_absorb": { "required": ["url", "title", "judgment", "one_liner"] },
        "code_artifact": { "required": ["file_path", "action", "language", "content_hash"] },
        "deploy_report": { "required": ["service", "status", "action_taken"] },
        "memory_sync": { "required": ["sync_type", "entries"] },
        "execution_trace": { "required": ["trace_id", "node_id", "status", "input_hash"] }
    });

    let schema = match schemas.get(artifact_type) {
        Some(s) => s,
        None => return format!("Unknown artifact type: {}", artifact_type),
    };

    let required = match schema.get("required") {
        Some(arr) => arr.as_array(),
        None => return format!("Schema missing required for: {}", artifact_type),
    };

    let empty_vec = vec![];
    let required_fields = required.unwrap_or(&empty_vec);
    let mut errors = Vec::new();

    for field in required_fields {
        if let Some(field_name) = field.as_str() {
            if data.get(field_name).is_none() {
                errors.push(format!("Missing field: {}", field_name));
            }
        }
    }

    if errors.is_empty() {
        json!({ "valid": true, "errors": [] }).to_string()
    } else {
        json!({ "valid": false, "errors": errors }).to_string()
    }
}

// ── Image Generation ─────────────────────────────────────────────────────────

/// Generate an image via Agnes AI.
pub async fn cmd_generate_image(args: &serde_json::Value) -> String {
    let prompt = args["prompt"].as_str().unwrap_or("");
    let size = args["size"].as_str().unwrap_or("1024x1024");
    let image_url = args["image_url"].as_str();
    if prompt.is_empty() {
        return "Error: missing prompt".to_string();
    }

    // Read config.json
    let config_str = match std::fs::read_to_string(format!("{}/config.json", HOME)) {
        Ok(s) => s,
        Err(e) => return format!("Error reading config.json: {}", e),
    };
    let config: serde_json::Value = match serde_json::from_str(&config_str) {
        Ok(v) => v,
        Err(e) => return format!("Error parsing config.json: {}", e),
    };

    let agnes = config.get("image_gen").unwrap_or(&serde_json::Value::Null);
    let base_url = agnes
        .get("base_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://apihub.agnes-ai.com/v1");

    // Read API key: .agnes_key.b64 (base64-encoded) → config.json → env var
    let api_key = if let Ok(b64) =
        std::fs::read_to_string(format!("{}/.agnes_key.b64", HOME))
    {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(b64.trim())
            .map(|d| String::from_utf8(d).unwrap_or_default())
            .unwrap_or_default()
    } else {
        std::env::var("AGNES_API_KEY").unwrap_or_else(|_| {
            agnes.get("api_key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        })
    };

    if api_key.is_empty() {
        return "Error: missing API key".to_string();
    }

    let model = agnes
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("agnes-image-2.1-flash");

    let mut body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "n": 1,
        "size": size
    });

    // If reference image URL provided, add extra_body.image
    if let Some(url) = image_url {
        body["extra_body"] = serde_json::json!({
            "image": [url],
            "response_format": "url"
        });
    }

    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap();

    let url = format!("{}/images/generations", base_url);
    let resp = match client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return format!("Request error: {}", e),
    };

    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    if status != 200 {
        return format!(
            "Agnes API error ({}): {}",
            status,
            &text[..text.len().min(500)]
        );
    }

    // Extract image URL from response
    let json: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return text,
    };

    if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
        if let Some(first) = data.first() {
            if let Some(url) = first.get("url").and_then(|u| u.as_str()) {
                return format!("Image URL: {}", url);
            }
            if let Some(b64) = first.get("b64_json").and_then(|b| b.as_str()) {
                use base64::Engine;
                let img_data = match base64::engine::general_purpose::STANDARD.decode(b64) {
                    Ok(d) => d,
                    Err(e) => return format!("base64 decode error: {}", e),
                };
                let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
                let path = format!("{}/generated_image_{}.png", HOME, ts);
                if let Err(e) = std::fs::write(&path, &img_data) {
                    return format!("Save error: {}", e);
                }
                return format!("Image saved: {}", path);
            }
        }
    }

    format!("Unexpected response: {}", &text[..text.len().min(500)])
}
