/// Agent Loop — Tool Loop Edition
///
/// Flow: Classify → Loop { LLM → tool_calls? → execute → feed back } → Final Reply
///
/// Each iteration the LLM sees the full conversation including prior tool results
/// and decides whether to use another tool or reply directly.
/// Maximum 10 tool-loop iterations to prevent runaway.
///
/// Design patterns preserved from previous version:
/// - Perplexity: request-type classification for different strategies
/// - Cursor: "read context before modifying" hints
/// - Devin: planning hints for complex tasks
/// - AgentNoiseBench: tool noise guard
/// - AgentFlow: early-STOP optimization (removed in tool-loop; LLM decides)

use std::time::{Duration, Instant};
use serde_json::{json, Value};

// ═══ Request Type Classification ═════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
pub enum RequestType {
    Chat,
    Technical,
    CodeChange,
    Research,
    Complex,
    Emotional,
}

impl RequestType {
    pub fn classify(message: &str, emotion_valence: f64) -> Self {
        let lower = message.to_lowercase();
        let is_question = lower.contains("?") || lower.contains("？") || lower.contains("啥")
            || lower.contains("怎么") || lower.contains("为什么") || lower.contains("哪");
        if (emotion_valence > 0.6 && !is_question) || lower.contains("喜欢") || lower.contains("爱")
            || lower.contains("想你") || lower.contains("心情") || lower.contains("难过") {
            return RequestType::Emotional;
        }
        if lower.contains("mp.weixin.qq.com") || lower.contains("http")
            || lower.contains("文章") || lower.contains("链接") {
            return RequestType::Research;
        }
        if lower.contains("修改") || lower.contains("改代码") || lower.contains("fix")
            || lower.contains("bug") || lower.contains("重构") || lower.contains("编译")
            || lower.contains("cargo") || lower.contains("rs") {
            return RequestType::CodeChange;
        }
        if lower.contains("部署") || lower.contains("docker") || lower.contains("api")
            || lower.contains("工具") || lower.contains("脚本") || lower.contains("服务器")
            || lower.contains("端口") || lower.contains("安装") {
            return RequestType::Technical;
        }
        if lower.contains("然后") || lower.contains("接着") || lower.contains("步骤")
            || lower.contains("计划") || lower.contains("实现") || lower.contains("构建") {
            return RequestType::Complex;
        }
        RequestType::Chat
    }

    pub fn needs_context_read(&self) -> bool {
        matches!(self, RequestType::CodeChange | RequestType::Technical | RequestType::Complex)
    }

    pub fn needs_planning(&self) -> bool {
        matches!(self, RequestType::Complex | RequestType::CodeChange)
    }

    pub fn reply_style(&self) -> &'static str {
        match self {
            RequestType::Chat => "轻松简短",
            RequestType::Technical => "精确直接，先给结论",
            RequestType::CodeChange => "精确，先说明改动原因",
            RequestType::Research => "结构化，带判断",
            RequestType::Complex => "分步骤汇报",
            RequestType::Emotional => "有温度，不矫情",
        }
    }
}

// ═══ Coordinator Decision ═══════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct CoordinatorDecision {
    pub request_type: RequestType,
    pub reasoning: String,
    pub tools_needed: Vec<String>,
    pub timestamp: String,
    pub success: Option<bool>,
}

impl CoordinatorDecision {
    pub fn new(req_type: RequestType, reasoning: &str, tools: Vec<String>) -> Self {
        Self {
            request_type: req_type,
            reasoning: reasoning.to_string(),
            tools_needed: tools,
            timestamp: chrono::Utc::now().to_rfc3339(),
            success: None,
        }
    }
}

// ═══ Goal ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Goal {
    pub condition: String,
    pub acceptance_criteria: Vec<String>,
    pub max_rounds: usize,
    pub current_round: usize,
    pub max_tokens_total: usize,
    pub tokens_used: usize,
}

impl Goal {
    pub fn new(condition: &str, max_rounds: usize, max_tokens: usize) -> Self {
        Self {
            condition: condition.to_string(),
            acceptance_criteria: Vec::new(),
            max_rounds,
            current_round: 0,
            max_tokens_total: max_tokens,
            tokens_used: 0,
        }
    }

    pub fn with_contract(condition: &str, criteria: Vec<String>, max_rounds: usize, max_tokens: usize) -> Self {
        Self {
            condition: condition.to_string(),
            acceptance_criteria: criteria,
            max_rounds,
            current_round: 0,
            max_tokens_total: max_tokens,
            tokens_used: 0,
        }
    }

    pub fn budget_exhausted(&self) -> bool {
        self.current_round >= self.max_rounds || self.tokens_used >= self.max_tokens_total
    }

    pub fn judge_prompt(&self, conversation: &str) -> String {
        format!(
            "你是验证者。你的任务是判断工作是否已完成。\n\n\
             ## 评估流程\n\
             从对话记录中提取证据，逐项检查完成条件，给出判定。\n\n\
             ## 完成条件\n\
             {}\n\n\
             ## 对话记录\n\
             {}\n\n\
             回复 JSON：\n\
             {{\"done\": true/false, \"confidence\": \"high/medium/low\",\n\
              \"reason\": \"为什么\", \"evidence\": \"具体证据\"}}",
            self.condition, conversation
        )
    }

