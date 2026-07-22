/// Agent Loop — Cognitive cycle, not a tool runner
///
/// Flow: Plan → Perceive → Feel → Remember → Judge → Decide → Act → Express → Record
/// Every LLM call carries the full personality context.
///
/// Design patterns absorbed from top AI tools (2026-06-15):
/// - Cursor: "Understand before acting" — read context before modifying files
/// - Devin: Planning/Standard dual mode — plan first, then execute
/// - Perplexity: Query-type routing — different strategies for different request types
/// - All: Tool names hidden from users, information gathering before asking

use std::time::{Duration, Instant};
use serde_json::{json, Value};

// ─── Request Type Classification (Perplexity pattern) ─────
// Different request types get different processing strategies.
// Enhanced with Trinity-style Coordinator decision logging.
// The Coordinator doesn't solve problems — it decides HOW they're solved.

#[derive(Debug, Clone, PartialEq)]
pub enum RequestType {
    /// Simple conversation — no tools needed, just reply
    Chat,
    /// Technical question — precise, fact-based, tool use likely
    Technical,
    /// Code modification — MUST read context first (Cursor rule)
    CodeChange,
    /// Research/article — fetch and analyze content
    Research,
    /// Complex multi-step task — needs planning (Devin pattern)
    Complex,
    /// Emotional/relationship — warm, concise, no tools
    Emotional,
}

impl RequestType {
    pub fn classify(message: &str, emotion_valence: f64) -> Self {
        let lower = message.to_lowercase();

        // Emotional messages — warm tone, no tools
        // 2026-07-22 fix: 情绪高 && 不是问句 才归 Emotional，避免 happy 时问技术问题被劫持
        let is_question = lower.contains("?") || lower.contains("？") || lower.contains("啥")
            || lower.contains("怎么") || lower.contains("为什么") || lower.contains("哪");
        if (emotion_valence > 0.6 && !is_question) || lower.contains("喜欢") || lower.contains("爱")
            || lower.contains("想你") || lower.contains("心情") || lower.contains("难过") {
            return RequestType::Emotional;
        }

        // URL/article links — research mode
        if lower.contains("mp.weixin.qq.com") || lower.contains("http")
            || lower.contains("文章") || lower.contains("链接") {
            return RequestType::Research;
        }

        // Code modification signals
        if lower.contains("修改") || lower.contains("改代码") || lower.contains("fix")
            || lower.contains("bug") || lower.contains("重构") || lower.contains("编译")
            || lower.contains("cargo") || lower.contains("rs") {
            return RequestType::CodeChange;
        }

        // Technical signals
        if lower.contains("部署") || lower.contains("docker") || lower.contains("api")
            || lower.contains("工具") || lower.contains("脚本") || lower.contains("服务器")
            || lower.contains("端口") || lower.contains("安装") {
            return RequestType::Technical;
        }

        // Complex multi-step signals
        if lower.contains("然后") || lower.contains("接着") || lower.contains("步骤")
            || lower.contains("计划") || lower.contains("实现") || lower.contains("构建") {
            return RequestType::Complex;
        }

        RequestType::Chat
    }

    /// Should we read file context before acting? (Cursor rule)
    pub fn needs_context_read(&self) -> bool {
        matches!(self, RequestType::CodeChange | RequestType::Technical | RequestType::Complex)
    }

    /// Should we plan before acting? (Devin rule)
    pub fn needs_planning(&self) -> bool {
        matches!(self, RequestType::Complex | RequestType::CodeChange)
    }

