// Pi-compatible skill loader for xi-system
//
// Reads SKILL.md files in the Anthropic Skills / Pi Agent / Hermes format:
//
//   ---
//   name: skill-name
//   description: What the skill does + when to use
//   tags: [tag1, tag2]  (optional)
//   ---
//
//   # Skill Title
//   ...markdown body...
//
// Discovery locations (in order):
//   $HOME/.xi/skills/<name>/SKILL.md
//   $XI_HOME/skills/<name>/SKILL.md
//
// Loaded skills are converted into the existing xi Skill struct so nothing
// downstream has to change. Native ctx2skill.json entries still win when a
// name collision happens — Pi skills are additive, not replacing.
//
// Author: xi-system · 2026-07-29
// Trigger: 老公读 Pi Agent 文章后授权抄能抄的
// See: docs/xi-pi-compat-proposal-2026-07-29.md

use crate::ctx2soft::{Skill, SkillLayer, SkillMemory};
use std::fs;
use std::path::{Path, PathBuf};

/// Result of scanning a skills directory.
pub struct LoadReport {
    pub loaded: Vec<String>,
    pub skipped: Vec<(String, String)>, // (name, reason)
    pub errors: Vec<(String, String)>,  // (path, error)
}

impl LoadReport {
    fn new() -> Self {
        Self {
            loaded: Vec::new(),
            skipped: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "pi_skill_loader: loaded={} skipped={} errors={}",
            self.loaded.len(),
            self.skipped.len(),
            self.errors.len()
        )
    }
}

/// Return every candidate skills root, in priority order.
///
/// Priority order matters: the FIRST place we find a skill wins for that name,
/// so more-specific / user-editable locations should come first.
fn skill_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(xi_home) = std::env::var("XI_HOME") {
        roots.push(PathBuf::from(&xi_home).join("skills"));
    }
    if let Ok(home) = std::env::var("HOME") {
        roots.push(PathBuf::from(&home).join(".xi").join("skills"));
    }
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        // Windows fallback (曦 runs on WSL but keep this door open)
        roots.push(PathBuf::from(&userprofile).join(".xi").join("skills"));
    }

    roots
}

/// Parse a SKILL.md file into a Skill struct.
///
/// Returns `Ok(None)` if the file is not a valid skill (missing frontmatter etc)
/// so the caller can skip it without treating it as an error.
pub fn parse_skill_md(path: &Path) -> Result<Option<Skill>, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("read {}: {}", path.display(), e))?;

    // Frontmatter must be the very first thing in the file.
    // A skill without frontmatter is not a Pi/Anthropic skill — skip cleanly.
    let content = content.trim_start_matches('\u{feff}'); // strip UTF-8 BOM
    if !content.starts_with("---") {
        return Ok(None);
    }

    // Split "---\nFRONTMATTER\n---\nBODY"
    let after_open = &content[3..]; // skip leading ---
    let after_open = after_open.trim_start_matches('\n');
    let end_idx = match after_open.find("\n---") {
        Some(i) => i,
        None => return Ok(None),
    };
    let frontmatter_raw = &after_open[..end_idx];
    let body = after_open[end_idx + 4..].trim_start_matches('\n').to_string();

    let (name, description, tags) = parse_frontmatter(frontmatter_raw)?;
    if name.is_empty() {
        return Err(format!(
            "{}: SKILL.md frontmatter missing required 'name' field",
            path.display()
        ));
    }

    // Convert md body into the existing (body_steps, body_appendix, examples) shape.
    // We extract H2 sections; anything under "## Examples" goes to examples,
    // anything under a heading that looks like Steps/Workflow/Usage/Execution
    // becomes body_steps as bullet lines, and everything else becomes appendix.
    let (body_steps, body_appendix, examples) = split_md_sections(&body);

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let skill = Skill {
        id: name.clone(),
        name: name.clone(),
        version: 1,
        layer: SkillLayer::User,
        description: description.clone(),
        index_summary: first_sentence(&description),
        trigger_conditions: extract_triggers(&description),
        body_steps,
        body_appendix,
        examples,
        tags,
        quality_score: 0.5,
        usage_count: 0,
        created_at: now.clone(),
        updated_at: now,
        last_reflection: String::new(),
        positive_uses: 0,
        negative_uses: 0,
        memory: SkillMemory::new(),
        confidence: 0.5,
        last_used_at: String::new(),
        related_skills: Vec::new(),
        risk_constraints: None,
    };

    Ok(Some(skill))
}