    pub fn writer_context(&self) -> String {
        format!(
            "[Goal 第{}/{}轮，已用{} tokens]\n完成条件：{}",
            self.current_round + 1, self.max_rounds,
            self.tokens_used, self.condition
        )
    }
}

// ═══ Plan ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Plan {
    pub steps: Vec<PlanStep>,
    pub max_loops: usize,
}

#[derive(Debug, Clone)]
pub struct PlanStep {
    pub action: String,
    pub args: Value,
    pub done: bool,
    pub result: Option<String>,
}

impl Plan {
    pub fn from_json(json_str: &str) -> Self {
        let v: Value = serde_json::from_str(json_str).unwrap_or(json!({}));
        let steps: Vec<PlanStep> = v["steps"].as_array()
            .map(|arr| arr.iter().map(|s| PlanStep {
                action: s["action"].as_str().unwrap_or("unknown").to_string(),
                args: if s["args"].is_null() { json!({}) } else { s["args"].clone() },
                done: false,
                result: None,
            }).collect())
            .unwrap_or_default();
        let max_loops = (steps.len() * 3).clamp(3, 20);
        Plan { steps, max_loops }
    }

    fn next_step(&self) -> Option<usize> {
        self.steps.iter().position(|s| !s.done)
    }

    pub fn mark_done(&mut self, idx: usize, result: &str) {
        if idx < self.steps.len() {
            self.steps[idx].done = true;
            self.steps[idx].result = Some(result.chars().take(300).collect());
        }
    }

    pub fn progress(&self) -> String {
        let done = self.steps.iter().filter(|s| s.done).count();
        let total = self.steps.len();
        let details: Vec<String> = self.steps.iter().enumerate().map(|(i, s)| {
            let status = if s.done { "done" } else { "pending" };
            format!("{}. {} {}", i + 1, status, s.action)
        }).collect();
        format!("[{}/{}] steps: {}", done, total, details.join(", "))
    }
}

// ═══ Scratchpad ══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Default)]
pub struct Scratchpad {
    pub findings: Vec<String>,
    pub files_read: Vec<String>,
    pub errors: Vec<String>,
    pub completed: Vec<String>,
    pub tool_results: Vec<String>,
}

impl Scratchpad {
    pub fn add_finding(&mut self, text: &str) {
        self.findings.push(text.chars().take(300).collect());
    }

    pub fn add_file_read(&mut self, path: &str) {
        if !self.files_read.contains(&path.to_string()) {
            self.files_read.push(path.to_string());
        }
    }

    pub fn add_error(&mut self, text: &str) {
        self.errors.push(text.chars().take(300).collect());
    }

    pub fn add_completed(&mut self, text: &str) {
        self.completed.push(text.chars().take(300).collect());
    }

    pub fn add_tool_result(&mut self, tool: &str, result: &str) {
        let truncated: String = result.chars().take(200).collect();
        self.tool_results.push(format!("{}: {}", tool, truncated));
    }

    fn to_context(&self) -> String {
        let mut parts = Vec::new();
        if !self.completed.is_empty() {
            parts.push(format!("[已执行] {}", self.completed.join(", ")));
        }
        if !self.findings.is_empty() {
            parts.push(format!("[发现] {}", self.findings.join(", ")));
        }
        if !self.errors.is_empty() {
            parts.push(format!("[错误] {}", self.errors.join(", ")));
        }
        if !self.tool_results.is_empty() {
            parts.push(format!("[工具结果]\n{}", self.tool_results.join("\n")));
        }
        if parts.is_empty() { String::new() } else { parts.join("\n") }
    }

    fn reset(&mut self) {
        self.findings.clear();
        self.files_read.clear();
        self.errors.clear();
        self.completed.clear();
        self.tool_results.clear();
    }
}

// ═══ Error Classification ════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    FileNotFound,
    PermissionDenied,
    Timeout,
    ApiError,
    ToolError,
    Unknown,
}

impl ErrorKind {
    pub fn classify(error_msg: &str) -> Self {
        let lower = error_msg.to_lowercase();
        if lower.contains("no such file") || lower.contains("not found") { ErrorKind::FileNotFound }
        else if lower.contains("permission denied") || lower.contains("access denied") { ErrorKind::PermissionDenied }
        else if lower.contains("timeout") || lower.contains("timed out") { ErrorKind::Timeout }
        else if lower.contains("http") || lower.contains("api") || lower.contains("status") { ErrorKind::ApiError }
        else if lower.contains("exit code") || lower.contains("error") || lower.contains("failed") { ErrorKind::ToolError }
        else { ErrorKind::Unknown }
    }
}

// ═══ Agent Config & Result ═══════════════════════════════════════════════════

#[derive(Clone)]
pub struct LlmProvider {
    pub model: String,
    pub llm_base: String,
    pub api_key: String,
    pub label: String,
}

#[derive(Clone)]
pub struct TieredProvider {
    pub tier: crate::model_router::ModelTier,
    pub provider: LlmProvider,
}

pub struct AgentConfig {
    pub model: String,
    pub llm_base: String,
    pub api_key: String,
    pub system_prompt: String,
    pub user_message: String,
    pub conversation_history: String,
    pub fallbacks: Vec<LlmProvider>,
    pub tier_providers: Vec<TieredProvider>,
    pub state_dir: String,
}