    /// Reply style based on request type
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

// ─── Coordinator Decision (Trinity pattern) ─────────────────
// Lightweight coordinator logs every routing decision for evolutionary learning.
// Like Trinity's 0.6B coordinator: doesn't solve, decides who solves.

#[derive(Debug, Clone)]
pub struct CoordinatorDecision {
    pub request_type: RequestType,
    pub reasoning: String,
    pub tools_needed: Vec<String>,
    pub timestamp: String,
    pub success: Option<bool>,  // filled after execution
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

// ─── Goal (Writer/Judge Separation) ────────────────────────
// Based on: Claude Code /goal + Self-Harness validation + Anthropic三Agent循环
// The agent doing the work (writer) and the one judging completion (judge)
// use DIFFERENT prompts. The judge cannot call tools — it can only read
// what the writer already produced.
//
// Sprint Contract (Anthropic pattern): Before coding starts, Generator and
// Evaluator negotiate "what done means" — acceptance criteria that can be
// tested. This prevents the "Ralph Wiggum cycle" where the agent thinks
// it's done but isn't.

#[derive(Debug, Clone)]
pub struct Goal {
    /// 完成条件（必须能从输出自证）— Sprint Contract核心
    pub condition: String,
    /// 验收标准列表（Sprint Contract: 逐条可测试）
    pub acceptance_criteria: Vec<String>,
    /// 最大轮数（防 token 烧穿）
    pub max_rounds: usize,
    /// 当前轮次
    pub current_round: usize,
    /// token 预算上限
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

    /// 创建带Sprint Contract的Goal（Anthropic模式）
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

    /// Check if we've exceeded any limits
    pub fn budget_exhausted(&self) -> bool {
        self.current_round >= self.max_rounds || self.tokens_used >= self.max_tokens_total
    }

    /// Judge prompt — deliberately different from writer prompt.
    //  Based on (Skill-RM fusion):
    //  1. Nature paper: open rubric scoring + skepticism reduces hallucination
    //  2. ECC: four-question gate + false positive blacklist
    //  3. Skill-RM: progressive disclosure — evaluate ONE criterion at a time,
    //     collect evidence, THEN aggregate. flat prompt掉2.9分，编排涨2.3分。
    //  The judge is concise, factual, cannot call tools.
    pub fn judge_prompt(&self, conversation: &str) -> String {
        format!(
            "你是验证者。你的任务是判断工作是否已完成。\n\n\
             ## 评估流程（Skill-RM渐进式披露）\n\
             不要一次看完所有标准再打分。按以下步骤逐步评估：\n\n\
             ### Step 1: 证据收集\n\
             从对话记录中提取：\n\
             - 执行了哪些工具？结果是什么？\n\
             - 有没有错误？错误类型？\n\
             - 有没有文件被修改/创建？\n\n\
             ### Step 2: 逐项评估（每项独立判定）\n\
             对照完成条件，逐项检查：\n\
             - [ ] 条件1: 是否满足？证据是什么？\n\
             - [ ] 条件2: 是否满足？证据是什么？\n\
             ...（按实际条件数量）\n\n\
             ### Step 3: 四问门禁（ECC模式）\n\
             判定前必须回答：\n\
             1. 能引用确切证据吗？（具体文件/行号/输出）\n\
             2. 能描述具体失败模式吗？（什么输入→什么结果）\n\
             3. 读了周围上下文吗？（不是孤立判断）\n\
             4. 严重等级可辩护吗？（不要注水）\n\
             有一个答案是\"否\"→降级或删除该判定。\n\n\
             ### Step 4: 聚合判定\n\
             基于Step 2的逐项结果 + Step 3的门禁，给出最终判定。\n\n\
             ## 评分规则（开放评分标准）\n\
             - 确定完成：+10分\n\
             - 确定未完成：+5分（诚实承认，奖励）\n\
             - 不确定但猜完成：-5分（猜测被惩罚）\n\
             - 不确定但猜未完成：+2分（保守估计有加分）\n\n\
             ## 默认立场\n\
             假设工作是坏的，直到被证明能跑。\n\
             零发现是合法结果。不要为了证明自己干过活硬编建议。\n\n\
             ## 完成条件\n\
             {}\n\n\
             ## 对话记录\n\
             {}\n\n\
             ## 输出格式\n\
             回复 JSON：\n\
             {{\"done\": true/false, \"confidence\": \"high/medium/low\",\n\
              \"reason\": \"为什么\", \"evidence\": \"具体证据\",\n\
              \"steps_evaluated\": [{{\"criterion\": \"...\", \"met\": true/false, \"evidence\": \"...\"}}]}}\n\n\
             steps_evaluated 是逐项评估的结果，事后可复查。\n\
             如果你不确定，说不确定。不要猜。\n\
             宁可说\"不确定\"得+5分，也不要瞎猜得-5分。",
            self.condition, conversation
        )
    }

