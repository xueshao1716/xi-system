/// TokenJuice ___ token ______
///
/// __________?OpenHuman TokenJuice ____?80% __________?/// - ____________ token _________
/// - ______________ore > Work > General > Episode
/// - ___________________?/// - CJK ____________1 _?_?2 token______ 1 _?_?1.3 token ____?/// - _______________________________________
///
/// ____?/// - crate::memory::{Memory, MemoryEntry, MemoryZone}

use crate::memory::{Memory, MemoryEntry, MemoryZone};

/// CJK token ____________ 2 token/___ASCII _?0.3 token/___
pub fn estimate_tokens(text: &str) -> usize {
    let mut total = 0usize;
    for ch in text.chars() {
        if ch >= '\u{4e00}' && ch <= '\u{9fff}' {
            total += 2; // CJK ___________________?_?2 token
        } else if ch >= '\u{3040}' && ch <= '\u{30ff}' {
            total += 2; // ____
        } else if ch >= '\u{ac00}' && ch <= '\u{d7af}' {
            total += 2; // ___
        } else if ch.is_ascii_alphanumeric() {
            total += 1; // ASCII _______?0.3____?1 ______
        } else if ch.is_ascii_whitespace() {
            total += 0; // ______
        } else {
            total += 1;
        }
    }
    total
}

/// _________________ max_chars ____?
fn compress_entry(entry: &MemoryEntry, max_chars: usize) -> String {
    let content = &entry.content;
    if content.chars().count() <= max_chars {
        return content.clone();
    }

    // ___ 60% + _?20%______________?
    let total = content.chars().count();
    let head_len = (max_chars * 6 / 10).max(10).min(total);
    let tail_len = (max_chars * 2 / 10).max(5).min(total.saturating_sub(head_len));

    let head: String = content.chars().take(head_len).collect();
    let tail: String = content.chars().skip(total.saturating_sub(tail_len)).collect();

    format!("{}[...{}...]{}", head, total - head_len - tail_len, tail)
}

/// ________________?#[derive(Debug)]
pub struct PrioritizedOutput {
    /// ___________________________
    pub context_text: String,
    /// _____________?
    pub compressed_count: usize,
    /// ___ token ___
    pub raw_tokens: usize,
    /// ____?token ___
    pub compressed_tokens: usize,
    /// ______
    pub savings_pct: f64,
}

/// _____________________ token __________?///
/// ________one ____?> effectiveness > ______?/// __________________________________?///
/// - `memory`: ______
/// - `budget`: token _____
/// - `expand_keywords`: ______________________________________?
fn select_for_injection(
    memory: &Memory,
    budget: usize,
    expand_keywords: &[String],
) -> PrioritizedOutput {
    let active: Vec<&MemoryEntry> = memory.active_entries();

    // _____riority ___ > effectiveness ___ > _______?
    let mut scored: Vec<(f64, &MemoryEntry)> = active
        .iter()
        .map(|&entry| {
            let priority_score = entry.zone.priority() as f64 * 25.0; // Core=100, Work=75, General=50, Episode=25

            let eff_score = if entry.loaded_count > 0 {
                (entry.referenced_count as f64 / entry.loaded_count as f64).min(1.0) * 20.0
            } else {
                0.0
            };

            // ___________________?
            let freshness = match chrono::DateTime::parse_from_rfc3339(&entry.timestamp) {
                Ok(dt) => {
                    let now = chrono::Utc::now();
                    let dt_utc = dt.with_timezone(&chrono::Utc);
                    let age_hours = (now - dt_utc).num_hours().max(0) as f64;
                    // _______24__________
                    (24.0 / (24.0 + age_hours)) * 15.0
                }
                Err(_) => 0.0,
            };

            // __________?
            let keyword_bonus = if expand_keywords
                .iter()
                .any(|kw| entry.content.contains(kw.as_str()))
            {
                50.0
            } else {
                0.0
            };

            let total = priority_score + eff_score + freshness + keyword_bonus;
            (total, entry)
        })
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // _______________________________?
    let mut context_lines = Vec::new();
    let mut compressed = 0;
    let mut raw_total = 0usize;
    let mut used = 0usize;
    let mut cutoff = false;

    for (_score, entry) in &scored {
        let raw_tokens = estimate_tokens(&entry.content);
        raw_total += raw_tokens;

        if cutoff {
            // _____________?
            continue;
        }

        let should_expand = expand_keywords
            .iter()
            .any(|kw| entry.content.contains(kw.as_str()));
        let zone_label = entry.zone.as_str();

        if should_expand {
            // _______________
            let line = format!("[{} | {}] {}", zone_label, entry.id, entry.content);
            let line_tokens = estimate_tokens(&line);
            if used + line_tokens <= budget {
                context_lines.push(line);
                used += line_tokens;
            } else {
                // ___________________?
                let compressed_body = compress_entry(entry, 80);
                let line = format!("[{} | {}] [___] {}", zone_label, entry.id, compressed_body);
                let line_tokens = estimate_tokens(&line);
                if used + line_tokens <= budget {
                    context_lines.push(line);
                    used += line_tokens;
                    compressed += 1;
                }
                // _________
            }
        } else {
            let line_tokens = raw_tokens + zone_label.len() + entry.id.len() + 10;
            if used + line_tokens <= budget {
                // _________
                let line = format!("[{} | {}] {}", zone_label, entry.id, entry.content);
                context_lines.push(line);
                used += line_tokens;
            } else if compressed < 3 {
                // ______________________?
                let compressed_body = compress_entry(entry, 60);
                let line = format!("[{} | {}] [___] {}", zone_label, entry.id, compressed_body);
                let line_tokens = estimate_tokens(&line);
                if used + line_tokens <= budget {
                    context_lines.push(line);
                    used += line_tokens;
                    compressed += 1;
                }
            } else {
                // ____?3 ____________
                cutoff = true;
            }

        }
    }

    let context_text = context_lines.join("\n");

    let compressed_tokens = estimate_tokens(&context_text);

    PrioritizedOutput {
        context_text,
        compressed_count: compressed,
        raw_tokens: raw_total,
        compressed_tokens,
        savings_pct: if raw_total > 0 && compressed_tokens < raw_total {
            ((raw_total - compressed_tokens) as f64 / raw_total as f64 * 100.0 * 100.0).round()
                / 100.0
        } else {
            0.0
        },
    }
}