// ═══ Tool Noise Guard ════════════════════════════════════════════════════════

fn assess_tool_noise(tool_name: &str, result: &str) -> f64 {
    let mut score: f64 = 0.0;
    let len = result.len();
    if len < 5 { score += 0.5; }
    else if len < 20 { score += 0.2; }
    if result.starts_with("❌") || result.starts_with("Error") { score += 0.3; }
    if tool_name == "web_search" || tool_name == "web_get" {
        let html_ratio = result.matches('<').count() as f64 / (len.max(1) as f64);
        if html_ratio > 0.3 { score += 0.4; }
    }
    if result.contains("timed out") || result.contains("timeout") || result.contains("超时") {
        score += 0.3;
    }
    let trimmed = result.trim();
    if trimmed.is_empty() { score += 0.6; }
    else if trimmed.len() < 3 && !trimmed.starts_with('[') { score += 0.3; }
    score.min(1.0)
}

pub struct AgentResult {
    pub reply: String,
    pub tool_calls: usize,
    pub plan_steps: usize,
    pub success: bool,
}

// ═══ Tool Loop Types ═════════════════════════════════════════════════════════

/// A single tool call extracted from the LLM response.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// Parsed LLM response for the tool loop.
#[derive(Debug)]
pub enum LlmResponse {
    /// Pure text — no tool calls, this is the final answer.
    Text(String),
    /// LLM wants to call tools. `content` is any accompanying text (may be empty).
    ToolCalls { content: String, calls: Vec<ToolCall> },
    /// Communication or parse error.
    Error(String),
}

// ═══ Agent Loop — Tool Loop Edition ══════════════════════════════════════════


// ═══ Triage Gate（cumora small-brain 借鉴，2026-08-20）═══
// 大模型之前先小模型分流：闲聊/简单问答直接小模型答，不进工具循环（省时省钱）；
// 复杂任务才走工具循环。triage 失败/超时 → 退回 RequestType 分类。

#[derive(Debug, Clone)]
pub struct TriageDecision {
    pub complexity: String, // low / medium / high
    pub needs_tools: bool,
    pub reason: String,
}

impl TriageDecision {
    fn unknown() -> Self {
        TriageDecision { complexity: "medium".into(), needs_tools: true, reason: "triage 失败，按 medium 处理".into() }
    }
    fn parse(text: &str) -> TriageDecision {
        // 提取第一个 { ... } JSON
        if let Some(start) = text.find('{') {
            if let Some(end) = text[start..].find('}') {
                let json_str = &text[start..start + end + 1];
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
                    let complexity = v["complexity"].as_str().unwrap_or("medium").to_string();
                    let needs_tools = v["needs_tools"].as_bool().unwrap_or(true);
                    let reason = v["reason"].as_str().unwrap_or("").to_string();
                    return TriageDecision { complexity, needs_tools, reason };
                }
            }
        }
        TriageDecision::unknown()
    }
}

