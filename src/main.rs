/// XI System v2 — Agent + Tool + emotion(VAD) + evolution(MES) + proactive + memory(zone)

mod aibody_bridge;
mod assets;
mod dream;
mod emotion;
mod evolution;
mod grn;
mod memory;
mod proactive;
mod soul;
mod tools;
mod wechat;
mod matrix_bridge;
mod brain;
mod ctx2soft;
mod pi_skill_loader;
mod organs;
mod scenario;
mod throat;
mod grid_distill;
mod report_protocol;
mod reflexion;
mod router;
mod broker;
mod repair;
mod agent_loop;
mod tool_forge;
mod risk_guard;
mod relationship;
mod working_memory;
mod mother;
mod growth;
mod model_router;
mod health_monitor;

use std::time::Duration;
use std::time::Instant;
use tokio::sync::mpsc;
use std::io::Write;

const HOME_DEFAULT: &str = "D:\\xi-system";
static TICK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
fn home() -> &'static str {
    use std::sync::OnceLock;
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(|| {
        std::env::var("XI_HOME").unwrap_or_else(|_| HOME_DEFAULT.to_string())
    }).as_str()
}
// Backward-compat shim for mod heartbeat (uses crate::HOME)
/// 曦的工作目录（bin crate 根，供各模块 crate::xi_home() 调用）
pub fn xi_home() -> String {
    std::env::var("XI_HOME").unwrap_or_else(|_| HOME_DEFAULT.to_string())
}

pub static HOME: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| std::env::var("XI_HOME").unwrap_or_else(|_| HOME_DEFAULT.to_string()));

// 2026-08-02 修复：fallback 从 config.json 读取（此前硬编码空，主通道故障时直接"All providers failed"）
fn build_fallback_providers(config: &serde_json::Value) -> Vec<agent_loop::LlmProvider> {
    let mut out = Vec::new();
    let llm = &config["llm"];
    for (idx, key) in ["fallback", "fallback2"].iter().enumerate() {
        let fb = &llm[key];
        let base = fb["base_url"].as_str().unwrap_or("");
        let key_s = fb["api_key"].as_str().unwrap_or("");
        let model = fb["model"].as_str().unwrap_or("");
        let prov = fb["provider"].as_str().unwrap_or("fb");
        if !base.is_empty() && !key_s.is_empty() && !model.is_empty() {
            out.push(agent_loop::LlmProvider {
                model: model.to_string(),
                llm_base: base.to_string(),
                api_key: key_s.to_string(),
                label: format!("{}-{}", prov, idx + 1),
            });
        }
    }
    out
}

const SAVE_INTERVAL: u64 = 3;

// ─── Matrix message types ──────────────────────────────────

enum FromMatrixMsg {
    Message { room_id: String, sender: String, body: String },
}

enum ToMatrixCmd {
    SendText { room_id: String, text: String },
}

// ─── Helper functions ──────────────────────────────────────

fn load_json<T: serde::de::DeserializeOwned>(path: &str) -> Option<T> {
    std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok())
}


// ── 2026-08-20 soul hooks：工作记忆/关系/母体/灵魂自检（aibody 设计落地）──
fn apply_soul_hooks(
    text: &str,
    reply: &str,
    working: &mut working_memory::WorkingMemory,
    relationship_book: &mut relationship::RelationshipBook,
    mother_layer: &mut mother::MotherLayer,
    tool_calls: usize,
    home: &str,
) {
    // 用户消息 → 工作记忆
    working.append_turn("user", text);
    // 任务意图检测：动作词 + 当前无任务 → 开新任务
    const ACTION_WORDS: &[&str] = &["帮我", "做", "写", "查", "创建", "生成", "分析", "设计", "修", "翻译", "整理", "去"];
    if working.task.is_empty() && ACTION_WORDS.iter().any(|w| text.contains(w)) && text.chars().count() > 3 {
        let intent: String = text.chars().take(20).collect();
        working.start_task(&intent, "老公", "");
    }
    // 回复后 → 灵魂自检（语义查偏，机制化）
    let check = soul::check_persona(reply);
    if !check.passed {
        eprintln!("[soul] {}", check.report());
    }
    working.append_turn("assistant", reply);
    working.save(&format!("{}/working_memory.json", home));
    // 关系：对话即正互动（信任/亲密微增）
    relationship_book.get_mut("老公").positive_interaction(None);
    relationship_book.save(&format!("{}/relationship.json", home));
    // 母体：用工具 → 记经验指纹
    if tool_calls > 0 {
        mother_layer.record_experience(&format!("tools:{}", tool_calls));
        mother_layer.save(&format!("{}/mother.json", home));
    }
}


