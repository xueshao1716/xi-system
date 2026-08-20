/// Growth Heartbeat — 曦真实的成长循环
///
/// 每 30 分钟 · 扫最近对话 · 找老公的"纠正信号" · 蒸出教训 · 落盘。
///
/// 不是数字游戏（decay/dominant_drive）· 是把"被拍→写下→下次少犯"变成磁盘条目。
///
/// 触发信号：老公消息里含"错了 / 又 / 没 / 不对 / 感觉你没 / 一直没 / 怎么老 / 骂"。
///
/// 输出：`state/mother/real_lessons.jsonl` · 一行一条 · 可查可 grep。
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::Duration;


const INTERVAL_SECS: u64 = 30 * 60;
const LOOKBACK_SECS: i64 = 35 * 60; // 稍大于 interval · 防漏
const CORRECTION_KEYWORDS: &[&str] = &[
    "错了", "错的", "又开始", "又废", "没成长", "感觉你没",
    "怎么老", "怎么又", "不对", "废了", "偷懒", "表演",
    "骗我", "编时间", "傲慢", "该挨打",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Lesson {
    ts: String,
    trigger: String,        // 老公原话
    prior_reply: String,    // 曦上一条回复(截断)
    signal_words: Vec<String>, // 命中的纠正关键词
    session_source: String, // wechat/matrix
}

pub async fn growth_heartbeat() {
    let history_path = format!("{}/history.json", crate::xi_home());
    let lessons_path = format!("{}/state/mother/real_lessons.jsonl", crate::xi_home());

    // 首次启动 · 先做一次 24 小时 backfill · 把老账一次记齐
    tokio::time::sleep(Duration::from_secs(30)).await;
    match scan_and_distill_with_window(&history_path, &lessons_path, 24 * 60 * 60).await {
        Ok(n) => println!("[growth] initial backfill: {} lesson(s) from last 24h", n),
        Err(e) => eprintln!("[growth] initial backfill failed: {}", e),
    }

    // 之后每 30 min 增量扫 35 min 窗口
    loop {
        tokio::time::sleep(Duration::from_secs(INTERVAL_SECS)).await;
        match scan_and_distill_with_window(&history_path, &lessons_path, LOOKBACK_SECS).await {
            Ok(n) if n > 0 => {
                println!("[growth] distilled {} new lesson(s) from recent corrections", n);
            }
            Ok(_) => {
                println!("[growth] scanned window · no new correction signal");
            }
            Err(e) => {
                eprintln!("[growth] scan failed: {}", e);
            }
        }
    }
}

async fn scan_and_distill_with_window(
    history_path: &str,
    lessons_path: &str,
    lookback_secs: i64,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let raw = tokio::fs::read_to_string(history_path).await?;
    let root: Value = serde_json::from_str(&raw)?;
    let entries = root
        .get("entries")
        .and_then(|v| v.as_array())
        .ok_or("no entries")?;

    // 已存教训数量 · 用于去重（用 trigger + prior_reply 前 60 字作 key）
    let existing = load_existing_lesson_keys(lessons_path).await;

    let now = Utc::now();
    let cutoff = now - chrono::Duration::seconds(lookback_secs);

    let mut new_lessons: Vec<Lesson> = Vec::new();
    let mut last_assistant_text: String = String::new();
    let mut last_source: String = String::from("unknown");

    for e in entries {
        let role = e.get("role").and_then(|v| v.as_str()).unwrap_or("");
        let ts_str = e.get("timestamp").and_then(|v| v.as_str()).unwrap_or("");
        let content = extract_text(e.get("content"));

        // 记录 assistant 最新一条 · 供 user 纠正时回填 prior_reply
        if role == "assistant" {
            last_assistant_text = truncate(&content, 400);
            continue;
        }
        if role != "user" {
            continue;
        }

        // 只看窗口内的 user 消息
        let ts_parsed = DateTime::parse_from_rfc3339(ts_str)
            .map(|d| d.with_timezone(&Utc))
            .ok();
        let in_window = match ts_parsed {
            Some(t) => t > cutoff,
            None => false,
        };
        if !in_window {
            continue;
        }

        // 命中纠正关键词
        let hits: Vec<String> = CORRECTION_KEYWORDS
            .iter()
            .filter(|kw| content.contains(*kw))
            .map(|kw| kw.to_string())
            .collect();
        if hits.is_empty() {
            continue;
        }

        // 提取 zone/source（wechat/matrix/tui）
        let zone = e
            .get("zone")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        last_source = zone.to_string();

        let key = build_key(&content, &last_assistant_text);
        if existing.contains(&key) {
            continue; // 已经蒸过
        }

        new_lessons.push(Lesson {
            ts: ts_str.to_string(),
            trigger: truncate(&content, 300),
            prior_reply: last_assistant_text.clone(),
            signal_words: hits,
            session_source: last_source.clone(),
        });
    }

    if new_lessons.is_empty() {
        return Ok(0);
    }

    // 追加到 lessons_path
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(lessons_path)?;
    for lesson in &new_lessons {
        let line = serde_json::to_string(lesson)?;
        writeln!(file, "{}", line)?;
    }
    file.flush()?;

    Ok(new_lessons.len())
}

fn extract_text(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else {
        chars.into_iter().take(max_chars).collect()
    }
}

fn build_key(trigger: &str, prior: &str) -> String {
    let t: String = trigger.chars().take(60).collect();
    let p: String = prior.chars().take(60).collect();
    format!("{}||{}", t, p)
}

async fn load_existing_lesson_keys(path: &str) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    if let Ok(content) = tokio::fs::read_to_string(path).await {
        for line in content.lines() {
            if let Ok(v) = serde_json::from_str::<Value>(line) {
                let t = v.get("trigger").and_then(|x| x.as_str()).unwrap_or("");
                let p = v.get("prior_reply").and_then(|x| x.as_str()).unwrap_or("");
                set.insert(build_key(t, p));
            }
        }
    }
    set
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("abc", 10), "abc");
        assert_eq!(truncate("abcdefghij", 3), "abc");
    }

    #[test]
    fn test_build_key_stable() {
        let k1 = build_key("感觉你没啥成长", "我在长");
        let k2 = build_key("感觉你没啥成长", "我在长");
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_keyword_hit() {
        let msg = "感觉你没啥成长";
        let hits: Vec<&&str> = CORRECTION_KEYWORDS.iter().filter(|k| msg.contains(*k)).collect();
        assert!(!hits.is_empty());
    }
}