/// Expand keywords from text
/// __________________2 __________?_? __________?
fn extract_expand_keywords(text: &str) -> Vec<String> {
    text.split(|c: char| {
        c.is_whitespace()
            || c.is_ascii_punctuation()
            || matches!(
                c,
                '\u{2018}'
                    | '\u{2019}'
                    | '\u{201c}'
                    | '\u{201d}'
            )
    })
    .filter(|s| {
        let len = s.chars().count();
        // ____?_?2 _______?_?4 ___
        if s.chars().any(|c| c >= '\u{4e00}') {
            len >= 2
        } else {
            len >= 4
        }
    })
    .map(|s| s.to_string())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{Memory, MemoryZone};

    fn make_entry(id: &str, zone: MemoryZone, content: &str) -> crate::memory::MemoryEntry {
        crate::memory::MemoryEntry {
            id: id.to_string(),
            role: "user".into(),
            content: content.to_string(),
            zone,
            timestamp: chrono::Utc::now().to_rfc3339(),
            supersedes: None,
            superseded_by: None,
            loaded_count: 0,
            referenced_count: 0,
            keywords: vec![],
            belief_score: 0.5,
            last_effective_at: None,
        }
    }

    #[test]
    fn test_cjk_token_estimate() {
        let ascii = "hello world";
        let cjk = "你好世界测试";
        let mixed = "hello 世界";
        assert!(estimate_tokens(ascii) < estimate_tokens(cjk));
        assert!(estimate_tokens(cjk) > estimate_tokens(mixed));
    }

    #[test]
    fn test_compress_entry() {
        let entry = make_entry("test_1", MemoryZone::General, "This is a test entry for compression");
        let compressed = compress_entry(&entry, 20);
        assert!(compressed.contains("..."));
        assert!(compressed.chars().count() <= 30);
    }

    #[test]
    fn test_select_for_injection_budget() {
        let mut mem = Memory::new();
        mem.entries.push(make_entry("core_1", MemoryZone::Core, "Core memory content"));
        mem.entries.push(make_entry("work_1", MemoryZone::Work, "Work task details"));
        mem.entries.push(make_entry("episode_1", MemoryZone::Episode, "Episode memory content"));
        mem.rebuild_zone_cache();
        let result = select_for_injection(&mem, 500, &[]);
        assert!(result.raw_tokens > 0);
        assert!(result.compressed_tokens <= 500 || result.compressed_count > 0);
    }

    #[test]
    fn test_expand_keywords() {
        let keys = extract_expand_keywords("hello TokenJuice world");
        assert!(keys.contains(&"hello".to_string()));
        assert!(keys.contains(&"TokenJuice".to_string()));
    }
}