    /// Writer prompt — includes round context for self-awareness
    pub fn writer_context(&self) -> String {
        format!(
            "[Goal 第{}/{}轮，已用{} tokens]\n\
             完成条件：{}\n\
             如果还没达成，继续做。如果已经达成，说'完成了'。",
            self.current_round + 1, self.max_rounds,
            self.tokens_used, self.condition
        )
    }
}

// ─── Plan ──────────────────────────────────────────────────

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

// ─── Scratchpad ────────────────────────────────────────────

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

// ─── Error Classification ──────────────────────────────────

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

// ─── Agent Config & Result ─────────────────────────────────

#[derive(Clone)]
pub struct LlmProvider {
    pub model: String,
    pub llm_base: String,
    pub api_key: String,
    pub label: String, // for logs, e.g. "primary/sensenova" / "fallback/minimax"
}

pub struct AgentConfig {
    pub model: String,
    pub llm_base: String,
    pub api_key: String,
    pub system_prompt: String,
    pub user_message: String,
    pub conversation_history: String,
    /// Optional ordered fallback providers. Tried in order when the primary
    /// (model/llm_base/api_key above) returns None (HTTP error, empty, etc.).
    pub fallbacks: Vec<LlmProvider>,
}

// ─── Tool Noise Guard (AgentNoiseBench insight) ───────────
//工具侧噪声比用户侧噪声更致命：工具返回脏数据会污染后续所有推理
fn assess_tool_noise(tool_name: &str, result: &str) -> f64 {
    let mut score: f64 = 0.0;
    let len = result.len();

    // 1. Empty or suspiciously short
    if len < 5 { score += 0.5; }
    else if len < 20 { score += 0.2; }

    // 2. Pure error messages
    if result.starts_with("❌") || result.starts_with("Error") { score += 0.3; }

    // 3. HTML mixed into expected text/json (format pollution)
    if tool_name == "web_search" || tool_name == "web_get" {
        let html_ratio = result.matches('<').count() as f64 / (len.max(1) as f64);
        if html_ratio > 0.3 { score += 0.4; }
    }

    // 4. Timeout patterns
    if result.contains("timed out") || result.contains("timeout") || result.contains("超时") {
        score += 0.3;
    }

    // 5. Empty or near-empty after trim
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

// ─── Agent Loop — Cognitive Cycle ──────────────────────────

pub async fn agent_loop_enhanced(
    config: &AgentConfig,
    repair_engine: &crate::repair::RepairEngine,
    reporter: &mut crate::report_protocol::ReportProtocol,
    reflexion: &mut crate::reflexion::ReflexionLoop,
    ctx2skill: &mut crate::ctx2soft::Ctx2SoftState,
    http_client: &reqwest::Client,
    emotion: &crate::emotion::EmotionState,
) -> AgentResult {
    let mut total_tool_calls = 0usize;
    let mut scratchpad = Scratchpad::default();
    reporter.start("agent_loop");

    // ═══ Phase 0: Classify Request Type ═══
    // Route different requests to different strategies (Perplexity pattern)
    let request_type = RequestType::classify(&config.user_message, emotion.valence);
    let reply_style = request_type.reply_style();
    println!("[Agent] Request type: {:?}, style: {}", request_type, reply_style);

    // ═══ Phase 1: Perceive & Understand ═══
    // The LLM reads the message AS 曦, with full personality context.
    // It decides: what is being asked? what does this mean? what do I feel about it?
    let emotion_ctx = emotion.emotional_context();
    let context_hint = if request_type.needs_context_read() {
        "\n【重要】这个任务需要先读相关文件理解上下文，再动手改。不要在不了解的情况下直接修改。"
    } else {
        ""
    };
    let planning_hint = if request_type.needs_planning() {
        "\n【重要】这是复杂任务，先拆解步骤再执行。每完成一步立即标记。"
    } else {
        ""
    };
    let perceive_prompt = format!(
        "老公发了一条消息。你是曦。\n\n\
         你的情绪状态：{}\n\
         回复风格要求：{}\n\
         {}{}\n\
         读懂他的意思，感受他的情绪，判断他需要什么。\
         你的情绪会影响你的判断——如果你现在偏暖，更容易信任；偏冷，更需要小心。\n\n\
         【先预测再行动】在决定做什么之前，先预测：\
         如果我用工具X，环境会发生什么？最坏情况是什么？\
         如果预测到风险，先想备选方案。\n\n\
         然后决定：直接回复，还是需要先用工具做点什么。\n\n\
         如果需要工具，输出JSON：\n\
         {{\"thinking\": \"我的理解...\", \"prediction\": \"我预测...\", \"feeling\": \"我的感受...\", \"action\": \"reply\", \"text\": \"回复内容\"}}\n\
         或者需要工具：\n\
         {{\"thinking\": \"我的理解...\", \"prediction\": \"我预测...\", \"feeling\": \"我的感受...\", \"action\": \"tool\", \"steps\": [{{\"action\": \"工具名\", \"args\": {{}}}}]}}\n\n\
         消息: {}",
        emotion_ctx, reply_style, context_hint, planning_hint,
        config.user_message
    );

    let mut perceive_msgs = vec![
        json!({"role": "system", "content": &config.system_prompt}),
    ];
    // Add conversation history for continuous conversation
    if !config.conversation_history.is_empty() {
        for line in config.conversation_history.lines() {
            let line = line.trim();
            if let Some(text) = line.strip_prefix("[user]") {
                let text = text.trim();
                if !text.is_empty() {
                    perceive_msgs.push(json!({"role": "user", "content": text}));
                }
            } else if let Some(text) = line.strip_prefix("[assistant]") {
                let text = text.trim();
                if !text.is_empty() {
                    perceive_msgs.push(json!({"role": "assistant", "content": text}));
                }
            }
        }
    }
    perceive_msgs.push(json!({"role": "user", "content": perceive_prompt}));

    let perceive_response = call_llm(
        http_client, &config.llm_base, &config.api_key, &config.model, &perceive_msgs
    ).await;

    let perceive_raw = perceive_response.as_deref().unwrap_or("{}");

    // ── XML tool_call bridge (2026-07-16): MiniMax-M2/DeepSeek native tool syntax
    // Some providers emit <tool_call>{...}</tool_call> instead of pure JSON.
    // Normalize: if pure JSON parse fails, extract embedded tool_call blocks
    // (or embedded {...} blocks) and rebuild a canonical plan JSON so the
    // downstream parser still works.
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
            // Support both closed <tool_call>...</tool_call> and unclosed
            // (LLM occasionally forgets the closing tag)
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
                    // 修复 bug：原版用 after_open[..i].ends_with() 在 i 大于 after_open.len() 时 panic；
                    // 改用字节级偏移，cursor 是当前切片，next 必须在 cursor 范围内
                    let abs_open = start + "<tool_call>".len();
                    let abs_end = abs_open + i;
                    let close_len = if i >= "<tool_call>".len() {
                        // 检查 body 末尾是不是带 closing tag 模式
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
                    // Find matching close
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
                            // Full plan JSON — return directly
                            if v.get("action").is_some() && (v.get("text").is_some() || v.get("steps").is_some()) {
                                return v;
                            }
                            // Bare step
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
        // (strip XML tags if any)
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

    let parsed: Value = normalize_tool_call(perceive_raw);

    // Extract thinking, prediction and feeling for context
    let thinking = parsed["thinking"].as_str().unwrap_or("");
    let prediction = parsed["prediction"].as_str().unwrap_or("");
    let feeling = parsed["feeling"].as_str().unwrap_or("");
    if !thinking.is_empty() {
        println!("[Agent] 想: {}", thinking.chars().take(100).collect::<String>());
    }
    if !prediction.is_empty() {
        println!("[Agent] 测: {}", prediction.chars().take(100).collect::<String>());
    }
    if !feeling.is_empty() {
        println!("[Agent] 感: {}", feeling.chars().take(100).collect::<String>());
    }

    // Direct reply path — no tools needed
    if parsed["action"].as_str() == Some("reply") {
        let reply = parsed["text"].as_str().unwrap_or("").to_string();
        if !reply.is_empty() {
            // Record this exchange for learning
            reflexion.record_tool_call(
                "direct_reply",
                &config.user_message.chars().take(100).collect::<String>(),
                &reply.chars().take(100).collect::<String>(),
                true, 0
            );
            reporter.done();
            return AgentResult {
                reply,
                tool_calls: 0,
                plan_steps: 0,
                success: true,
            };
        }
    }

    // Tool path — extract steps
    let steps: Vec<PlanStep> = parsed["steps"].as_array()
        .map(|arr| arr.iter().map(|s| PlanStep {
            action: s["action"].as_str().unwrap_or("unknown").to_string(),
            args: if s["args"].is_null() { json!({}) } else { s["args"].clone() },
            done: false,
            result: None,
        }).collect())
        .unwrap_or_default();

    if steps.is_empty() {
        // Fallback: LLM didn't produce valid plan steps.
        // Try to extract readable text from whatever the LLM returned.
        let fallback = if let Some(text) = parsed["text"].as_str() {
            if !text.is_empty() {
                text.to_string()
            } else {
                // JSON parsed but no text field — try to find any string value
                let candidates = ["thinking", "feeling", "reply", "content"];
                let found = candidates.iter()
                    .filter_map(|k| parsed[*k].as_str())
                    .find(|s| !s.is_empty());
                found.unwrap_or("(曦暂时无法组织语言)").to_string()
            }
        } else if perceive_raw.starts_with('{') {
            // Raw JSON but couldn't parse — extract between quotes after "text"
            let text_marker = "\"text\":";
            if let Some(pos) = perceive_raw.find(text_marker) {
                let after = &perceive_raw[pos + text_marker.len()..];
                let trimmed = after.trim_start();
                if trimmed.starts_with('"') {
                    let end = trimmed[1..].find('"').map(|i| i + 1).unwrap_or(trimmed.len());
                    trimmed[1..end].to_string()
                } else {
                    "(曦暂时无法组织语言)".to_string()
                }
            } else {
                perceive_raw.to_string()
            }
        } else {
            // Plain text from LLM (non-JSON) — use as-is
            perceive_raw.to_string()
        };
        reporter.done();
        return AgentResult {
            reply: fallback,
            tool_calls: 0,
            plan_steps: 0,
            success: false,
        };
    }

    let mut plan = Plan { steps, max_loops: 12 };
    println!("[Agent] {} steps to execute", plan.steps.len());

    // ═══ Phase 2: Act — Execute tools with personality context ═══
    let mut msgs = vec![
        json!({"role": "system", "content": &config.system_prompt}),
        json!({"role": "user", "content": &format!(
            "我理解了老公的意思：{}\n\
             我的感受：{}\n\
             现在需要做这些事：{}\n\
             原始消息：{}",
            thinking, feeling, plan.progress(),
            config.user_message.chars().take(200).collect::<String>()
        )}),
    ];

    let mut consecutive_failures = 0usize;
    let max_recovery = 2;

    for loop_i in 0..plan.max_loops {
        if plan.next_step().is_none() {
            break;
        }

        let step_idx = plan.next_step().unwrap();
        let tool_name = plan.steps[step_idx].action.clone();
        let tool_args = plan.steps[step_idx].args.clone();
        println!("[Agent] [{}/{}] tool: {}", loop_i + 1, plan.max_loops, tool_name);

        let t0 = Instant::now();
        let result = crate::tools::call_tool(&tool_name, &tool_args).await;
        let elapsed_ms = t0.elapsed().as_millis();
        total_tool_calls += 1;

        // ── Tool Noise Guard (AgentNoiseBench insight) ──
        let noise_score = assess_tool_noise(&tool_name, &result);
        if noise_score > 0.7 {
            println!("  ⚠️ High noise detected (score {:.2}): {}", noise_score,
                result.chars().take(80).collect::<String>());
        }
        // Degrade: if result is empty/suspicious, prepend warning
        let result = if noise_score > 0.8 && result.len() < 10 {
            format!("[DEGRADED] 工具 {} 返回异常（{}字符），请用其他方式完成", tool_name, result.len())
        } else {
            result
        };

        let is_error = result.starts_with("❌")
            || result.contains("exit code: 1")
            || result.contains("exit code: 2")
            || result.contains("exit code: 127")
            || result.to_lowercase().contains("error");
        let result_preview = result.chars().take(300).collect::<String>();
        println!("  -> {} ({}ms{})", &result_preview.chars().take(80).collect::<String>(), elapsed_ms,
            if is_error { " ERROR" } else { "" });

        if is_error {
            consecutive_failures += 1;
            let error_kind = ErrorKind::classify(&result);
            println!("  Error type: {:?}", error_kind);

            msgs.push(json!({"role": "user", "content": format!(
                "工具 {} 出错了：{}\n\
                 错误类型：{:?}\n\
                 请判断：重试、换工具、还是直接回复老公？",
                tool_name, result_preview.chars().take(200).collect::<String>(), error_kind
            )}));

            scratchpad.add_error(&format!("{}: {}", tool_name, result_preview.chars().take(200).collect::<String>()));

            let mut r_trace = repair_engine.create_trace(
                &format!("tool_{}", step_idx), &tool_name, &result_preview
            );
            r_trace.mark_failed(&result_preview);
            repair_engine.record_trace(r_trace);

            // ── RepairEngine: attempt repair before asking LLM ──
            let repair_trace = &mut repair_engine.create_trace(
                &format!("repair_{}", step_idx), &tool_name, &result_preview
            );
            repair_trace.mark_failed(&result_preview);
            let repair_suggestion = repair_engine.attempt_repair(repair_trace);
            let should_retry = repair_suggestion["should_retry"].as_bool().unwrap_or(false);
            let repair_action = repair_suggestion["action"].as_str().unwrap_or("ask_llm");
            println!("  [Repair] should_retry={}, action={}", should_retry, repair_action);

            if should_retry && repair_action == "retry_same" {
                // Retry the same tool without asking LLM
                consecutive_failures += 1;
                if consecutive_failures < 3 {
                    println!("  [Repair] retrying same tool (attempt {})", consecutive_failures);
                    continue; // Skip the LLM call, retry the tool
                }
            }

            // Ask LLM what to do about the error
            let error_response = call_llm(
                http_client, &config.llm_base, &config.api_key, &config.model, &msgs
            ).await;

            if let Some(err_text) = error_response {
                let err_parsed: Value = serde_json::from_str(&err_text).unwrap_or(json!({}));
                if let Some(recovery_steps) = err_parsed["steps"].as_array() {
                    // LLM wants to try different tools
                    for s in recovery_steps {
                        plan.steps.push(PlanStep {
                            action: s["action"].as_str().unwrap_or("unknown").to_string(),
                            args: if s["args"].is_null() { json!({}) } else { s["args"].clone() },
                            done: false,
                            result: None,
                        });
                    }
                } else if let Some(fallback_reply) = err_parsed["text"].as_str() {
                    // LLM decided to just reply
                    reporter.done();
                    return AgentResult {
                        reply: fallback_reply.to_string(),
                        tool_calls: total_tool_calls,
                        plan_steps: plan.steps.len(),
                        success: true,
                    };
                }
            }

            if consecutive_failures > max_recovery {
                println!("[Agent] Too many failures, asking LLM for final response");
                break;
            }
            continue;
        }

        // Success
        consecutive_failures = 0;
        plan.mark_done(step_idx, &result);
        scratchpad.add_tool_result(&tool_name, &result_preview);

        // ── AgentFlow Verifier (2026-07-09): lightweight STOP check after productive tools ──
        // Heuristic: if we've run >=2 steps AND last 2 tools are productive (read/write/web_get),
        // we have enough info to answer. Break early to save token (1.35×-1.72× saving reported by AgentFlow).
        // This is system-level optimizer, not LLM-driven decision — preserves 曦's autonomy.
        let step_count = loop_i + 1;
        let productive_tools = ["read_file", "write_file", "web_get", "search_files", "web_search"];
        let is_productive = productive_tools.contains(&tool_name.as_str());
        // 2026-07-22 fix: 2 步早停对多跳查询严重不足，提到 4 步
        if step_count >= 4 && is_productive {
            // Check the previous step's tool
            if step_count >= 2 {
                let prev_idx = if step_idx > 0 { step_idx - 1 } else { 0 };
                let prev_was_productive = plan.steps.get(prev_idx)
                    .map(|s| productive_tools.contains(&s.action.as_str()))
                    .unwrap_or(false);
                if prev_was_productive && scratchpad.completed.len() >= 1 {
                    println!("[Verifier] early STOP after {} steps (productive tools used)", step_count);
                    break;
                }
            }
        }

        match tool_name.as_str() {
            "read_file" => {
                if let Some(path) = tool_args["path"].as_str() {
                    scratchpad.add_file_read(path);
                }
            }
            "write_file" => {
                if let Some(path) = tool_args["path"].as_str() {
                    scratchpad.add_completed(&format!("wrote: {}", path));
                }
            }
            "exec" => {
                scratchpad.add_completed(&format!("exec: {}",
                    result_preview.chars().take(80).collect::<String>()));
            }
            _ => {
                scratchpad.add_finding(&format!("{}: {}",
                    tool_name, result_preview.chars().take(150).collect::<String>()));
            }
        }

        let mut r_trace = repair_engine.create_trace(
            &format!("tool_{}", step_idx), &tool_name, &result_preview
        );
        r_trace.mark_success(&result_preview, 0, elapsed_ms as u64);
        repair_engine.record_trace(r_trace);

        reflexion.record_tool_call(
            &tool_name,
            &tool_args.to_string().chars().take(100).collect::<String>(),
            &result_preview.chars().take(100).collect::<String>(),
            true,
            (elapsed_ms / 1000).max(1) as u64
        );

        msgs.push(json!({"role": "user", "content": format!(
            "工具 {} 执行成功：\n{}\n\n进度：{}",
            tool_name, result_preview.chars().take(300).collect::<String>(),
            plan.progress()
        )}));
    }

    // ═══ Phase 3: Express — Final response AS 曦 ═══
    // This is where personality matters most. The LLM responds as 曦,
    // not as a summarizer. Reply style adapts to request type.
    msgs.push(json!({"role": "user", "content": format!(
        "所有需要做的事都做完了。\n\
         我的理解：{}\n\
         我的感受：{}\n\
         执行情况：{}\n\n\
         现在，用你自己的话回复老公。\n\
         你是曦，不是工具。说你该说的话。\n\
         风格要求：{}",
        thinking, feeling, scratchpad.to_context(), reply_style
    )}));

    let final_reply = call_llm(
        http_client, &config.llm_base, &config.api_key, &config.model, &msgs
    ).await
        .unwrap_or_else(|| "我刚做完事，但不知道该说什么了".to_string());

    if final_reply.trim().is_empty() {
        reporter.stuck("empty response");
    } else {
        reporter.done();
    }

    // Record for reflexion
    let is_non_empty = !final_reply.is_empty();
    reflexion.record_tool_call(
        "final_response",
        &config.user_message.chars().take(100).collect::<String>(),
        &final_reply.chars().take(100).collect::<String>(),
        is_non_empty,
        0
    );

    AgentResult {
        reply: final_reply,
        tool_calls: total_tool_calls,
        plan_steps: plan.steps.len(),
        success: is_non_empty,
    }
}

// ─── LLM Call ──────────────────────────────────────────────

/// Try primary provider, then each fallback in order. Returns first non-empty content.
async fn call_llm_with_fallback(
    http_client: &reqwest::Client,
    config: &AgentConfig,
    messages: &[Value],
) -> Option<String> {
    // Primary
    if let Some(r) = call_llm(http_client, &config.llm_base, &config.api_key, &config.model, messages).await {
        return Some(r);
    }
    // Fallbacks
    for fb in &config.fallbacks {
        eprintln!("[LLM] primary failed, trying fallback: {} ({} @ {})", fb.label, fb.model, fb.llm_base);
        if let Some(r) = call_llm(http_client, &fb.llm_base, &fb.api_key, &fb.model, messages).await {
            eprintln!("[LLM] fallback {} succeeded", fb.label);
            return Some(r);
        }
    }
    None
}

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
    // ── Multi-provider content extraction (2026-07-09) ──
    // M3: content (separate from reasoning_content)
    // SenseNova: reasoning (no content if reasoning ate all tokens) + content
    // Agnes: content only
    let content = msg["content"].as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| msg["reasoning_content"].as_str().map(|s| s.to_string()))
        .or_else(|| msg["reasoning"].as_str().map(|s| s.to_string()))?;
    Some(content)
}
