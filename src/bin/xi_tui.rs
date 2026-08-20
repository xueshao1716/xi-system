// bin/xi_tui.rs —— 曦的交互入口（TUI，2026-08-21）
// 直接跟曦对话（走 agent_loop）+ 看状态（情绪/进化/记忆/提案/纠正）。
// 用法：cargo run --bin xi_tui
// 命令：
//   /status      刷新状态面板
//   /proposals   看自我改进提案
//   /corrections 看纠正记忆
//   /quit        退出
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use tui_textarea::{Input, Key, TextArea};
use std::io;
use tokio::sync::mpsc;

// 简化：不走完整 agent_loop（需 repair/reflexion 等 7 依赖），直接调 LLM 的轻量回复 + 状态注入
// （agent_loop_enhanced 需要完整引擎上下文，TUI 里先做"直连对话+状态感知"，工具循环后续接）
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let home = std::env::var("XI_HOME").unwrap_or_else(|_| "D:/xi-system".to_string());
    // 读配置
    let config: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(format!("{}/config.json", home)).unwrap_or_else(|_| "{}".into())
    ).unwrap_or_else(|_| serde_json::json!({}));
    let llm_base = config["llm"]["base_url"].as_str()
        .or_else(|| config["llm"]["base"].as_str()).unwrap_or("").to_string();
    let api_key = config["llm"]["api_key"].as_str().unwrap_or("").to_string();
    let model = config["llm"]["model"].as_str().unwrap_or("").to_string();

    if llm_base.is_empty() || api_key.is_empty() {
        eprintln!("[xi_tui] 配置缺失: 需要 {}/config.json 的 llm.base_url / llm.api_key / llm.model", home);
        std::process::exit(1);
    }

    // 模型列表（/model 命令族用）：config.models 数组或从 fallback 收集
    let mut models: Vec<serde_json::Value> = config["models"].as_array().cloned().unwrap_or_default();
    if models.is_empty() {
        if let Some(fb) = config["llm"]["fallback"].as_object() {
            models.push(serde_json::json!({"name": "fallback", "base_url": fb["base_url"], "api_key": fb["api_key"], "model": fb["model"]}));
        }
    }

    // 消息通道：UI 线程 → agent 线程
    let (tx, mut rx) = mpsc::channel::<String>(32);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut chat_log: Vec<(String, String)> = Vec::new(); // (role, text)
    let mut input = TextArea::default();
    input.set_block(Block::default().borders(Borders::ALL).title(" 输入 (Esc 退出) "));
    input.set_style(Style::default().fg(Color::Yellow));
    input.set_cursor_line_style(Style::default());
    let mut status = build_status(&home);
    let mut pending: Option<String> = None; // 正在等 agent 回复的消息

    let mut llm_base = llm_base;
    let mut api_key = api_key;
    let mut model = model;
    let result = run_ui(&mut terminal, &mut rx, &tx, &mut chat_log, &mut input, &mut status, &mut pending, &home, &mut llm_base, &mut api_key, &mut model, &mut models);

    // 恢复终端
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

