/// ______________ream Engine_?///
/// 1. ___ _?______?working memory_________
/// 2. ___ _?_______?/// 3. REM _?___________
///
/// _________________________?
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const HOME: &str = "/mnt/d/xi-system";

// __ ___ __
const MIN_WORKING_FOR_LIGHT: usize = 3;
const MIN_EPISODIC_FOR_DEEP: usize = 10;
const MIN_TOTAL_FOR_REM: usize = 20;
const WORKING_WINDOW: usize = 5;
const EPISODIC_WINDOW: usize = 20;
const REM_BUCKETS: usize = 3;
const MAX_LOG: usize = 50;

// __ ______ __

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamEntry {
    pub ts: String,
    pub phase: String,
    pub findings: Vec<String>,
    pub l2_key: String,
}

#[derive(Debug, Clone)]
pub struct MemoryState {
    pub working: Vec<String>,
    pub episodic_summaries: Vec<String>,
}

// __ _________6_____
const TOPICS: &[(&str, &[&str])] = &[
    ("tech", &["code", "rust", "python", "system", "tools", "deploy", "bug", "compile", "api", "git", "linux", "wsl"]),
    ("relation", &["you", "husband", "love", "miss", "care", "emotion", "heart", "together", "missyou", "happy"]),
    ("creative", &["write", "novel", "story", "design", "art", "creative", "structure", "role", "inspire"]),
    ("learn", &["study", "understand", "read", "see", "article", "know", "research", "explore"]),
    ("plan", &["do", "plan", "tomorrow", "later", "goal", "prepare", "intend", "want"]),
    ("emotion", &["haha", "cry", "hot", "cold", "sad", "water", "smile", "laugh", "anger"]),
];

fn extract_words(text: &str) -> HashSet<String> {
    text.chars()
        .collect::<Vec<_>>()
        .windows(2)
        .filter(|w| w[0].is_alphabetic() && w[1].is_alphabetic())
        .map(|w| w.iter().collect::<String>())
        .collect()
}

fn classify_topic(text: &str) -> Vec<&'static str> {
    let mut active = Vec::new();
    for (name, keywords) in TOPICS {
        let count = keywords.iter().filter(|kw| text.contains(*kw)).count();
        if count >= 2 {
            active.push(*name);
        }
    }
    active
}

fn sentiment(text: &str) -> &'static str {
    let pos = ["happy", "good", "great", "love", "nice", "joy", "smile", "warm", "bright", "peace"];
    let neg = ["sad", "bad", "cold", "angry", "hate", "pain", "dark", "lost", "cry"];
    let pos_c = pos.iter().filter(|w| text.contains(*w)).count();
    let neg_c = neg.iter().filter(|w| text.contains(*w)).count();
    if pos_c > neg_c + 1 { "positive" } else if neg_c > pos_c + 1 { "negative" } else { "neutral" }
}

// __ ______ __

/// ___ _?______?working memory _______?
fn light_dream(working: &[String]) -> Option<DreamEntry> {
    if working.len() < MIN_WORKING_FOR_LIGHT {
        return None;
    }

    let window = &working[working.len().saturating_sub(WORKING_WINDOW)..];
    let combined = window.join(" ");
    let topics = classify_topic(&combined);
    let sent = sentiment(&combined);

    let mut findings = vec![format!("______: {}", sent)];
    if !topics.is_empty() {
        findings.push(format!("______: {}", topics.join(", ")));
    }

    // ______
    let all_words: Vec<&str> = window.iter().flat_map(|s| s.split_whitespace()).collect();
    let mut freq: HashMap<&str, usize> = HashMap::new();
    for w in all_words {
        if w.len() >= 2 {
            *freq.entry(w).or_insert(0) += 1;
        }
    }
    let mut freq_vec: Vec<(&str, usize)> = freq.into_iter().filter(|(_, c)| *c >= 2).collect();
    freq_vec.sort_by(|a, b| b.1.cmp(&a.1));
    for (word, count) in freq_vec.iter().take(3) {
        findings.push(format!("___: {} ({}_?", word, count));
    }

    let ts = Utc::now();
    let l2_key = format!("dream-light-{}", ts.format("%Y%m%d-%H%M%S"));

    Some(DreamEntry {
        ts: ts.to_rfc3339(),
        phase: "light".into(),
        findings,
        l2_key,
    })
}

