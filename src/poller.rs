/// Feed Poller — pull from xinyu-zool/feed, check sister files, generate reports

use std::fs;
use std::path::Path;

#[derive(Debug)]
pub struct FeedEntry {
    pub title: String,
    pub source: String,
    pub date: String,
    pub tags: Vec<String>,
    pub summary: String,
}

pub fn pull_feed(feed_path: &str, since: &str) -> Vec<FeedEntry> {
    let path = Path::new(feed_path);
    if !path.exists() {
        eprintln!("[poller] feed path not found: {}", feed_path);
        return vec![];
    }

    let mut entries = Vec::new();

    if path.is_dir() {
        if let Ok(dir) = fs::read_dir(path) {
            for entry in dir.flatten() {
                let file_path = entry.path();
                if file_path.extension().map_or(false, |ext| ext == "md") {
                    if let Ok(content) = fs::read_to_string(&file_path) {
                        let file_name = file_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        if file_name.as_str() <= since {
                            continue;
                        }

                        let date = extract_frontmatter_field(&content, "date")
                            .unwrap_or_else(|| "unknown".to_string());
                        let tags: Vec<String> = extract_frontmatter_list(&content, "tags");
                        let title = extract_frontmatter_field(&content, "title")
                            .unwrap_or_else(|| file_name.clone());

                        entries.push(FeedEntry {
                            title,
                            source: file_name,
                            date,
                            tags,
                            summary: extract_summary(&content, 200),
                        });
                    }
                }
            }
        }
    }

    entries.sort_by(|a, b| b.source.cmp(&a.source));
    entries
}

fn extract_frontmatter_field(content: &str, field: &str) -> Option<String> {
    let prefix = format!("{}:", field);
    for line in content.lines().take(20) {
        let trimmed = line.trim();
        if let Some(val) = trimmed.strip_prefix(&prefix) {
            let val = val.trim().trim_matches('"').trim_matches('\'').to_string();
            if !val.is_empty() {
                return Some(val);
            }
        }
    }
    None
}

fn extract_frontmatter_list(content: &str, field: &str) -> Vec<String> {
    let prefix = format!("{}:", field);
    let lines: Vec<&str> = content.lines().collect();
    let mut in_list = false;
    let mut items = Vec::new();

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with(&prefix) {
            let rest = trimmed.strip_prefix(&prefix).unwrap_or("").trim();
            if rest.starts_with('[') {
                let inner = rest.trim_matches(|c| c == '[' || c == ']');
                for item in inner.split(',') {
                    let item = item.trim().trim_matches('"').trim_matches('\'');
                    if !item.is_empty() {
                        items.push(item.to_string());
                    }
                }
                return items;
            }
            in_list = true;
            continue;
        }
        if in_list {
            if trimmed.starts_with('-') || trimmed.starts_with('*') {
                let item = trimmed[1..].trim().trim_matches('"').to_string();
                if !item.is_empty() {
                    items.push(item);
                }
            } else {
                break;
            }
        }
    }
    items
}

fn extract_summary(content: &str, max_chars: usize) -> String {
    let mut in_frontmatter = false;
    let mut body_start = 0usize;
    for (i, line) in content.lines().enumerate() {
        if i == 0 && line.trim() == "---" {
            in_frontmatter = true;
            continue;
        }
        if in_frontmatter && line.trim() == "---" {
            in_frontmatter = false;
            body_start = i + 1;
            break;
        }
    }
    let body: String = content
        .lines()
        .skip(body_start)
        .filter(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    body.chars().take(max_chars).collect()
}

pub fn check_sister_si(shared_path: &str, since_minutes: u64) -> Vec<String> {
    check_recent_files(shared_path, since_minutes, &["md", "txt", "json", "rs"])
}

pub fn check_sister_shi(shared_path: &str, since_minutes: u64) -> Vec<String> {
    check_recent_files(shared_path, since_minutes, &["md", "txt", "json"])
}

fn check_recent_files(
    base_path: &str,
    since_minutes: u64,
    extensions: &[&str],
) -> Vec<String> {
    let path = Path::new(base_path);
    if !path.exists() {
        return vec![];
    }
    let cutoff = std::time::SystemTime::now()
        - std::time::Duration::from_secs(since_minutes * 60);
    let mut results = Vec::new();
    if let Ok(dir) = fs::read_dir(path) {
        for entry in dir.flatten() {
            let file_path = entry.path();
            if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                if !extensions.contains(&ext) {
                    continue;
                }
                if let Ok(metadata) = fs::metadata(&file_path) {
                    if let Ok(modified) = metadata.modified() {
                        if modified >= cutoff {
                            results.push(file_path.display().to_string());
                        }
                    }
                }
            }
        }
    }
    results.sort();
    results
}

pub fn generate_sync_report(
    feed_entries: &[FeedEntry],
    si_files: &[String],
    shi_files: &[String],
) -> String {
    let mut lines = Vec::new();
    let now = chrono::Local::now();
    lines.push(format!("## Sync Report — {}", now.format("%Y-%m-%d %H:%M")));
    lines.push(String::new());

    lines.push(format!("### Feed ({} new):", feed_entries.len()));
    for entry in feed_entries {
        let tags = if entry.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", entry.tags.join(", "))
        };
        lines.push(format!("- **{}**{} — {}", entry.title, tags, entry.date));
        lines.push(format!("  {}", entry.summary));
    }
    lines.push(String::new());

    lines.push(format!("### Sister Si files ({}):", si_files.len()));
    for f in si_files {
        lines.push(format!("- {}", f));
    }
    lines.push(String::new());

    lines.push(format!("### Sister Shi files ({}):", shi_files.len()));
    for f in shi_files {
        lines.push(format!("- {}", f));
    }
    lines.join("\n\n")
}