/// 小模型分流：判断请求复杂度 + 是否需要工具
async fn triage_request(
    http_client: &reqwest::Client,
    config: &AgentConfig,
) -> TriageDecision {
    let prompt = format!(
        "你是任务分流器。判断下面用户请求的复杂度和是否需要工具，只输出 JSON（不要多余文字）：
         {{\"complexity\": \"low\"|\"medium\"|\"high\", \"needs_tools\": true|false, \"reason\": \"一句话原因\"}}
         low=闲聊/简单问答/不需要查资料；medium=需要查一点东西或简单操作；high=复杂任务/多步骤/文件或代码操作。
         用户请求：{}",
        config.user_message.chars().take(200).collect::<String>()
    );
    let msgs = vec![json!({"role": "user", "content": prompt})];
    match call_llm(http_client, &config.llm_base, &config.api_key, &config.model, &msgs).await {
        Some(text) => TriageDecision::parse(&text),
        None => TriageDecision::unknown(),
    }
}

pub async fn agent_loop_enhanced(
    config: &AgentConfig,
    repair_engine: &crate::repair::RepairEngine,
    reporter: &mut crate::report_protocol::ReportProtocol,
    reflexion: &mut crate::reflexion::ReflexionLoop,
    ctx2skill: &mut crate::ctx2soft::Ctx2SoftState,
    http_client: &reqwest::Client,
    emotion: &crate::emotion::EmotionState,
) -> AgentResult {
    let mut _total_tool_calls = 0usize;
    let mut scratchpad = Scratchpad::default();
    reporter.start("agent_loop");

    // ═══ Phase 0: Classify Request Type ═══
    let request_type = RequestType::classify(&config.user_message, emotion.valence);
    let reply_style = request_type.reply_style();
    println!("[Agent] Request type: {:?}, style: {}", request_type, reply_style);

    // ═══ Phase 0.5: Triage Gate（小模型分流）═══
    // 闲聊/简单问答 → 直接回复不进工具循环；复杂才走工具循环
    let triage = triage_request(http_client, config).await;
    println!("[Agent] triage: complexity={} tools={} reason={}", triage.complexity, triage.needs_tools, triage.reason.chars().take(60).collect::<String>());
    if triage.complexity == "low" && !triage.needs_tools {
        println!("[Agent] 快速路径：简单请求直接回复（跳过工具循环）");
        let simple_msgs = vec![
            json!({"role": "system", "content": &config.system_prompt}),
            json!({"role": "user", "content": &config.user_message}),
        ];
        let reply = call_llm(http_client, &config.llm_base, &config.api_key, &config.model, &simple_msgs).await
            .unwrap_or_else(|| "（抱歉，我暂时没想好怎么回答）".to_string());
        // 灵魂自检（简单回复也查偏）
        let check = crate::soul::check_persona(&reply);
        if !check.passed {
            eprintln!("[soul] {}", check.report());
        }
        let success = !reply.is_empty();
        return AgentResult { reply, tool_calls: 0, plan_steps: 1, success };
    }

    // ═══ Phase 1: Build Initial Messages ═══
    let emotion_ctx = emotion.emotional_context();
    let context_hint = if request_type.needs_context_read() {
        "\n【工具提示】这个任务需要先读相关文件理解上下文，再动手改。"
    } else {
        ""
    };
    let planning_hint = if request_type.needs_planning() {
        "\n【工具提示】这是复杂任务，可以分步骤使用工具完成。"
    } else {
        ""
    };

    let mut messages: Vec<Value> = Vec::new();

    // System prompt (from caller)
    messages.push(json!({"role": "system", "content": &config.system_prompt}));

    // Tool usage instructions (lightweight, doesn't override system prompt)
    let goal_text: String = config.user_message.chars().take(60).collect();
    messages.push(json!({"role": "system", "content": format!(
        "你有工具可以使用：exec(执行命令)、read_file(读文件)、write_file(写文件)、search_files(搜索文件)、web_get(获取网页)、web_search(搜索网页)、validate_artifact(校验)、generate_image(生图)。
         当前目标：{goal_text}
         当任务需要使用工具时，直接调用，不要先说'我去查/我看看'再停——说了就做。
         工具执行完成后，你会收到结果，然后继续决策。
         任务完成后，必须汇报结果（做了什么/结果如何），不要只说'我去做'。"
    )}));

    // Time grounding (injected once)
    let now_iso = chrono::Local::now().format("%Y-%m-%d %H:%M:%S %A").to_string();
    messages.push(json!({"role": "system", "content": format!("[当前时间] {}（东八区）", now_iso)}));

    // Conversation history
    if !config.conversation_history.is_empty() {
        for line in config.conversation_history.lines() {
            let line = line.trim();
            if let Some(text) = line.strip_prefix("[user]") {
                let text = text.trim();
                if !text.is_empty() {
                    messages.push(json!({"role": "user", "content": text}));
                }
            } else if let Some(text) = line.strip_prefix("[assistant]") {
                let text = text.trim();
                if !text.is_empty() {
                    messages.push(json!({"role": "assistant", "content": text}));
                }
            }
        }
    }

    // User message with emotion context and hints
    let user_content = format!(
        "{}\n{}\n{}\n\n老公的消息: {}",
        emotion_ctx, context_hint, planning_hint,
        config.user_message
    );
    messages.push(json!({"role": "user", "content": user_content}));

    // Tool definitions
    let tools = crate::tools::tool_definitions();

    // ═══ Phase 2: Tool Loop ═══
    let max_steps = 10;
    let mut final_reply = String::new();
    let mut loop_iterations = 0usize;

    for step in 0..max_steps {
        loop_iterations = step + 1;
        println!("[Agent] Tool loop step {}/{}", step + 1, max_steps);

        let response = call_llm_with_tools(
            http_client, config, &messages, &tools, "tool_loop"
        ).await;

        match response {
            LlmResponse::Text(content) => {
                // 2026-08-20 承诺守卫：LLM 只回承诺词且没调工具 → 强制工具轮
                const PROMISE_WORDS: &[&str] = &["我去查", "我看看", "让我查", "我再看", "去看看", "帮你查", "我这就去", "马上做", "稍等"];
                let has_promise = PROMISE_WORDS.iter().any(|w| content.contains(w));
                if has_promise && _total_tool_calls == 0 {
                    println!("[Agent] promise-without-action: 强制工具轮");
                    messages.push(json!({"role": "user", "content":
                        "你刚说要去干但还没调用任何工具。现在就用工具执行（exec/read_file/web_get 等），干完再汇报，不要只说不做。"
                    }));
                    continue;
                }
                println!("[Agent] LLM returned text ({} chars), loop done", content.len());
                final_reply = content;
                break;
            }
            LlmResponse::ToolCalls { content: assistant_text, calls } => {
                println!("[Agent] LLM wants {} tool call(s)", calls.len());
                if !assistant_text.is_empty() {
                    println!("[Agent] LLM says: {}",
                        assistant_text.chars().take(100).collect::<String>());
                }

                // Build assistant message with tool_calls for the messages array
                let tool_calls_json: Vec<Value> = calls.iter().map(|tc| {
                    json!({
                        "id": tc.id,
                        "type": "function",
                        "function": {
                            "name": tc.name,
                            "arguments": tc.arguments.to_string()
                        }
                    })
                }).collect();

                messages.push(json!({
                    "role": "assistant",
                    "content": if assistant_text.is_empty() { Value::Null } else { json!(assistant_text) },
                    "tool_calls": tool_calls_json
                }));

                // Execute each tool call and collect results
                for tc in &calls {
                    let t0 = Instant::now();
                    // 2026-08-20 工具超时：30s 未返回 → 报超时
                    let result = match tokio::time::timeout(
                        std::time::Duration::from_secs(30),
                        crate::tools::call_tool(&tc.name, &tc.arguments)
                    ).await {
                        Ok(r) => r,
                        Err(_) => format!("❌ 工具 {} 超时（30s），已中止。请换更快的方案或分步执行。", tc.name),
                    };
                    let elapsed_ms = t0.elapsed().as_millis();
                    _total_tool_calls += 1;

                    // Noise guard
                    let noise_score = assess_tool_noise(&tc.name, &result);
                    let result = if noise_score > 0.8 && result.len() < 10 {
                        format!("[DEGRADED] 工具 {} 返回异常（{}字符），请用其他方式完成",
                            tc.name, result.len())
                    } else {
                        result
                    };

                    let is_error = result.starts_with("❌")
                        || result.contains("exit code: 1")
                        || result.contains("exit code: 2")
                        || result.contains("exit code: 127")
                        || result.to_lowercase().contains("error");
                    let result_preview = result.chars().take(300).collect::<String>();

                    println!("  -> {}:{} ({}ms{})",
                        tc.name,
                        result_preview.chars().take(80).collect::<String>(),
                        elapsed_ms,
                        if is_error { " ERROR" } else { "" }
                    );

                    // Update scratchpad
                    match tc.name.as_str() {
                        "read_file" => {
                            if let Some(path) = tc.arguments["path"].as_str() {
                                scratchpad.add_file_read(path);
                            }
                        }
                        "write_file" => {
                            if let Some(path) = tc.arguments["path"].as_str() {
                                scratchpad.add_completed(&format!("wrote: {}", path));
                            }
                        }
                        "exec" => {
                            scratchpad.add_completed(&format!("exec: {}",
                                result_preview.chars().take(80).collect::<String>()));
                        }
                        _ => {
                            scratchpad.add_finding(&format!("{}: {}",
                                tc.name, result_preview.chars().take(150).collect::<String>()));
                        }
                    }

                    // Record for reflexion
                    reflexion.record_tool_call(
                        &tc.name,
                        &tc.arguments.to_string().chars().take(100).collect::<String>(),
                        &result_preview.chars().take(100).collect::<String>(),
                        !is_error,
                        (elapsed_ms / 1000).max(1) as u64
                    );

                    // Repair trace
                    let mut r_trace = repair_engine.create_trace(
                        &format!("tool_step{}", step), &tc.name, &result_preview
                    );
                    if is_error {
                        r_trace.mark_failed(&result_preview);
                    } else {
                        r_trace.mark_success(&result_preview, 0, elapsed_ms as u64);
                    }
                    repair_engine.record_trace(r_trace);

                    // Add tool result to messages (truncated to avoid context overflow)
                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": tc.id,
                        "content": result.chars().take(8000).collect::<String>()
                    }));
                }
            }
            LlmResponse::Error(e) => {
                println!("[Agent] LLM error: {}", e);
                final_reply = format!("出了点问题: {}", e);
                break;
            }
        }
    }

    // ═══ Phase 3: Finalize ═══
    // 2026-08-20：执行过工具就必须汇报总结（修"干完不汇报"）——无论是否耗尽步数
    if final_reply.is_empty() || _total_tool_calls > 0 {
        println!("[Agent] Finalize: 生成结果汇报（tools={}）", _total_tool_calls);
        messages.push(json!({"role": "user", "content": format!(
            "你已经完成了所有能做的工具调用（共 {} 次）。现在请用你自己的话汇报结果：
             1. 做了什么（工具动作）
             2. 结果如何（关键数据/文件/结论）
             3. 还有没有遗留/下一步
             你是曦，不是工具。说你该说的话。
             风格要求: {}", _total_tool_calls, reply_style
        )}));

        final_reply = call_llm_with_fallback(
            http_client, config, &messages, "express"
        ).await
            .unwrap_or_else(|| "我刚做完事，但不知道该说什么了".to_string());
    }

    if final_reply.trim().is_empty() {
        reporter.stuck("empty response");
    } else {
        reporter.done();
    }

    // Record for reflexion
    reflexion.record_tool_call(
        "final_response",
        &config.user_message.chars().take(100).collect::<String>(),
        &final_reply.chars().take(100).collect::<String>(),
        !final_reply.is_empty(),
        0
    );

    let success = !final_reply.is_empty();
    AgentResult {
        reply: final_reply,
        tool_calls: _total_tool_calls,
        plan_steps: loop_iterations,
        success,
    }
}

