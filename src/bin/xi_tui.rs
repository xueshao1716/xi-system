// bin/xi_tui.rs —— 曦的交互入口（TUI，2026-08-21）
// 直接跟曦对话（走 agent_loop）+ 看状态（情绪/进化/记忆/提案/纠正）。
// 用法：cargo run --bin xi_tui
// 命令：
//   /status      刷新状态面板
//   /proposals   看自我改进提案
//   /corrections 看纠正记忆
//   /quit        退出
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
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
use std::io;
use tokio::sync::mpsc;

// 简化：不走完整 agent_loop（需 repair/reflexion 等 7 依赖），直接调 LLM 的轻量回复 + 状态注入
// （agent_loop_enhanced 需要完整引擎上下文，TUI 里先做"直连对话+状态感知"，工具循环后续接）
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let home = std::env::var("XI_HOME").unwrap_or_else(|_| "C:/xi-home".to_string());
    // 读配置
    let config: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(format!("{}/config.json", home)).unwrap_or_else(|_| "{}".into())
    ).unwrap_or_else(|_| serde_json::json!({}));
    let llm_base = config["llm"]["base_url"].as_str()
        .or_else(|| config["llm"]["base"].as_str()).unwrap_or("").to_string();
    let api_key = config["llm"]["api_key"].as_str().unwrap_or("").to_string();
    let model = config["llm"]["model"].as_str().unwrap_or("").to_string();

    if llm_base.is_empty() || api_key.is_empty() {
        eprintln!("[xi_tui] 配置缺失: 需要 {}/config.json 的 llm.base / llm.api_key / llm.model", home);
        std::process::exit(1);
    }

    // 消息通道：UI 线程 → agent 线程
    let (tx, mut rx) = mpsc::channel::<String>(32);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut chat_log: Vec<(String, String)> = Vec::new(); // (role, text)
    let mut input = String::new();
    let mut status = build_status(&home);
    let mut pending: Option<String> = None; // 正在等 agent 回复的消息

    let result = run_ui(&mut terminal, &mut rx, &tx, &mut chat_log, &mut input, &mut status, &mut pending, &home, &llm_base, &api_key, &model);

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
    input: &mut String,
    status: &mut String,
    pending: &mut Option<String>,
    home: &str, llm_base: &str, api_key: &str, model: &str,
) -> Result<(), Box<dyn std::error::Error>>
where <B as Backend>::Error: 'static
{
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
                match key.code {
                    KeyCode::Char(c) => { input.push(c); }
                    KeyCode::Backspace => { input.pop(); }
                    KeyCode::Enter => {
                        let text = input.trim().to_string();
                        if text.is_empty() { continue; }
                        input.clear();
                        match text.as_str() {
                            "/quit" | "/exit" => break,
                            "/status" => { *status = build_status(home); }
                            "/proposals" => { *status = build_proposals(home); }
                            "/corrections" => { *status = build_corrections(home); }
                            _ => {
                                // 异步调 agent（不阻塞 UI）
                                let tx2 = tx.clone();
                                let q = text.clone();
                                let (b, k, m, h) = (llm_base.to_string(), api_key.to_string(), model.to_string(), home.to_string());
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
                    KeyCode::Esc => break,
                    _ => {}
                }
            }
        }
    }
    Ok(())
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
    // 情绪
    if let Ok(e) = serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(format!("{}/state/emotion.json", home)).unwrap_or_default()) {
        let valence = e["valence"].as_f64().unwrap_or(0.0);
        let label = if valence > 0.3 { "积极" } else if valence < -0.3 { "低落" } else { "平稳" };
        lines.push(format!("情绪: {} (valence {:.2})", label, valence));
    } else { lines.push("情绪: 无数据".into()); }
    // 记忆
    if let Ok(m) = serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(format!("{}/state/memory.json", home)).unwrap_or_default()) {
        let n = m["entries"].as_array().map(|a| a.len()).unwrap_or(0);
        lines.push(format!("记忆: {} 条", n));
    } else { lines.push("记忆: 无数据".into()); }
    // 每日判断
    if let Ok(dj) = std::fs::read_to_string(format!("{}/state/daily_judgment.json", home)) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&dj) {
            lines.push(format!("今日判断: {}", v["judgment"].as_str().unwrap_or("").chars().take(40).collect::<String>()));
        }
    }
    // 改进提案
    if let Ok(content) = std::fs::read_to_string(format!("{}/state/improvement_proposals.jsonl", home)) {
        let open = content.lines().filter(|l| l.contains("\"open\"")).count();
        lines.push(format!("改进提案: {} 条 open", open));
    }
    // 纠正记忆
    if let Ok(content) = std::fs::read_to_string(format!("{}/state/corrections.jsonl", home)) {
        let n = content.lines().filter(|l| l.contains("\"active\":true")).count();
        lines.push(format!("纠正记忆: {} 条生效", n));
    }
    lines.join("\n")
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

fn draw(f: &mut Frame, chat_log: &[(String, String)], input: &str, status: &str, pending: &Option<String>) {
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

    // 输入
    let inp = Paragraph::new(input.clone()).block(Block::default().borders(Borders::ALL).title(" 输入 (Esc 退出) ")).style(Style::default().fg(Color::Yellow));
    f.render_widget(inp, chunks[3]);
}