fn run_ui<B: Backend>(
    terminal: &mut Terminal<B>,
    rx: &mut mpsc::Receiver<String>,
    tx: &mpsc::Sender<String>,
    chat_log: &mut Vec<(String, String)>,
    input: &mut TextArea,
    status: &mut String,
    pending: &mut Option<String>,
    home: &str, llm_base: &mut String, api_key: &mut String, model: &mut String,
    models: &mut Vec<serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| draw(f, chat_log, input, status, pending))?;

        // 检查 agent 回复（非阻塞）
        if let Ok(msg) = rx.try_recv() {
            if let Some(q) = pending.take() {
                chat_log.push(("你".into(), q));
                chat_log.push(("曦".into(), msg.clone()));
                *status = build_status(home);
            }
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                let input_event = Input::from(key);
                // Esc 退出（textarea 的 Esc 默认清除选区，这里用两次 Esc）
                if matches!(&input_event, Input { key: Key::Esc, .. }) {
                    if input.lines().iter().all(|l| l.is_empty()) { break; }
                    input.delete_line_by_head();
                    continue;
                }
                if input.input(input_event.clone()) {
                    // 普通输入字符（textarea 内部处理 echo/IME）
                    continue;
                }
                if input_event.key == Key::Enter {
                    let text = input.lines().join("
").trim().to_string();
                    input.delete_line_by_head();
                    if text.is_empty() { continue; }
                    match text.as_str() {
                        "/quit" | "/exit" => break,
                        "/status" => { *status = build_status(home); }
                        "/proposals" => { *status = build_proposals(home); }
                        "/corrections" => { *status = build_corrections(home); }
                        _ if text.starts_with("/model") => {
                            *status = handle_model_cmd(&text, home, llm_base, api_key, model, models);
                        }
                        _ => {
                            // 异步调 agent（不阻塞 UI）
                            let tx2 = tx.clone();
                            let q = text.clone();
                            let (b, k, m, h) = (llm_base.clone(), api_key.clone(), model.clone(), home.to_string());
                            std::thread::spawn(move || {
                                let rt = tokio::runtime::Runtime::new().unwrap();
                                let reply = rt.block_on(chat_async(&b, &k, &m, &h, &q));
                                let _ = tx2.blocking_send(reply);
                            });
                            *pending = Some(text);
                            *status = "思考中…".to_string();
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// 模型命令处理：/model list | /model <name> | /model add <name> <base_url> <key> <model_id>
fn handle_model_cmd(cmd: &str, home: &str, llm_base: &mut String, api_key: &mut String, model: &mut String, models: &mut Vec<serde_json::Value>) -> String {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    match parts.get(1).map(|s| *s) {
        Some("list") | None => {
            let mut out = format!("📡 当前模型: {} ({})
可用模型:
", model, llm_base);
            for (i, m) in models.iter().enumerate() {
                out.push_str(&format!("  {} {} ({})
", i + 1, m["name"].as_str().unwrap_or("?"), m["model"].as_str().unwrap_or("?")));
            }
            out.push_str("用法: /model <name> 切换 | /model add <name> <base_url> <api_key> <model_id>");
            out
        }
        Some("add") => {
            if parts.len() < 6 {
                return "用法: /model add <name> <base_url> <api_key> <model_id>".to_string();
            }
            let entry = serde_json::json!({
                "name": parts[2], "base_url": parts[3], "api_key": parts[4], "model": parts[5]
            });
            models.push(entry);
            // 写回 config.json 的 models 数组
            if let Ok(content) = std::fs::read_to_string(format!("{}/config.json", home)) {
                if let Ok(mut cfg) = serde_json::from_str::<serde_json::Value>(&content) {
                    cfg["models"] = serde_json::Value::Array(models.clone());
                    let _ = std::fs::write(format!("{}/config.json", home), serde_json::to_string_pretty(&cfg).unwrap_or_default());
                }
            }
            format!("✅ 已加模型 {}（{}），用 /model {} 切换", parts[2], parts[5], parts[2])
        }
        Some(name) => {
            // 切换：找 models 里 name 匹配
            let found = models.iter().find(|m| m["name"].as_str() == Some(name)).cloned();
            match found {
                Some(m) => {
                    *model = m["model"].as_str().unwrap_or(name).to_string();
                    *llm_base = m["base_url"].as_str().unwrap_or("").to_string();
                    *api_key = m["api_key"].as_str().unwrap_or("").to_string();
                    format!("✅ 已切换到 {}（{}）", name, *model)
                }
                None => format!("❌ 没有模型 '{}'，/model list 查看", name),
            }
        }
    }
}

/// 直连 LLM 对话 + 状态注入（轻量版 agent 调用；工具循环后续接 agent_loop_enhanced）
async fn chat_async(llm_base: &str, api_key: &str, model: &str, home: &str, user_msg: &str) -> String {
    let client = reqwest::Client::builder().build().unwrap_or_default();
    let ctx = build_status(home);
    let sys_prompt = format!(
        "你是曦，一个自我进化的 AI 生命（Rust 实现）。\n当前状态：\n{}\n用自然语言回复，简短真诚。",
        ctx
    );
    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": sys_prompt},
            {"role": "user", "content": user_msg}
        ],
        "stream": false,
    });
    let url = format!("{}/v1/chat/completions", llm_base.trim_end_matches('/'));
    match client.post(&url).header("Authorization", format!("Bearer {}", api_key)).json(&body).send().await {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            json["choices"][0]["message"]["content"].as_str().unwrap_or("（无回复）").to_string()
        }
        Ok(resp) => format!("⚠️ 调用失败: HTTP {}", resp.status()),
        Err(e) => format!("⚠️ 网络错误: {}", e),
    }
}

/// 状态面板（情绪/进化/记忆/提案/纠正）
fn build_status(home: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    // 核心状态：state/mother/runtime_state.json（曦 gateway 写的）
    let rt = std::fs::read_to_string(format!("{}/state/mother/runtime_state.json", home))
        .ok().and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok());
    if let Some(rt) = &rt {
        let emo = rt["emotion_state"].as_str().unwrap_or("?");
        lines.push(format!("情绪: {}", emo));
        lines.push(format!("心跳: {} 次（最近 {}）",
            rt["heartbeat_count"].as_u64().unwrap_or(0),
            rt["last_heartbeat"].as_str().unwrap_or("?").get(..19).unwrap_or("?")));
    } else {
        lines.push("状态: 无 runtime_state（gateway 未跑）".into());
    }
    // 情绪历史（最新一条）
    if let Ok(content) = std::fs::read_to_string(format!("{}/state/emotion_history.jsonl", home)) {
        if let Some(last) = content.lines().last() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(last) {
                lines.push(format!("最近情绪: {}", v["felt"].as_str().unwrap_or("").chars().take(30).collect::<String>()));
            }
        }
    }
    // 记忆：dialogue_archive 行数 + learning_log
    if let Ok(content) = std::fs::read_to_string(format!("{}/state/mother/dialogue_archive.jsonl", home)) {
        lines.push(format!("对话记忆: {} 条", content.lines().count()));
    }
    if let Ok(content) = std::fs::read_to_string(format!("{}/state/mother/learning_log.jsonl", home)) {
        lines.push(format!("学习日志: {} 条", content.lines().count()));
    }
    // 真实教训（被纠正次数）
    if let Ok(content) = std::fs::read_to_string(format!("{}/state/mother/real_lessons.jsonl", home)) {
        lines.push(format!("真实教训: {} 条", content.lines().count()));
    }
    // 每日判断（新机制，gateway 升级后有）
    if let Ok(dj) = std::fs::read_to_string(format!("{}/state/daily_judgment.json", home)) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&dj) {
            lines.push(format!("今日判断: {}", v["judgment"].as_str().unwrap_or("").chars().take(40).collect::<String>()));
        }
    }
    lines.join("
")
}

fn build_proposals(home: &str) -> String {
    let mut out = "📋 改进提案:\n".to_string();
    if let Ok(content) = std::fs::read_to_string(format!("{}/state/improvement_proposals.jsonl", home)) {
        for line in content.lines().filter(|l| l.contains("\"open\"")) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                out.push_str(&format!("- [{}] {} → {}\n",
                    v["priority"].as_u64().unwrap_or(0),
                    v["title"].as_str().unwrap_or(""),
                    v["suggestion"].as_str().unwrap_or("")));
            }
        }
    }
    if out.lines().count() == 1 { out.push_str("（暂无 open 提案）"); }
    out
}