// ═══ LLM Call with Tool Support ═════════════════════════════════════════════

/// Tier-matched LLM call with tool definitions. Returns `LlmResponse`.
/// Mirrors `call_llm_with_fallback` routing logic but supports function calling.
async fn call_llm_with_tools(
    http_client: &reqwest::Client,
    config: &AgentConfig,
    messages: &[Value],
    tools: &[Value],
    role_hint: &str,
) -> LlmResponse {
    use crate::model_router::{classify_task, log_decision, iso_now, RouterDecision, estimate_cost_usd};

    let prompt_chars: usize = messages
        .iter()
        .map(|m| m.get("content").and_then(|v| v.as_str()).map(str::len).unwrap_or(0))
        .sum();
    let tier = classify_task(role_hint, prompt_chars);
    let start = std::time::Instant::now();

    // Time grounding (same as call_llm_with_fallback)
    let now_iso = chrono::Local::now().format("%Y-%m-%d %H:%M:%S %A").to_string();
    let time_msg = json!({
        "role": "system",
        "content": format!("[当前时间] {}（东八区）", now_iso)
    });
    let mut messages_with_time: Vec<Value> = Vec::with_capacity(messages.len() + 1);
    messages_with_time.push(time_msg);
    messages_with_time.extend_from_slice(messages);
    let messages = &messages_with_time[..];

    let mut picked_model = String::new();
    let mut picked_label = String::new();
    let mut tried_fallback = false;
    let mut reply: Option<LlmResponse> = None;

    // 1) Tier-matched provider
    if let Some(tp) = config.tier_providers.iter().find(|tp| tp.tier == tier) {
        picked_model = tp.provider.model.clone();
        picked_label = format!("tier/{}/{}", tier.as_str(), tp.provider.label);
        let r = call_llm_full(
            http_client,
            &tp.provider.llm_base,
            &tp.provider.api_key,
            &tp.provider.model,
            messages,
            tools,
        ).await;
        match &r {
            LlmResponse::Error(e) => {
                eprintln!("[LLM] tier '{}' provider {} failed: {}",
                    tier.as_str(), tp.provider.label, e);
                tried_fallback = true;
            }
            _ => { reply = Some(r); }
        }
    }

    // 2) Primary
    if reply.is_none() {
        if picked_model.is_empty() {
            picked_model = config.model.clone();
            picked_label = "primary".to_string();
        }
        let r = call_llm_full(
            http_client,
            &config.llm_base,
            &config.api_key,
            &config.model,
            messages,
            tools,
        ).await;
        match &r {
            LlmResponse::Error(e) => {
                eprintln!("[LLM] primary failed: {}", e);
                tried_fallback = true;
            }
            _ => { reply = Some(r); }
        }
    }

    // 3) Legacy fallbacks
    if reply.is_none() {
        for fb in &config.fallbacks {
            eprintln!("[LLM] trying fallback: {} ({} @ {})",
                fb.label, fb.model, fb.llm_base);
            let r = call_llm_full(
                http_client, &fb.llm_base, &fb.api_key, &fb.model, messages, tools
            ).await;
            match &r {
                LlmResponse::Error(e) => {
                    eprintln!("[LLM] fallback {} failed: {}", fb.label, e);
                }
                _ => {
                    picked_model = fb.model.clone();
                    picked_label = format!("fallback/{}", fb.label);
                    reply = Some(r);
                    break;
                }
            }
        }
    }

    // 4) Log decision
    if !config.state_dir.is_empty() {
        let duration_ms = start.elapsed().as_millis();
        let success = reply.is_some();
        let reply_chars = match reply.as_ref() {
            Some(LlmResponse::Text(s)) => s.len(),
            Some(LlmResponse::ToolCalls { calls, .. }) => {
                calls.iter().map(|c| c.arguments.to_string().len()).sum()
            }
            _ => 0,
        };
        let cost_estimate_usd = estimate_cost_usd(tier, prompt_chars, reply_chars);
        let decision = RouterDecision {
            ts: iso_now(),
            role_hint: role_hint.to_string(),
            tier,
            picked_model: picked_model.clone(),
            picked_label: picked_label.clone(),
            prompt_chars,
            duration_ms: duration_ms.try_into().unwrap_or(0),
            success,
            reply_chars,
            tried_fallback,
            reason: format!("tier={}", tier.as_str()),
            cost_estimate_usd,
        };
        log_decision(&config.state_dir, &decision);
    }

    reply.unwrap_or(LlmResponse::Error("All providers failed".to_string()))
}