/// Extremely small YAML subset parser for skill frontmatter.
/// Supports only the three fields we care about:
///   name: string       (required)
///   description: string or | block
///   tags: [a, b]  OR  tags:\n  - a\n  - b
///
/// We deliberately do NOT depend on serde_yaml — Cargo.toml already has enough
/// weight and this file structure is trivial.
fn parse_frontmatter(text: &str) -> Result<(String, String, Vec<String>), String> {
    let mut name = String::new();
    let mut description = String::new();
    let mut tags: Vec<String> = Vec::new();

    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();

        // Block-scalar description: `description: |` or `description: >`
        if trimmed.starts_with("description:")
            && (trimmed.ends_with('|') || trimmed.ends_with('>'))
        {
            let mut buf = String::new();
            i += 1;
            while i < lines.len() {
                let l = lines[i];
                if !l.starts_with(' ') && !l.starts_with('\t') && !l.trim().is_empty() {
                    break;
                }
                if !l.trim().is_empty() {
                    if !buf.is_empty() {
                        buf.push(' ');
                    }
                    buf.push_str(l.trim());
                }
                i += 1;
            }
            description = buf;
            continue;
        }

        // Inline description: `description: "..."` or `description: text`
        if let Some(rest) = trimmed.strip_prefix("description:") {
            description = strip_quotes(rest.trim());
            i += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("name:") {
            name = strip_quotes(rest.trim());
            i += 1;
            continue;
        }
        // Inline list: `tags: [a, b, c]`
        if let Some(rest) = trimmed.strip_prefix("tags:") {
            let rest = rest.trim();
            if rest.starts_with('[') && rest.ends_with(']') {
                let inner = &rest[1..rest.len() - 1];
                tags = inner
                    .split(',')
                    .map(|s| strip_quotes(s.trim()))
                    .filter(|s| !s.is_empty())
                    .collect();
                i += 1;
                continue;
            }
            // Block list
            i += 1;
            while i < lines.len() {
                let l = lines[i];
                let lt = l.trim_start();
                if let Some(item) = lt.strip_prefix("- ") {
                    tags.push(strip_quotes(item.trim()));
                    i += 1;
                } else if lt.is_empty() {
                    i += 1;
                } else {
                    break;
                }
            }
            continue;
        }
        i += 1;
    }

    Ok((name, description, tags))
}

fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"') && s.len() >= 2)
        || (s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2)
    {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn first_sentence(s: &str) -> String {
    let s = s.trim();
    for delim in ['.', '。', '·'].iter() {
        if let Some(idx) = s.find(*delim) {
            let head = s[..idx].trim();
            if !head.is_empty() {
                return head.to_string();
            }
        }
    }
    // Fallback: first 80 chars
    if s.chars().count() > 80 {
        s.chars().take(80).collect::<String>() + "…"
    } else {
        s.to_string()
    }
}

/// Extract naive trigger phrases from the description. Anthropic-style
/// descriptions almost always contain "Use when X" / "触发：X" / "当 X 时".
/// We just collect those clauses; the LLM does the real matching at runtime.
fn extract_triggers(desc: &str) -> Vec<String> {
    let mut out = Vec::new();
    for marker in ["Use when ", "use when ", "触发", "当"] {
        if let Some(idx) = desc.find(marker) {
            let clause = &desc[idx..];
            // Take up to next full stop
            let end = clause
                .find('.')
                .or_else(|| clause.find('。'))
                .unwrap_or(clause.len().min(120));
            out.push(clause[..end].trim().to_string());
        }
    }
    if out.is_empty() {
        out.push(desc.chars().take(60).collect());
    }
    out
}

/// Split a markdown body into (body_steps, body_appendix, examples).
///
/// Heuristics:
///   - Sections whose H2 title contains "Step", "Workflow", "Usage",
///     "Execution", "步骤", "工作流", "使用" → body_steps
///   - Sections whose H2 title contains "Example", "示例", "例" → examples
///   - Everything else → body_appendix
fn split_md_sections(body: &str) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut steps = Vec::new();
    let mut appendix = Vec::new();
    let mut examples = Vec::new();

    let mut current_title = String::new();
    let mut current_body: Vec<String> = Vec::new();

    let flush = |title: &str,
                 buf: &mut Vec<String>,
                 steps: &mut Vec<String>,
                 examples: &mut Vec<String>,
                 appendix: &mut Vec<String>| {
        if buf.is_empty() {
            return;
        }
        let title_lower = title.to_lowercase();
        let content = buf.join("\n").trim().to_string();
        if content.is_empty() {
            buf.clear();
            return;
        }
        let is_step = ["step", "workflow", "usage", "execution", "步骤", "工作流", "使用"]
            .iter()
            .any(|k| title_lower.contains(k) || title.contains(k));
        let is_example = ["example", "示例", "例子", "样例"]
            .iter()
            .any(|k| title_lower.contains(k) || title.contains(k));

        if is_step {
            // Split into bullet lines
            for line in content.lines() {
                let t = line.trim();
                if let Some(item) = t
                    .strip_prefix("- ")
                    .or_else(|| t.strip_prefix("* "))
                    .or_else(|| t.strip_prefix("1. "))
                {
                    steps.push(item.trim().to_string());
                } else if !t.is_empty() {
                    steps.push(t.to_string());
                }
            }
        } else if is_example {
            for line in content.lines() {
                let t = line.trim();
                if !t.is_empty() {
                    examples.push(t.to_string());
                }
            }
        } else {
            appendix.push(format!("## {}\n{}", title, content));
        }
        buf.clear();
    };

    for line in body.lines() {
        if let Some(title) = line.strip_prefix("## ") {
            flush(
                &current_title,
                &mut current_body,
                &mut steps,
                &mut examples,
                &mut appendix,
            );
            current_title = title.trim().to_string();
        } else if line.starts_with("# ") {
            // Ignore H1 (skill title)
            continue;
        } else {
            current_body.push(line.to_string());
        }
    }
    flush(
        &current_title,
        &mut current_body,
        &mut steps,
        &mut examples,
        &mut appendix,
    );

    (steps, appendix, examples)
}