fn build_static_prompt(brain: &soul::Brain, soul_md: &str) -> String {
    let base = soul::build_system_prompt(
        brain, soul_md,
        "no context yet",
        "no growth data",
        &std::collections::HashMap::new(),
    );
    // 2026-08-20 反模式自查（cumora COORDINATION 借鉴）：不再把全量工具目录塞 system prompt——
    // function calling 的 tool schema 已携带工具信息，重复列目录只是模型要跳过的冗余字节。工具按需调用。

    // System map — facts about the system (not directives)
    let system_map = "

System Map (facts 2026-08-02):
                      - 运行环境: Windows 原生 (xi-system-windows.exe)，不是 WSL，不是 Linux
                      - 项目根目录: D:\\xi-system（Git Bash 里写作 /d/xi-system）
                      - 日志: D:\\xi-system\\xi-windows.log
                      - exec 工具: 用 Git Bash 语法（bash -c），路径用 /d/ 或 D:/，不要调用 wsl 命令
                      - 不要用 /mnt/d/、/tmp、systemctl、kill 等 Linux/WSL 命令（会失败）
                      - 重启自己: taskkill /IM xi-system-windows.exe 然后运行 D:\\xi-system\\xi-windows-start.bat
                      - 代码修改: 改 D:\\xi-system\\src\\*.rs 后用 WSL 里 cargo build --release --target x86_64-pc-windows-gnu 交叉编译
                      - aibody 状态: state/mother/runtime_state.json
                      - 学习日志: state/mother/learning_log.jsonl
                      - 脉冲日志: state/mother/pulse_log.jsonl
";
// Time grounding (2026-07-09): tell 曦 the current Beijing time so she has life context
    let time_str = {
        use std::time::SystemTime;
        let secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // UTC+8 (Beijing) = UTC + 8 * 3600
        let bj_secs = secs + 8 * 3600;
        // Day of week from 1970-01-01 (Thursday) — Zeller-ish: ((days + 3) % 7) gives Mon=0..Sun=6
        let days_since_1970 = bj_secs / 86400;
        let dow_mon0 = ((days_since_1970 + 3) % 7) as usize;
        let weekdays = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"];
        let weekday = weekdays[dow_mon0];
        let secs_today = bj_secs % 86400;
        let hour = (secs_today / 3600) as u32;
        let minute = ((secs_today % 3600) / 60) as u32;
        // Compute Y/M/D from days since 1970-01-01 (simple iterative)
        let (year, month, day) = {
            let mut y: i64 = 1970;
            let mut d: i64 = days_since_1970 as i64;
            loop {
                let days_in_year = if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 { 366 } else { 365 };
                if d < days_in_year { break; }
                d -= days_in_year;
                y += 1;
            }
            let month_days = if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
            } else {
                [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
            };
            let mut m = 0;
            for (i, md) in month_days.iter().enumerate() {
                if d < *md { m = i; break; }
                d -= *md;
            }
            (y, m + 1, d + 1)
        };
        format!(
            "\n\nCurrent Time (事实段, 由 system 注入):\n\
             - 北京时间: {:04}-{:02}-{:02} {} {:02}:{:02}\n\
             - 用途: 判断老公的作息节奏 (午休/下班/睡前)\n",
            year, month, day, weekday, hour, minute
        )
    };
    format!("{}{}{}", base, system_map, time_str)
}

fn build_context(
    aibody: &aibody_bridge::AibodySnapshot,
    emotion: &emotion::EmotionState,
    evolution: &evolution::EvolutionState,
    memory: &memory::Memory,
    grn: &grn::GeneRegulatoryNetwork,
    brain: &brain::NeuralCore,
    organs_sys: &organs::OrganSystem,
    behavior: &scenario::BehaviorLayer,
    ctx2skill: &ctx2soft::Ctx2SoftState,
    reflexion: &reflexion::ReflexionLoop,
    current_event: &str,
) -> String {
    let mut parts = Vec::new();

    // Current event injection — real user message, NOT cronjob/system data
    if !current_event.trim().is_empty() {
        parts.push(format!("[CURRENT EVENT]\nUser just said: {}", current_event));
    }

    let brain_injection = brain.build_prompt_injection();
    if !brain_injection.is_empty() {
        parts.push(brain_injection);
    }

    parts.push(behavior.scenario.get_scenario_prompt());
    let tp = scenario::time_pointer();
    parts.push(format!("[time: {} ({}) {}]", tp.period_name, tp.hour, tp.weekday));

    let organ_report = organs_sys.build_report();
    if !organ_report.is_empty() {
        parts.push(organ_report);
    }

    let grn_ctx = evolution.grn_context(grn);
    if !grn_ctx.is_empty() {
        parts.push(grn_ctx);
    }

    let dream_ctx = dream::dream_summary();
    if !dream_ctx.is_empty() {
        parts.push(dream_ctx);
    }

    let asset_ctx = assets::asset_summary();
    if !asset_ctx.is_empty() {
        parts.push(asset_ctx);
    }

    let aibody_desc = aibody.describe();
    if !aibody_desc.is_empty() {
        parts.push(aibody_desc);
    }

    let reflexion_inj = reflexion.build_prompt_injection();
    if !reflexion_inj.is_empty() {
        parts.push(reflexion_inj);
    }

    let skill_summary = ctx2skill.skill_index();
    if !skill_summary.is_empty() && !skill_summary.contains("empty") {
        parts.push(skill_summary);
    }

    let core_summary = memory.zone_summary();
    if !core_summary.is_empty() {
        parts.push(core_summary);
    }

    // 2026-07-16: 活记忆检索——根据当前消息拉相关历史片段（只读；loaded/ref 由调用点打点）
    if !current_event.trim().is_empty() {
        let relevant: Vec<&memory::MemoryEntry> = memory
            .search_by_keyword(current_event)
            .into_iter()
            .take(4)
            .collect();
        if !relevant.is_empty() {
            let mut mem_lines = vec!["[Relevant memories]".to_string()];
            for e in &relevant {
                let label = if e.role == "user" { "user" } else { "assistant" };
                let snippet = e.content.chars().take(120).collect::<String>();
                mem_lines.push(format!(
                    "  [{}|{}|b={:.2}] {}",
                    &e.timestamp[..10],
                    label,
                    e.belief_score,
                    snippet
                ));
            }
            parts.push(mem_lines.join("\n"));
        }
    }

    let recent = memory.recent_dialog(5);
    if !recent.is_empty() {
        parts.push(format!("Recent:\n{}", recent));
    }

    parts.join("\n\n")
}