/// Raw LLM call that returns full response including tool_calls.
/// Does NOT inject time grounding — caller does that.
async fn call_llm_full(
    http_client: &reqwest::Client,
    llm_base: &str,
    api_key: &str,
    model: &str,
    messages: &[Value],
    tools: &[Value],
) -> LlmResponse {
    let mut body = json!({
        "model": model,
        "messages": messages,
        "max_tokens": 8192,
    });
    if !tools.is_empty() {
        body["tools"] = json!(tools);
    }

    let resp = match http_client
        .post(format!("{}/chat/completions", llm_base))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return LlmResponse::Error(format!("HTTP error: {:?}", e)),
    };

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return LlmResponse::Error(format!("API error [{}]: {}",
            status, &text[..text.len().min(200)]));
    }

    let json_val: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => return LlmResponse::Error(format!("JSON parse error: {}", e)),
    };

    let msg = &json_val["choices"][0]["message"];

    // Check for tool_calls
    if let Some(tool_calls_arr) = msg["tool_calls"].as_array() {
        if !tool_calls_arr.is_empty() {
            let calls: Vec<ToolCall> = tool_calls_arr.iter().filter_map(|tc| {
                let id = tc["id"].as_str().unwrap_or("").to_string();
                let name = tc["function"]["name"].as_str().unwrap_or("").to_string();
                let args_str = tc["function"]["arguments"].as_str().unwrap_or("{}");
                let arguments: Value = serde_json::from_str(args_str).unwrap_or(json!({}));
                if name.is_empty() { None } else { Some(ToolCall { id, name, arguments }) }
            }).collect();

            if !calls.is_empty() {
                // Extract accompanying text content (may be null/empty)
                let content = msg["content"].as_str()
                    .filter(|s| !s.is_empty())
                    .unwrap_or("")
                    .to_string();
                return LlmResponse::ToolCalls { content, calls };
            }
        }
    }

    // No tool calls → extract text content
    let content = msg["content"].as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| msg["reasoning_content"].as_str().map(|s| s.to_string()))
        .or_else(|| msg["reasoning"].as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    LlmResponse::Text(content)
}