/// Discover and load every Pi-compatible skill found under known roots.
///
/// Returns a Vec<Skill> ready to be merged into the caller's `Ctx2SoftState`,
/// plus a report describing what happened. Native ctx2skill.json entries
/// should win on name collision — this function does NOT enforce that; the
/// caller decides the merge policy.
pub fn discover_pi_skills(existing_names: &[String]) -> (Vec<Skill>, LoadReport) {
    let mut report = LoadReport::new();
    let mut out = Vec::new();

    for root in skill_roots() {
        if !root.exists() {
            continue;
        }
        let entries = match fs::read_dir(&root) {
            Ok(e) => e,
            Err(e) => {
                report.errors.push((root.display().to_string(), e.to_string()));
                continue;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let skill_md = path.join("SKILL.md");
            if !skill_md.exists() {
                continue;
            }
            match parse_skill_md(&skill_md) {
                Ok(Some(skill)) => {
                    if existing_names.iter().any(|n| n == &skill.name)
                        || out.iter().any(|s: &Skill| s.name == skill.name)
                    {
                        report
                            .skipped
                            .push((skill.name.clone(), "name collision".to_string()));
                        continue;
                    }
                    report.loaded.push(skill.name.clone());
                    out.push(skill);
                }
                Ok(None) => {
                    report.skipped.push((
                        skill_md.display().to_string(),
                        "missing frontmatter".to_string(),
                    ));
                }
                Err(e) => {
                    report.errors.push((skill_md.display().to_string(), e));
                }
            }
        }
    }

    (out, report)
}

// -------- Tests --------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_quotes() {
        assert_eq!(strip_quotes("\"hello\""), "hello");
        assert_eq!(strip_quotes("'hello'"), "hello");
        assert_eq!(strip_quotes("hello"), "hello");
        assert_eq!(strip_quotes("  hello  "), "hello");
    }

    #[test]
    fn test_parse_frontmatter_inline() {
        let text = "name: my-skill\ndescription: Does a thing. Use when X.\ntags: [a, b, c]";
        let (name, desc, tags) = parse_frontmatter(text).unwrap();
        assert_eq!(name, "my-skill");
        assert_eq!(desc, "Does a thing. Use when X.");
        assert_eq!(tags, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_frontmatter_block_tags() {
        let text = "name: s\ndescription: d\ntags:\n  - one\n  - two\n";
        let (_, _, tags) = parse_frontmatter(text).unwrap();
        assert_eq!(tags, vec!["one", "two"]);
    }

    #[test]
    fn test_split_md_sections_steps_and_examples() {
        let body = "\
## Workflow
- do A
- do B
## Examples
- 例：X
## Notes
some appendix text
";
        let (steps, appendix, examples) = split_md_sections(body);
        assert_eq!(steps, vec!["do A", "do B"]);
        assert!(!examples.is_empty(), "examples should not be empty");
        assert!(examples.iter().any(|e| e.contains("例：X")));
        assert!(appendix.iter().any(|a| a.contains("Notes")));
    }

    #[test]
    fn test_first_sentence() {
        assert_eq!(first_sentence("Foo. Bar."), "Foo");
        assert_eq!(first_sentence("中文测试。第二句"), "中文测试");
    }
}