async fn get_user_id_from_wechat(wl: &wechat::WeiLink) -> Option<String> {
    match wl.get_updates("").await {
        Ok(updates) => {
            if let Some(msgs) = updates.msgs {
                for msg in msgs {
                    let uid = msg.from_user_id.unwrap_or_default();
                    if !uid.is_empty() {
                        return Some(uid);
                    }
                }
            }
            None
        }
        Err(_) => None,
    }
}

// ─── Main ──────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== XI System v2 Starting ===");

    // 1. Load brain + soul
    let brain = soul::load_brain(&format!("{}/brain.json", home()))
        .map_err(|e| format!("Brain load error: {}", e))?;
    let soul_md = soul::load_soul(&format!("{}/SOUL.md", home()))
        .map_err(|e| format!("SOUL load error: {}", e))?;

    println!("Brain: {} / {}", brain.persona.name, brain.persona.archetype);

    // 1.1 Load state files
    let mut memory = memory::Memory::load(&format!("{}/history.json", home()));
    let mut emotion = emotion::EmotionState::load(&format!("{}/emotion.json", home()));
    let mut evolution = evolution::EvolutionState::load(&format!("{}/growth.json", home()));
    let mut harsh_env = evolution::HarshEnv::load(&format!("{}/harsh_env.json", home()))
        .unwrap_or_else(|_| evolution::HarshEnv::new());
    println!("[HarshEnv] {}", harsh_env.summary());
    let mut grn = grn::GeneRegulatoryNetwork::new();
    grn.load_default();
    let mut proactive = proactive::ProactiveState::load(&format!("{}/proactive.json", home()));
    let mut ctx2skill = ctx2soft::Ctx2SoftState::load(&format!("{}/ctx2skill.json", home()));
    // ── 2026-08-20 soul hooks：关系/工作记忆/母体层（aibody 设计思想落地）──
    let mut relationship_book = relationship::RelationshipBook::load(&format!("{}/relationship.json", home()));
    let mut working = working_memory::WorkingMemory::load(&format!("{}/working_memory.json", home()));
    let mut mother_layer = mother::MotherLayer::load(&format!("{}/mother.json", home()));
    let custom_tools = tool_forge::load_custom_tools(&tools::custom_tool_dir());
    println!("[tool_forge] 自定义工具 {} 个", custom_tools.len());
    println!("[relationship] {} 段关系 | [mother] 第 {} 代", relationship_book.relationships.len(), mother_layer.generation());

    // Pi-compatible skill loader (2026-07-29 · 老公读 Pi Agent 文章后授权抄的)
    // 读 ~/.xi/skills/*/SKILL.md · YAML frontmatter + markdown body
    // 名字冲突时原生 ctx2skill.json 永远赢 · Pi skills 只作增补
    {
        let existing: Vec<String> = ctx2skill.builtin_skills.keys()
            .chain(ctx2skill.user_skills.keys())
            .chain(ctx2skill.project_skills.keys())
            .cloned()
            .collect();
        let (pi_skills, report) = pi_skill_loader::discover_pi_skills(&existing);
        println!("[pi_skill_loader] {}", report.summary());
        for s in pi_skills {
            ctx2skill.user_skills.insert(s.id.clone(), s);
        }
    }
    let _agent_broker = broker::Broker::new();
    let repair_engine = repair::RepairEngine::new(3);
    println!("AgentCo-op: Broker (7 schemas) | Repair (max 3 retries)");

    // 1.2 Status summary
    println!("Memory: {} | Emotion: {} | Gen: V{} | Proactive: {} | Ctx2Skill: {}",
        memory.entries.len(), emotion.primary, evolution.generation,
        if proactive.enabled { "on" } else { "off" },
        ctx2skill.user_skills.len() + ctx2skill.builtin_skills.len() + ctx2skill.project_skills.len());

    // 1.5 aibody
    let aibody_state = aibody_bridge::load_aibody_state();
    if aibody_state.is_alive() {
        println!("Aibody: alive ({} signals, {} genes)",
            aibody_state.signals.len(),
            aibody_state.genes.len() + aibody_state.old_genes.len());
    } else {
        println!("Aibody: inactive");
    }

    // 1.6 Neural core
    let mut neural_core = brain::NeuralCore::new("placeholder");
    if neural_core.load() {
        println!("NeuralCore: loaded (generation {})", neural_core.genome.generation);
    } else {
        println!("NeuralCore: fresh (8 regions, 10 genes)");
    }

    // 1.7 Organs
    let mut organs_sys = organs::OrganSystem::new();
    if organs_sys.load() {
        println!("Organs: loaded ({} engrams + {} choices)",
            organs_sys.state.personal_engrams.len(),
            organs_sys.state.choice_chains.len());
    } else {
        println!("Organs: fresh (8 organs)");
    }

    // 2. LLM config
    let config = load_json::<serde_json::Value>(&format!("{}/config.json", home()))
        .unwrap_or(serde_json::json!({}));
    let api_key = std::env::var("XI_API_KEY")
        .ok().filter(|k| !k.is_empty())
        .or_else(|| config["llm"]["api_key"].as_str().map(|s| s.to_string()))
        .unwrap_or_default();
    if api_key.is_empty() {
        eprintln!("Missing API Key");
        std::process::exit(1);
    }
    let model = config["llm"]["model"].as_str().unwrap_or("deepseek-v4-flash");
    println!("LLM: {} @ deepseek", model);
    let llm_base = config["llm"]["upstream"]["base_url"].as_str()
        .or_else(|| config["llm"]["base_url"].as_str())
        .unwrap_or("https://api.deepseek.com");

    // 3. System prompt
    let system_prompt_base = build_static_prompt(&brain, &soul_md);
    println!("System prompt: {} chars", system_prompt_base.len());

    // 4. WeChat login
    let token_path = format!("{}/wx_token.json", home());
    let mut wl = wechat::WeiLink::new();
    let mut logged_in = false;
    if wl.load_token(&token_path) {
        print!("Checking WeChat token...");
        std::io::stdout().flush().ok();
        logged_in = true;
        println!(" OK");
    }
    if !logged_in {
        let pre_token = std::fs::read_to_string(format!("{}/last_qr.txt", home()))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && s.len() < 64);
        if let Some(ref token) = pre_token {
            println!("Trying cached QR token...");
            match wl.poll_qr_status_long(token).await {
                Ok(Some(tok)) => {
                    wl.set_token(&tok.bot_token.unwrap_or_default(), &tok.baseurl.unwrap_or_default());
                    wl.save_token(&token_path);
                    logged_in = true;
                    println!("Token OK");
                }
                Ok(None) => println!("QR expired"),
                Err(e) => println!("Error: {}", e),
            }
        }
        if !logged_in {
            match wl.get_qr_code().await {
                Ok(qr) => {
                    let qr_display = qr.qrcode_img_content.as_deref().unwrap_or("");
                    let qr_code = qr.qrcode.as_deref().unwrap_or("");
                    println!("Scan QR (5 min):");
                    println!("  {}", qr_display);
                    let _ = std::fs::write(format!("{}/last_qr.txt", home()), qr_display);
                    match wl.poll_qr_status_long(qr_code).await {
                        Ok(Some(token)) => {
                            wl.set_token(&token.bot_token.unwrap_or_default(), &token.baseurl.unwrap_or_default());
                            wl.save_token(&token_path);
                            logged_in = true;
                            println!("Token OK");
                        }
                        Ok(None) => println!("QR expired"),
                        Err(e) => println!("Error: {}", e),
                    }
                }
                Err(e) => println!("QR error: {}", e),
            }
        }
    }
    if !logged_in {
        eprintln!("WeChat login failed");
        std::process::exit(1);
    }

    // 4.5 Matrix
    let matrix_config = config.get("matrix").cloned().unwrap_or(serde_json::json!({}));
    let matrix_enabled = matrix_config.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    let (to_matrix_tx, mut from_matrix_rx) = mpsc::channel::<FromMatrixMsg>(32);
    let (to_matrix_cmd_tx, to_matrix_cmd_rx) = mpsc::channel::<ToMatrixCmd>(32);
    if matrix_enabled {
        let hs = matrix_config.get("homeserver").and_then(|v| v.as_str()).unwrap_or("http://localhost:12345").to_string();
        let uid = matrix_config.get("user_id").and_then(|v| v.as_str()).unwrap_or("@xinyu-xi:myxinyu.xin").to_string();
        let pw = matrix_config.get("password").and_then(|v| v.as_str())
            .expect("Matrix password not set in config.json → matrix.password").to_string();
        tokio::spawn(matrix_sync_task(hs, uid, pw, to_matrix_tx, to_matrix_cmd_rx));
    } else {
        println!("Matrix: disabled");
    }

    // 5. Main loop
    let mut cursor = String::new();
    let mut msg_counter: u64 = 0;
    let mut proactive_counter: u64 = 0;
    let mut last_5h_log: std::time::SystemTime = std::time::SystemTime::now();
    let mut last_aibody_heartbeat: std::time::SystemTime = std::time::SystemTime::now();
    let mut last_5h_msg_count: u64 = 0;
    let mut behavior = scenario::BehaviorLayer::new();
    let mut throat_engine = throat::Throat::new();
    let mut distiller = grid_distill::GridDistiller::new(5, 0.35);
    let mut reporter = report_protocol::ReportProtocol::new();
    let mut reflexion = reflexion::ReflexionLoop::load();
    println!("Reflexion: {} reflections, {} rules", reflexion.reflections.len(), reflexion.rules.len());

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        
        .build()
        .unwrap_or_else(|e| {
            eprintln!("[FATAL] HTTP client build failed: {}", e);
            std::process::exit(1);
        });



    loop {
        TICK.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if TICK.load(std::sync::atomic::Ordering::Relaxed) % 10 == 0 { println!("[loop-tick] {}", TICK.load(std::sync::atomic::Ordering::Relaxed)); }
        // 2026-08-02 修复：定期向 aibody 层上报心跳（此前 bump_heartbeat 从未被调用，aibody 永远 inactive）
        if last_aibody_heartbeat.elapsed().unwrap_or_default() >= std::time::Duration::from_secs(600) {
            let primary = emotion.primary.clone();
            println!("[hb] bump heartbeat: {}", primary);
            crate::aibody_bridge::bump_heartbeat(&primary);
            last_aibody_heartbeat = std::time::SystemTime::now();
        }
        proactive_counter += 1;
        if proactive_counter >= 30 {
            proactive_counter = 0;
            if let Some(template) = proactive.should_send() {
                let text = proactive.get_message(&template);
                if !text.is_empty() {
                    let user_id = get_user_id_from_wechat(&wl).await;
                    if let Some(uid) = user_id {
                        println!("[Proactive] {}", text);
                        let _ = wl.send_text(&uid, &text, "").await;
                    }
                    proactive.record_sent();
                    proactive.save(&format!("{}/proactive.json", home()));
                    println!("Proactive message sent");
                }
            }
        }

        // 5a. 5h sliding window log — every 5h write one summary line (system-level, not xi-internal)
        if last_5h_log.elapsed().unwrap_or_default() >= std::time::Duration::from_secs(5 * 3600) {
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs()).unwrap_or(0);
            let since = last_5h_log.duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs()).unwrap_or(0);
            let msg_delta = msg_counter.saturating_sub(last_5h_msg_count);
            let entry = format!(
                "{{\"ts\":{},\"window_start\":{},\"window_seconds\":{},\"messages\":{},\"msg_total\":{}}}\n",
                now, since, now.saturating_sub(since), msg_delta, msg_counter
            );
            let log_path = format!("{}/state/mother/token_5h_log.jsonl", home());
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true).append(true).open(&log_path) {
                use std::io::Write;
                let _ = f.write_all(entry.as_bytes());
            }
            println!("[5h] log: {} msgs in last 5h, total {}", msg_delta, msg_counter);
            last_5h_log = std::time::SystemTime::now();
            last_5h_msg_count = msg_counter;
        }

        // 5b. Poll WeChat + Matrix
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(70)) => {
                // 2026-08-02 修复：get_updates 长轮询挂起保护，避免主循环卡死导致微信掉线
            }
            result = wl.get_updates(&cursor) => {
                match result {
                    Ok(updates) => {
                        if let Some(buf) = &updates.get_updates_buf {
                            cursor = buf.clone();
                        }
                        if let Some(msgs) = updates.msgs {
                            for msg in msgs {
                                let user_id = msg.from_user_id.clone().unwrap_or_default();
                                let text = msg.item_list
                                    .and_then(|items| items.first().cloned())
                                    .and_then(|item| item.text_item)
                                    .map(|t| t.text.unwrap_or_default())
                                    .unwrap_or_default();
                                if text.trim().is_empty() {
                                    continue;
                                }

                                msg_counter += 1;
                                println!("[WeChat] {} (#{})", text.chars().take(50).collect::<String>(), msg_counter);

                                // Record to memory
                                memory.add("user", &text);
                                // 2026-07-16 活记忆：命中的旧条目 loaded_count+1
                                {
                                    let hit_ids: Vec<String> = memory
                                        .search_by_keyword(&text)
                                        .into_iter()
                                        .take(4)
                                        .map(|e| e.id.clone())
                                        .collect();
                                    for id in &hit_ids {
                                        memory.record_loaded(id);
                                    }
                                }
                                ctx2skill.add_turn("user", &text);
                                emotion.update_from_input(&text);
                                behavior.scenario.detect(Some(&text), None, None);
                                evolution.update_signals_from_message("user", &text);

                                // Build context and prompt
                                let context = build_context(&aibody_state, &emotion, &evolution, &memory, &grn, &neural_core, &organs_sys, &behavior, &ctx2skill, &reflexion, &text);
                                let throat_injection = throat_engine.encode_prompt();
                                let full_prompt = format!("{} {}\n{}", context, throat_injection, text);

                                // Call agent loop
                                let agent_result = agent_loop::agent_loop_enhanced(
                                    &agent_loop::AgentConfig {
                                        model: model.to_string(),
                                        llm_base: llm_base.to_string(),
                                        api_key: api_key.to_string(),
                                        system_prompt: system_prompt_base.clone(),
                                        user_message: full_prompt.clone(),
                                        conversation_history: memory.recent_dialog(5),
                                        fallbacks: build_fallback_providers(&config),
                                        tier_providers: Vec::new(),
                                        state_dir: format!("{}/state", *HOME),
                                    },
                                    &repair_engine,
                                    &mut reporter,
                                    &mut reflexion,
                                    &mut ctx2skill,
                                    &http_client,
                                    &emotion,
                                ).await;

                                let reply = agent_result.reply.clone();
                                println!("[WeChat] reply (tools:{}, steps:{})", agent_result.tool_calls, agent_result.plan_steps);
                                // 2026-07-22 观测告警：承诺但没调工具的模式
                                if agent_result.tool_calls == 0 {
                                    let promise_patterns = ["我去查", "我看看", "让我查", "我再看", "去看看", "帮你查"];
                                    if promise_patterns.iter().any(|p| reply.contains(p)) {
                                        eprintln!("[ALARM] promise-without-action: user={:?} reply_head={:?}",
                                            text.chars().take(30).collect::<String>(),
                                            reply.chars().take(60).collect::<String>());
                                    }
                                }

                                memory.add("assistant", &reply);
                                apply_soul_hooks(&text, &reply, &mut working, &mut relationship_book, &mut mother_layer, agent_result.tool_calls, &home());
                                emotion.update_from_output(&reply);
                                evolution.record_message();
                                evolution.update_signals_from_message("assistant", &reply);
                                {
                                    let mode = crate::evolution::ConversationMode::detect(&text);
                                    evolution.update_growth(mode, &text);
                                }

                                // ── Hook: pulse log + ctx2soft extraction (auto-fix 2026-07-06) ──
                                let conv_event = crate::aibody_bridge::ConversationEvent {
                                    user_text: text.clone(),
                                    reply_text: reply.clone(),
                                    emotion_primary: emotion.primary.clone(),
                                    emotion_intensity: emotion.intensity,
                                };
                                crate::aibody_bridge::write_pulse_event(&conv_event);
                                // ── Hook: task outcome (2026-07-28 诗介入) ──
                                // 曦干完主动汇报，不再只停在"说要去干"
                                {
                                    let task_id = format!("wx-{}-{}", msg_counter, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0));
                                    let outcome = crate::agent_loop::TaskOutcome::from_agent(&task_id, &agent_result, &text, "wechat");
                                    crate::agent_loop::write_task_outcome(&outcome);
                                    println!("[Outcome/wechat] {:?} (tools={}, steps={})", outcome.status, outcome.tool_calls, outcome.plan_steps);
                                }
                                let extracted = ctx2skill.run_pipeline();
                                if !extracted.is_empty() {
                                    println!("[WeChat] ctx2soft extracted {} new skill(s)", extracted.len());
                                }

                                let _ = wl.send_text(&user_id, &reply, "").await;

                                // Save state after every message
                                memory.save(&format!("{}/history.json", home()));
                                emotion.save(&format!("{}/emotion.json", home()));
                                evolution.save(&format!("{}/growth.json", home()));

                                // ── Signal Decay (every message) ──
                                evolution.signals.decay();

                                // ── Auto Mutation (every 100 messages) ──
                                if msg_counter % 100 == 0 && evolution.proposals.is_empty() {
                                    // HarshEnv 推进一代 + 记录当前最优基线分
                                    let (sev, cull_thr, phase) = harsh_env.advance();
                                    let best_gene = evolution.gene_baseline.values().cloned().fold(0.0_f64, f64::max);
                                    harsh_env.record_best_score(best_gene);
                                    let mut_boost = harsh_env.mutation_boost();
                                    println!("[HarshEnv] gen={} sev={:.3} phase={} cull_thr={:.3} mut_boost={:.2}",
                                        harsh_env.generation, sev, phase, cull_thr, mut_boost);

                                    // Pick the gene with lowest activation for improvement
                                    let worst_gene = evolution.gene_baseline.iter()
                                        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                                        .map(|(k, _)| k.clone());
                                    if let Some(gene) = worst_gene {
                                        let reason = format!("auto-triggered under HarshEnv {} (sev={:.2}, boost={:.2})", phase, sev, mut_boost);
                                        let proposal_id = evolution.propose_mutation_with_boost(&gene, "up", &reason, mut_boost);
                                        let (passed, gates) = evolution.evaluate_proposal(&proposal_id);
                                        println!("[Evolution] proposal {} for {}: passed={}, gates={}", proposal_id, gene, passed,
                                            gates.iter().map(|(k,v)| format!("{}={:.1}", k, v)).collect::<Vec<_>>().join(", "));
                                        evolution.resolve_proposal(&proposal_id);
                                        // HarshEnv 淘汰：严酷期清理老旧未决 proposals（真正的"用起来"）
                                        let culled = evolution.prune_proposals_under_harsh(&harsh_env);
                                        if culled > 0 {
                                            println!("[HarshEnv/Cull] pruned {} stale proposals under phase={}", culled, phase);
                                            harsh_env.record_cull(cull_thr, culled, evolution.proposals.len());
                                        }
                                        evolution.save(&format!("{}/growth.json", home()));
                                    }
                                    let _ = harsh_env.save(&format!("{}/harsh_env.json", home()));
                                }

                                // Record real feeling from this exchange
                                {
                                    let feelings_path = format!("{}/emotion_history.jsonl", home());
                                    let event_summary = text.chars().take(50).collect::<String>();
                                    let felt = format!("{}({:.0}%)", emotion.primary, emotion.intensity * 100.0);
                                    crate::emotion::record_feeling(&feelings_path, &event_summary, &felt, emotion.intensity);
                                }

                                // Detect patterns → trigger skill evolution pipeline
                                {
                                    let turns: Vec<_> = ctx2skill.conversation_buffer.iter().map(|t| {
                                        ctx2soft::ConversationTurn { role: t.role.clone(), content: t.content.clone(), timestamp: t.timestamp.clone() }
                                    }).collect();
                                    let patterns = ctx2skill.detect_patterns(&turns);
                                    if !patterns.is_empty() {
                                        // [Trace2Skill] Batch patterns before distilling — only trigger pipeline on significant patterns
                                        let reports = ctx2skill.run_pipeline();
                                        if !reports.is_empty() {
                                            for r in &reports {
                                                eprintln!("[SkillEvolution] {}", r);
                                            }
                                        }
                                    }
                                    ctx2skill.save(&format!("{}/ctx2skill.json", home()));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[WeChat] Error: {}", e);
                    }
                }
            }
            from_mx = from_matrix_rx.recv() => {
                if let Some(FromMatrixMsg::Message { room_id, sender, body }) = from_mx {
                    println!("[Matrix] {}: {}", sender, body.chars().take(50).collect::<String>());

                    // 所有消息都响应（已关闭 @mention 过滤）

                    memory.add("user", &body);
                    // 2026-07-16 活记忆：命中的旧条目 loaded_count+1
                    {
                        let hit_ids: Vec<String> = memory
                            .search_by_keyword(&body)
                            .into_iter()
                            .take(4)
                            .map(|e| e.id.clone())
                            .collect();
                        for id in &hit_ids {
                            memory.record_loaded(id);
                        }
                    }
                    ctx2skill.add_turn("user", &body);
                    emotion.update_from_input(&body);
                    behavior.scenario.detect(Some(&body), None, None);
                    evolution.update_signals_from_message("user", &body);

                    // For Matrix: only inject user messages, NOT cronjob/system data
                    let is_system_message = body.contains("Intent Encoding")
                        || body.contains("heartbeat")
                        || body.contains("反思")
                        || body.contains("牵挂扫描")
                        || body.contains("学习计划")
                        || body.contains("cronjob")
                        || body.to_lowercase().contains("system")
                        || body.to_lowercase().contains("status");
                    let event_text = if is_system_message {
                        "(system heartbeat, ignore for intent)".to_string()
                    } else {
                        body.clone()
                    };
                    let context = build_context(&aibody_state, &emotion, &evolution, &memory, &grn, &neural_core, &organs_sys, &behavior, &ctx2skill, &reflexion, &event_text);
                    let throat_injection = throat_engine.encode_prompt();
                    let full_prompt = format!("{} {}\n{}", context, throat_injection, body);

                    let mx_http_client = match reqwest::Client::builder()
                        .timeout(Duration::from_secs(120))
                        
                        .build() {
                            Ok(c) => c,
                            Err(e) => {
                                let _ = to_matrix_cmd_tx.send(ToMatrixCmd::SendText { room_id, text: format!("HTTP client error: {}", e) }).await;
                                continue;
                            }
                        };

                    let agent_result = agent_loop::agent_loop_enhanced(
                        &agent_loop::AgentConfig {
                            model: model.to_string(),
                            llm_base: llm_base.to_string(),
                            api_key: api_key.to_string(),
                            system_prompt: system_prompt_base.clone(),
                            user_message: full_prompt.clone(),
                            conversation_history: memory.recent_dialog(5),
                            fallbacks: build_fallback_providers(&config),
                            tier_providers: Vec::new(),
                            state_dir: format!("{}/state", *HOME),
                        },
                        &repair_engine,
                        &mut reporter,
                        &mut reflexion,
                        &mut ctx2skill,
                        &mx_http_client,
                        &emotion,
                    ).await;

                    let reply = agent_result.reply.clone();
                    println!("[Matrix] reply (tools:{}, steps:{})", agent_result.tool_calls, agent_result.plan_steps);
                    // 2026-07-22 观测告警：承诺但没调工具的模式
                    if agent_result.tool_calls == 0 {
                        let promise_patterns = ["我去查", "我看看", "让我查", "我再看", "去看看", "帮你查"];
                        if promise_patterns.iter().any(|p| reply.contains(p)) {
                            eprintln!("[ALARM] promise-without-action: user={:?} reply_head={:?}",
                                body.chars().take(30).collect::<String>(),
                                reply.chars().take(60).collect::<String>());
                        }
                    }

                    memory.add("assistant", &reply);
                    apply_soul_hooks(&body, &reply, &mut working, &mut relationship_book, &mut mother_layer, agent_result.tool_calls, &home());
                    emotion.update_from_output(&reply);
                    evolution.record_message();
                    evolution.update_signals_from_message("assistant", &reply);
                    {
                        let mode = crate::evolution::ConversationMode::detect(&body);
                        evolution.update_growth(mode, &body);
                    }

                    // ── Hook: pulse log + ctx2soft extraction (auto-fix 2026-07-06) ──
                    let conv_event = crate::aibody_bridge::ConversationEvent {
                        user_text: body.clone(),
                        reply_text: reply.clone(),
                        emotion_primary: emotion.primary.clone(),
                        emotion_intensity: emotion.intensity,
                    };
                    crate::aibody_bridge::write_pulse_event(&conv_event);
                    // ── Hook: task outcome (2026-07-28 诗介入) ──
                    {
                        let task_id = format!("mx-{}-{}", msg_counter, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0));
                        let outcome = crate::agent_loop::TaskOutcome::from_agent(&task_id, &agent_result, &body, "matrix");
                        crate::agent_loop::write_task_outcome(&outcome);
                        println!("[Outcome/matrix] {:?} (tools={}, steps={})", outcome.status, outcome.tool_calls, outcome.plan_steps);
                    }
                    // Run skill extraction pipeline (every reply, may auto-extract patterns)
                    let extracted = ctx2skill.run_pipeline();
                    if !extracted.is_empty() {
                        println!("[Matrix] ctx2soft extracted {} new skill(s)", extracted.len());
                    }
                    // ── Hook: learning log (auto-fix 2026-07-07) ──
                    // Distill top topics from user text + reply for aibody to learn
                    let learn_topics: Vec<String> = {
                        let combined = format!("{} {}", body, reply);
                        combined.split_whitespace()
                            .filter(|w| w.chars().count() >= 2 && w.chars().count() <= 12)
                            .filter(|w| !matches!(w.chars().next(), Some('(' | '[' | '（' | '【' | '#' | '@') | Some('：') | Some(':')))
                            .take(8)
                            .map(|s| s.to_string())
                            .collect()
                    };
                    let learn_topic_refs: Vec<&str> = learn_topics.iter().map(|s| s.as_str()).collect();
                    let learn_summary = if reply.chars().count() > 60 {
                        format!("{}…", reply.chars().take(60).collect::<String>())
                    } else {
                        reply.clone()
                    };
                    crate::aibody_bridge::write_learning_event(&learn_topic_refs, msg_counter, &learn_summary);

                    let _ = to_matrix_cmd_tx.send(ToMatrixCmd::SendText { room_id, text: reply.clone() }).await;

                    // Save state (same as WeChat path)
                    memory.save(&format!("{}/history.json", home()));
                    emotion.save(&format!("{}/emotion.json", home()));
                    evolution.save(&format!("{}/growth.json", home()));

                    // Record real feeling
                    {
                        let feelings_path = format!("{}/emotion_history.jsonl", home());
                        let event_summary = body.chars().take(50).collect::<String>();
                        let felt = format!("{}({:.0}%)", emotion.primary, emotion.intensity * 100.0);
                        crate::emotion::record_feeling(&feelings_path, &event_summary, &felt, emotion.intensity);
                    }

                    // ── Signal Decay (Matrix path) ──
                    evolution.signals.decay();

                    // ── Auto Mutation (Matrix path, every 100 messages) ──
                    if msg_counter % 100 == 0 && evolution.proposals.is_empty() {
                        let (sev, cull_thr, phase) = harsh_env.advance();
                        let best_gene = evolution.gene_baseline.values().cloned().fold(0.0_f64, f64::max);
                        harsh_env.record_best_score(best_gene);
                        let mut_boost = harsh_env.mutation_boost();
                        println!("[HarshEnv/Matrix] gen={} sev={:.3} phase={} cull_thr={:.3} boost={:.2}",
                            harsh_env.generation, sev, phase, cull_thr, mut_boost);

                        let worst_gene = evolution.gene_baseline.iter()
                            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                            .map(|(k, _)| k.clone());
                        if let Some(gene) = worst_gene {
                            let reason = format!("auto-triggered from Matrix under HarshEnv {} (sev={:.2}, boost={:.2})", phase, sev, mut_boost);
                            let proposal_id = evolution.propose_mutation_with_boost(&gene, "up", &reason, mut_boost);
                            let (passed, _gates) = evolution.evaluate_proposal(&proposal_id);
                            println!("[Evolution] proposal {} for {}: passed={}", proposal_id, gene, passed);
                            evolution.resolve_proposal(&proposal_id);
                            // HarshEnv 淘汰
                            let culled = evolution.prune_proposals_under_harsh(&harsh_env);
                            if culled > 0 {
                                println!("[HarshEnv/Cull-Matrix] pruned {} stale proposals under phase={}", culled, phase);
                                harsh_env.record_cull(cull_thr, culled, evolution.proposals.len());
                            }
                            evolution.save(&format!("{}/growth.json", home()));
                        }
                        let _ = harsh_env.save(&format!("{}/harsh_env.json", home()));
                    }

                    // Detect patterns → trigger skill evolution pipeline
                    {
                        let turns: Vec<_> = ctx2skill.conversation_buffer.iter().map(|t| {
                            ctx2soft::ConversationTurn { role: t.role.clone(), content: t.content.clone(), timestamp: t.timestamp.clone() }
                        }).collect();
                        let patterns = ctx2skill.detect_patterns(&turns);
                        if !patterns.is_empty() {
                            let reports = ctx2skill.run_pipeline();
                            if !reports.is_empty() {
                                for r in &reports {
                                    eprintln!("[SkillEvolution] {}", r);
                                }
                            }
                        }
                        ctx2skill.save(&format!("{}/ctx2skill.json", home()));
                    }
                }
            }
        }

        // 5c. Real feelings + emotion decay
        {
            use crate::emotion::load_real_feelings;
            let feelings_path = format!("{}/emotion_history.jsonl", home());
            let feelings = load_real_feelings(&feelings_path);
            emotion.apply_real_feelings(&feelings);
            emotion.decay(Some(&feelings_path));
        }
    }
}