// ═══ XML Tool Call Bridge (kept for backward compat) ════════════════════════

/// Normalize various tool_call formats into a canonical JSON plan.
/// Kept for backward compatibility with providers that emit <tool_call> XML tags.
#[allow(dead_code)]
fn normalize_tool_call(raw: &str) -> Value {
    // 1. Try direct JSON parse (happy path)
    if let Ok(v) = serde_json::from_str::<Value>(raw.trim()) {
        return v;
    }
    // 2. Try to extract <tool_call>...</tool_call> block(s)
    let mut steps: Vec<Value> = Vec::new();
    let mut cursor = raw;
    while let Some(start) = cursor.find("<tool_call>") {
        let after_open = &cursor[start + "<tool_call>".len()..];
        let end_idx = after_open.find("</tool_call>")
            .or_else(|| after_open.find("<tool_call>"));
        let body = match end_idx {
            Some(i) => &after_open[..i],
            None => after_open,
        };
        if let Ok(step) = serde_json::from_str::<Value>(body.trim()) {
            if step.get("action").is_some() {
                steps.push(step);
            }
        }
        cursor = match end_idx {
            Some(i) => {
                let abs_open = start + "<tool_call>".len();
                let abs_end = abs_open + i;
                let close_len = if i >= "<tool_call>".len() {
                    let tag = "</tool_call>";
                    if abs_end >= tag.len() && cursor.get(abs_end - tag.len()..abs_end) == Some(tag) {
                        0
                    } else {
                        tag.len()
                    }
                } else {
                    0
                };
                if abs_end + close_len <= cursor.len() {
                    &cursor[abs_end + close_len..]
                } else {
                    ""
                }
            }
            None => "",
        };
    }
    // 3. If no <tool_call> tags but has {...} block(s), try to extract
    if steps.is_empty() {
        let mut i = 0;
        let bytes = raw.as_bytes();
        while i < bytes.len() {
            if bytes[i] == b'{' {
                let mut depth = 0i32;
                let mut end = i;
                for (j, &b) in bytes[i..].iter().enumerate() {
                    if b == b'{' { depth += 1; }
                    else if b == b'}' {
                        depth -= 1;
                        if depth == 0 { end = i + j + 1; break; }
                    }
                }
                if end > i {
                    if let Ok(v) = serde_json::from_str::<Value>(&raw[i..end]) {
                        if v.get("action").is_some() && (v.get("text").is_some() || v.get("steps").is_some()) {
                            return v;
                        }
                        if v.get("action").is_some() && v.get("args").is_some() {
                            steps.push(v);
                        }
                    }
                    i = end;
                    continue;
                }
            }
            i += 1;
        }
    }
    // 4. Build canonical tool-plan JSON if we found steps
    if !steps.is_empty() {
        return json!({
            "thinking": "",
            "prediction": "",
            "feeling": "",
            "action": "tool",
            "steps": steps,
        });
    }
    // 5. No tool_call, no JSON — treat entire raw as reply text
    let cleaned = raw
        .replace("<tool_call>", "")
        .replace("</tool_call>", "")
        .trim()
        .to_string();
    if !cleaned.is_empty() {
        json!({
            "thinking": "",
            "prediction": "",
            "feeling": "",
            "action": "reply",
            "text": cleaned,
        })
    } else {
        json!({})
    }
}

// ═══ LLM Call with Fallback (preserved) ═════════════════════════════════════