/// ___ _?_______?
fn deep_dream(episodic: &[String]) -> Option<DreamEntry> {
    if episodic.len() < MIN_EPISODIC_FOR_DEEP {
        return None;
    }

    let window = &episodic[episodic.len().saturating_sub(EPISODIC_WINDOW)..];
    let combined = window.join(" ");
    let topics = classify_topic(&combined);

    // Emotional curve analysis
    let recent: Vec<&str> = window.iter().rev().take(5).map(|s| sentiment(s)).collect();
    let pos_count = recent.iter().filter(|s| **s == "positive").count();
    let neg_count = recent.iter().filter(|s| **s == "negative").count();
    let curve = if pos_count > neg_count + 1 { "↗ 上扬" } else if neg_count > pos_count + 1 { "↘ 下沉" } else { "→ 平稳" };

    let mut findings = vec![format!("情绪曲线: {}", curve)];
    if !topics.is_empty() {
        findings.push(format!("______: {}", topics.join(", ")));
    }

    let ts = Utc::now();
    let l2_key = format!("dream-deep-{}", ts.format("%Y%m%d-%H%M%S"));

    Some(DreamEntry {
        ts: ts.to_rfc3339(),
        phase: "deep".into(),
        findings,
        l2_key,
    })
}

/// REM _?___________
fn rem_dream(working: &[String], episodic: &[String]) -> Option<DreamEntry> {
    let total = working.len() + episodic.len();
    if total < MIN_TOTAL_FOR_REM {
        return None;
    }

    // __
    let mut buckets: HashMap<String, Vec<String>> = HashMap::new();
    for entry in working {
        let bucket = classify_topic(entry).first().map(|s| s.to_string()).unwrap_or("___".into());
        buckets.entry(bucket).or_default().push(entry.clone());
    }
    for entry in episodic {
        let bucket = classify_topic(entry).first().map(|s| s.to_string()).unwrap_or("___".into());
        buckets.entry(bucket).or_default().push(entry.clone());
    }

    // _______?
    let mut connections = Vec::new();
    let bucket_names: Vec<String> = buckets.keys().cloned().collect();
    for i in 0..bucket_names.len() {
        for j in (i + 1)..bucket_names.len() {
            let a = &buckets[&bucket_names[i]];
            let b = &buckets[&bucket_names[j]];
            // ______
            for ea in a.iter().take(2) {
                for eb in b.iter().take(2) {
                    let words_a: HashSet<String> = ea.split_whitespace().map(|s| s.to_string()).collect();
                    let words_b: HashSet<String> = eb.split_whitespace().map(|s| s.to_string()).collect();
                    let overlap: Vec<&String> = words_a.intersection(&words_b).collect();
                    if overlap.len() >= 2 {
                        connections.push(format!("{} -> {}: {:?}", bucket_names[i], bucket_names[j],
                            overlap.iter().take(3).map(|s| s.as_str()).collect::<Vec<_>>()));
                    }
                }
            }
        }
    }

    let mut findings = vec!["REM _____".into()];
    if connections.is_empty() {
        findings.push("___________".into());
    } else {
        for c in connections.iter().take(5) {
            findings.push(c.clone());
        }
    }

    let ts = Utc::now();
    let l2_key = format!("dream-rem-{}", ts.format("%Y%m%d-%H%M%S"));

    Some(DreamEntry {
        ts: ts.to_rfc3339(),
        phase: "rem".into(),
        findings,
        l2_key,
    })
}

// __ ____?__

/// ____________?
fn dream_cycle(working: Vec<String>, episodic: Vec<String>) -> Vec<DreamEntry> {
    let mut results = Vec::new();

    if let Some(d) = light_dream(&working) {
        results.push(d);
    }
    if let Some(d) = deep_dream(&episodic) {
        results.push(d);
    }
    if let Some(d) = rem_dream(&working, &episodic) {
        results.push(d);
    }

    // ____?
    if !results.is_empty() {
        let path = format!("{}/dreams.json", HOME);
        let mut existing: Vec<DreamEntry> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        for d in &results {
            existing.push(d.clone());
        }
        while existing.len() > MAX_LOG {
            existing.remove(0);
        }

        if let Ok(json) = serde_json::to_string_pretty(&existing) {
            let _ = std::fs::write(&path, &json);
        }

        // _?L2 fact
        for d in &results {
            let fact_path = format!("{}/memory/l2_facts/{}.md", HOME, d.l2_key);
            let content = format!(
                "# dream: {}{}",

                d.phase,
                d.findings.join(""),

            );
            let _ = std::fs::write(&fact_path, &content);
        }
    }
    results
}

// __ _________?dream_summary _____
pub fn dream_summary() -> String {
    let path = format!("{}/dreams.json", HOME);
    let dreams: Vec<DreamEntry> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if dreams.is_empty() {
        return String::new();
    }
    let latest: Vec<&DreamEntry> = dreams.iter().rev().take(3).collect();
    let mut parts = Vec::new();
    for d in latest {
        let findings = d.findings.join("; ");
        parts.push(format!("[{}] {}", d.phase, findings));
    }
    format!("\n{}", parts.join("\n"))
}