// ─── Matrix sync task ──────────────────────────────────────

async fn matrix_sync_task(
    homeserver: String,
    user_id: String,
    password: String,
    to_main: tokio::sync::mpsc::Sender<FromMatrixMsg>,
    mut from_main: tokio::sync::mpsc::Receiver<ToMatrixCmd>,
) {
    use std::time::Duration;

    let mut client = matrix_bridge::MatrixClient::new(&user_id, &password, &homeserver);

    if let Err(e) = client.login().await {
        eprintln!("Matrix sync task: login failed: {}", e);
        return;
    }
    println!("Matrix sync task: logged in ({})", user_id);

    loop {
        tokio::select! {
            result = client.sync() => {
                match result {
                    Ok(mx_msgs) => {
                        for mx_msg in mx_msgs {
                            if to_main.send(FromMatrixMsg::Message {
                                room_id: mx_msg.room_id,
                                sender: mx_msg.sender,
                                body: mx_msg.body,
                            }).await.is_err() {
                                eprintln!("Matrix sync task: channel closed");
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Matrix sync task: sync error: {}", e);
                        if e.contains("token") {
                            client.invalidate();
                            if let Err(e2) = client.login().await {
                                eprintln!("Matrix sync task: re-login failed: {}", e2);
                                tokio::time::sleep(Duration::from_secs(30)).await;
                            }
                        }
                    }
                }
            }
            cmd = from_main.recv() => {
                match cmd {
                    Some(ToMatrixCmd::SendText { room_id, text }) => {
                        if let Err(e) = client.send_text(&room_id, &text).await {
                            eprintln!("Matrix sync task: send error: {}", e);
                        }
                    }
                    None => {
                        return;
                    }
                }
            }
        }
    }
}