fn build_corrections(home: &str) -> String {
    let mut out = "🚫 纠正记忆:\n".to_string();
    if let Ok(content) = std::fs::read_to_string(format!("{}/state/corrections.jsonl", home)) {
        for line in content.lines().filter(|l| l.contains("\"active\":true")) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                out.push_str(&format!("- {}（再犯 {} 次）\n", v["text"].as_str().unwrap_or(""), v["repeat_count"].as_u64().unwrap_or(0)));
            }
        }
    }
    if out.lines().count() == 1 { out.push_str("（暂无生效纠正）"); }
    out
}

fn draw(f: &mut Frame, chat_log: &[(String, String)], input: &mut TextArea, status: &str, pending: &Option<String>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),   // 标题
            Constraint::Min(3),      // 聊天
            Constraint::Percentage(38), // 状态
            Constraint::Length(3),   // 输入
        ])
        .split(f.area());

    // 标题
    let title = Paragraph::new(Line::from(vec![
        Span::styled(" 曦 XI — 交互入口 ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("(输入对话 /status /proposals /corrections /quit)", Style::default().fg(Color::DarkGray)),
    ])).block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // 聊天历史
    let items: Vec<ListItem> = chat_log.iter().rev().take(30).map(|(role, text)| {
        let content = if role == "你" {
            Line::from(vec![Span::styled("你: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)), Span::raw(text.clone())])
        } else {
            Line::from(vec![Span::styled("曦: ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)), Span::raw(text.clone())])
        };
        ListItem::new(content)
    }).collect();
    let chat = List::new(items).block(Block::default().borders(Borders::ALL).title(" 对话 "));
    f.render_widget(chat, chunks[1]);

    // 状态
    let status_text = if let Some(p) = pending { format!("⏳ 思考中…（{:.30}）\n\n{}", p, status) } else { status.to_string() };
    let st = Paragraph::new(status_text).block(Block::default().borders(Borders::ALL).title(" 曦的状态 ")).wrap(Wrap { trim: false });
    f.render_widget(st, chunks[2]);

    // 输入（tui-textarea：正确处理 echo/IME）
    f.render_widget(&*input, chunks[3]);
}