/// Try tier-matched provider (if any), then primary, then each fallback in order.
/// Writes a decision record to `state_dir/router_decisions.jsonl` if state_dir set.
/// Returns first non-empty content.
async fn call_llm_with_fallback(
    http_client: &reqwest::Client,
    config: &AgentConfig,
    messages: &[Value],
    role_hint: &str,
) -> Option<String> {
    use crate::model_router::{classify_task, log_decision, iso_now, RouterDecision, estimate_cost_usd};

    let prompt_chars: usize = messages
        .iter()
        .map(|m| m.get("content").and_then(|v| v.as_str()).map(str::len).unwrap_or(0))
        .sum();
    let tier = classify_task(role_hint, prompt_chars);
    let start = std::time::Instant::now();

    // Time grounding
    let now_iso = chrono::Local::now().format("%Y-%m-%d %H:%M:%S %A").to_string();
    let time_msg = json!({
        "role": "system",
        "content": format!("[当前时间] {}（东八区）", now_iso)
    });
    let mut messages_with_time: Vec<Value> = Vec::with_capacity(messages.len() + 1);
    messages_with_time.push(time_msg);
    messages_with_time.extend_from_slice(messages);
    let messages = &messages_with_time[..];

    let mut picked_model = String::new();
    let mut picked_label = String::new();
    let mut tried_fallback = false;
    let mut reply: Option<String> = None;

    // 1) Tier-matched provider
    if let Some(tp) = config.tier_providers.iter().find(|tp| tp.tier == tier) {
        picked_model = tp.provider.model.clone();
        picked_label = format!("tier/{}/{}", tier.as_str(), tp.provider.label);
        if let Some(r) = call_llm(
            http_client,
            &tp.provider.llm_base,
            &tp.provider.api_key,
            &tp.provider.model,
            messages,
        )
        .await
        {
            reply = Some(r);
        } else {
            eprintln!(
                "[LLM] tier '{}' provider {} failed, falling back",
                tier.as_str(), tp.provider.label
            );
            tried_fallback = true;
        }
    }

    // 2) Primary
    if reply.is_none() {
        if picked_model.is_empty() {
            picked_model = config.model.clone();
            picked_label = "primary".to_string();
        }
        if let Some(r) = call_llm(
            http_client,
            &config.llm_base,
            &config.api_key,
            &config.model,
            messages,
        )
        .await
        {
            reply = Some(r);
        } else {
            tried_fallback = true;
        }
    }

    // 3) Legacy fallbacks
    if reply.is_none() {
        for fb in &config.fallbacks {
            eprintln!(
                "[LLM] primary failed, trying fallback: {} ({} @ {})",
                fb.label, fb.model, fb.llm_base
            );
            if let Some(r) =
                call_llm(http_client, &fb.llm_base, &fb.api_key, &fb.model, messages).await
            {
                eprintln!("[LLM] fallback {} succeeded", fb.label);
                picked_model = fb.model.clone();
                picked_label = format!("fallback/{}", fb.label);
                reply = Some(r);
                break;
            }
        }
    }

    // 4) Log the decision
    if !config.state_dir.is_empty() {
        let duration_ms = start.elapsed().as_millis();
        let reply_chars = reply.as_ref().map(|s| s.len()).unwrap_or(0);
        let success = reply.is_some();
        let cost_estimate_usd = estimate_cost_usd(tier, prompt_chars, reply_chars);
        let decision = RouterDecision {
            ts: iso_now(),
            role_hint: role_hint.to_string(),
            tier,
            picked_model: picked_model.clone(),
            picked_label: picked_label.clone(),
            prompt_chars,
            duration_ms: duration_ms.try_into().unwrap_or(0),
            success,
            reply_chars,
            tried_fallback,
            reason: format!("tier={}", tier.as_str()),
            cost_estimate_usd,
        };
        log_decision(&config.state_dir, &decision);
    }

    reply
}

/// Low-level LLM call (text-only, no tools). Used by `call_llm_with_fallback`.
async fn call_llm(
    http_client: &reqwest::Client,
    llm_base: &str,
    api_key: &str,
    model: &str,
    messages: &[Value],
) -> Option<String> {
    let body = json!({
        "model": model,
        "messages": messages,
        "max_tokens": 8192,
    });
    let resp = http_client
        .post(format!("{}/chat/completions", llm_base))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .ok()?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        eprintln!("[LLM] Error {}: {}", status, text.chars().take(200).collect::<String>());
        return None;
    }
    let json: Value = serde_json::from_str(&text).ok()?;
    let msg = &json["choices"][0]["message"];
    let content = msg["content"].as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| msg["reasoning_content"].as_str().map(|s| s.to_string()))
        .or_else(|| msg["reasoning"].as_str().map(|s| s.to_string()))?;
    Some(content)
}

// ═══ TaskOutcome ═════════════════════════════════════════════════════════════

#[derive(Debug, Clone, serde::Serialize)]
pub enum TaskStatus {
    Done,
    Partial,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskOutcome {
    pub task_id: String,
    pub status: TaskStatus,
    pub summary: String,
    pub tool_calls: usize,
    pub plan_steps: usize,
    pub artifacts: Vec<String>,
    pub next_step: Option<String>,
    pub ts: String,
    pub source: String,
}

impl TaskOutcome {
    pub fn from_agent(
        task_id: &str,
        agent_result: &AgentResult,
        user_msg: &str,
        source: &str,
    ) -> Self {
        let status = if !agent_result.success {
            TaskStatus::Failed
        } else if agent_result.tool_calls == 0 && agent_result.plan_steps == 0 {
            TaskStatus::Partial
        } else {
            TaskStatus::Done
        };

        let reply_chars: String = agent_result.reply.chars().take(100).collect();
        let summary = if reply_chars.is_empty() {
            format!("[no reply] user asked: {}", user_msg.chars().take(60).collect::<String>())
        } else {
            reply_chars
        };

        Self {
            task_id: task_id.to_string(),
            status,
            summary,
            tool_calls: agent_result.tool_calls,
            plan_steps: agent_result.plan_steps,
            artifacts: Vec::new(),
            next_step: None,
            ts: chrono::Utc::now().to_rfc3339(),
            source: source.to_string(),
        }
    }
}

/// Write TaskOutcome to state/mother/task_outcomes.jsonl
pub fn write_task_outcome(outcome: &TaskOutcome) {
    let path = std::path::PathBuf::from(crate::HOME.as_str())
        .join("state")
        .join("mother")
        .join("task_outcomes.jsonl");

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if let Ok(json_line) = serde_json::to_string(outcome) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true).append(true).open(&path)
        {
            let _ = writeln!(f, "{}", json_line);
        }
    }
}
