/// Memory System — Zone-based + Supersedes + Effectiveness tracking
///
/// Zones: core, work, episode, general
/// Supersedes: newer entries can supersede older ones
/// Effectiveness: track loaded vs referenced counts

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

/// Memory zones
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryZone {
    Core,
    Work,
    Episode,
    General,
}

impl Default for MemoryZone {
    fn default() -> Self {
        MemoryZone::General
    }
}

impl MemoryZone {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryZone::Core => "core",
            MemoryZone::Work => "work",
            MemoryZone::Episode => "episode",
            MemoryZone::General => "general",
        }
    }

    /// Priority: Core > Work > General > Episode
    pub fn priority(&self) -> u8 {
        match self {
            MemoryZone::Core => 4,
            MemoryZone::Work => 3,
            MemoryZone::General => 2,
            MemoryZone::Episode => 1,
        }
    }
}

/// A single memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    #[serde(default = "default_id")]
    pub id: String,
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub zone: MemoryZone,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub supersedes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub superseded_by: Option<String>,
    #[serde(default)]
    pub loaded_count: u32,
    #[serde(default)]
    pub referenced_count: u32,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub last_effective_at: Option<String>,
    /// Belief Entropy score (0.0-1.0): coherence with surrounding context
    #[serde(default = "default_belief")]
    pub belief_score: f64,
    // Last time this entry was actually referenced (for decay calculation)
}

fn default_belief() -> f64 { 0.5 }

fn default_id() -> String {
    format!("mem_legacy_{}", Utc::now().timestamp_micros())
}

/// Extract keywords from text (stop words removed).
/// 2026-07-16: 中文按 unicode 段切开且额外切 2-gram，避免整段中文变成单一 token。
fn extract_keywords(text: &str) -> Vec<String> {
    let stop_words: std::collections::HashSet<&str> = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "shall", "can", "need", "dare", "ought",
        "used", "to", "of", "in", "for", "on", "with", "at", "by", "from",
        "as", "into", "through", "during", "before", "after", "above", "below",
        "between", "out", "off", "over", "under", "again", "further", "then",
        "once", "here", "there", "when", "where", "why", "how", "all", "both",
        "each", "few", "more", "most", "other", "some", "such", "no", "nor",
        "not", "only", "own", "same", "so", "than", "too", "very", "just",
        "don", "now",
        // 中文口语停用词
        "的", "了", "在", "是", "我", "你", "他", "她", "它", "们", "和", "跟",
        "也", "都", "就", "还", "又", "或", "把", "被", "让", "呢", "吧", "啊",
        "呀", "嗯", "哦", "哈", "个", "这", "那", "有", "没", "着", "过",
    ]
    .iter()
    .cloned()
    .collect();

    let is_sep = |c: char| {
        c.is_whitespace()
            || matches!(
                c,
                '"' | ',' | '.' | '!' | '?' | ';' | ':' | '(' | ')' | '[' | ']'
                    | '{' | '}' | '#' | '@' | '/' | '\\' | '|' | '-' | '_'
                    | '+' | '=' | '<' | '>' | '`' | '~' | '^' | '&' | '*' | '\''
            )
            // 中文全角标点
            || matches!(
                c,
                '，' | '。' | '！' | '？' | '；' | '：' | '（' | '）'
                    | '「' | '」' | '『' | '』' | '【' | '】' | '《' | '》'
                    | '“' | '”' | '‘' | '’' | '、' | '·' | '～'
            )
    };

    let mut out: Vec<String> = Vec::new();
    for chunk in text.split(is_sep).filter(|s| !s.is_empty()) {
        // 判断是否含中日韩字符
        let has_cjk = chunk.chars().any(|c| {
            let u = c as u32;
            (0x3400..=0x9FFF).contains(&u) || (0xF900..=0xFAFF).contains(&u)
        });
        if has_cjk {
            let chars: Vec<char> = chunk.chars().collect();
            // 单字 + 2-gram（bi-gram 是中文关键词最实用的最小切分）
            for c in &chars {
                let s: String = std::iter::once(*c).collect();
                if !stop_words.contains(s.as_str()) && s.chars().any(|c| {
                    let u = c as u32;
                    (0x3400..=0x9FFF).contains(&u)
                }) {
                    out.push(s);
                }
            }
            for w in chars.windows(2) {
                let s: String = w.iter().collect();
                out.push(s);
            }
        } else {
            let low = chunk.to_lowercase();
            if !low.is_empty() && !stop_words.contains(low.as_str()) {
                out.push(low);
            }
        }
    }
    // 去重保序（限制单条最多 40 kw 防爆）
    let mut seen = std::collections::HashSet::new();
    out.retain(|k| seen.insert(k.clone()));
    out.truncate(40);
    out
}

impl MemoryEntry {
    pub fn is_active(&self) -> bool {
        self.superseded_by.is_none()
    }

    /// Compute belief score: keyword overlap with recent context (MMPO insight)
    pub fn compute_belief(recent_keywords: &[String], entry_keywords: &[String]) -> f64 {
        if recent_keywords.is_empty() || entry_keywords.is_empty() { return 0.5; }
        let overlap = entry_keywords.iter()
            .filter(|k| recent_keywords.iter().any(|r| r == *k))
            .count();
        (overlap as f64 / entry_keywords.len().max(1) as f64).clamp(0.0, 1.0)
    }
}

const MAX_ENTRIES: usize = 150;
const MAX_ENTRIES_PER_ZONE: usize = 50;

/// Memory system with zone-based organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub entries: Vec<MemoryEntry>,
    #[serde(skip)]
    pub zones: HashMap<String, Vec<usize>>,
    #[serde(default)]
    pub version: u32,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            zones: HashMap::new(),
            version: 2,
        }
    }

    pub fn load(path: &str) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => {
                let mut m: Memory = serde_json::from_str(&content).unwrap_or_else(|e| {
                    eprintln!("[memory] load error: {}", e);
                    Memory::new()
                });
                if m.version < 2 {
                    m.version = 2;
                    for entry in &mut m.entries {
                        if !entry.content.contains("[zone:") {
                            entry.zone = MemoryZone::General;
                        }
                    }
                    eprintln!("[memory] Migrated to v2");
                }
                m.rebuild_zone_cache();
                m
            }
            Err(_) => {
                eprintln!("[memory] No state file, creating new");
                Memory::new()
            }
        }
    }

    pub fn save(&self, path: &str) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            if let Err(e) = fs::write(path, &json) {
                eprintln!("[memory] save error: {}", e);
            }
        }
        // Also write to four-layer architecture
        self.save_layers(path);
    }

    /// Save entries to four-layer memory files
    fn save_layers(&self, base_path: &str) {
        let base = std::path::Path::new(base_path).parent().unwrap_or(std::path::Path::new("."));
        let mem_dir = base.join("memory");

        // L1: Working memory — last 20 active entries
        let l1: Vec<&MemoryEntry> = self.entries.iter()
            .rev()
            .filter(|e| e.is_active())
            .take(20)
            .collect();
        let l1_path = mem_dir.join("l1_index.json");
        if let Ok(json) = serde_json::to_string_pretty(&l1) {
            let _ = fs::write(&l1_path, json);
        }

        // L2: Semantic — core zone entries
        let l2_entries: Vec<&MemoryEntry> = self.entries.iter()
            .filter(|e| e.is_active() && e.zone == MemoryZone::Core)
            .collect();
        let l2_dir = mem_dir.join("l2_facts");
        let _ = fs::create_dir_all(&l2_dir);
        for entry in &l2_entries {
            let filename = format!("{}.md", entry.id);
            let content = format!("# {}\n\n{}\n\n Keywords: {}\n",
                entry.timestamp, entry.content, entry.keywords.join(", "));
            let _ = fs::write(l2_dir.join(&filename), content);
        }

        // L3: Procedural — work zone entries
        let l3_entries: Vec<&MemoryEntry> = self.entries.iter()
            .filter(|e| e.is_active() && e.zone == MemoryZone::Work)
            .collect();
        let l3_dir = mem_dir.join("l3_sop");
        let _ = fs::create_dir_all(&l3_dir);
        for entry in &l3_entries {
            let filename = format!("{}.md", entry.id);
            let content = format!("# {}\n\n{}\n",
                entry.timestamp, entry.content);
            let _ = fs::write(l3_dir.join(&filename), content);
        }

        // L4: Episodic — episode zone as JSONL trace
        let l4_entries: Vec<&MemoryEntry> = self.entries.iter()
            .filter(|e| e.is_active() && e.zone == MemoryZone::Episode)
            .collect();
        let l4_dir = mem_dir.join("l4_archives");
        let _ = fs::create_dir_all(&l4_dir);
        let l4_path = l4_dir.join("traces.jsonl");
        let lines: Vec<String> = l4_entries.iter().map(|e| {
            serde_json::to_string(e).unwrap_or_default()
        }).collect();
        let _ = fs::write(&l4_path, lines.join("\n"));
    }

    pub fn rebuild_zone_cache(&mut self) {
        let mut zones: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, entry) in self.entries.iter().enumerate() {
            let zone_name = entry.zone.as_str().to_string();
            zones.entry(zone_name).or_default().push(i);
        }
        self.zones = zones;
    }

    fn generate_id(role: &str, zone: &MemoryZone) -> String {
        let ts = Utc::now().timestamp_micros();
        let prefix = match zone {
            MemoryZone::Core => "c",
            MemoryZone::Work => "w",
            MemoryZone::Episode => "e",
            MemoryZone::General => "g",
        };
        let role_prefix = match role {
            "user" => "u",
            "assistant" => "a",
            "system" => "s",
            _ => "x",
        };
        format!("mem_{}{}_{}", prefix, role_prefix, ts)
    }

    pub fn add(&mut self, role: &str, content: &str) {
        let zone = self.detect_zone(role, content);
        self.add_with_zone(role, content, zone, None);
    }

    pub fn add_with_zone(&mut self, role: &str, content: &str, zone: MemoryZone, supersedes: Option<String>) {
        let id = Self::generate_id(role, &zone);
        if let Some(ref old_id) = supersedes {
            for entry in &mut self.entries {
                if entry.id == *old_id {
                    entry.superseded_by = Some(id.clone());
                    break;
                }
            }
        }
        let keywords = extract_keywords(content);
        // Compute belief score: coherence with recent memory context
        let recent_kws: Vec<String> = self.entries.iter().rev().take(10)
            .flat_map(|e| e.keywords.iter().cloned())
            .collect();
        let belief = MemoryEntry::compute_belief(&recent_kws, &keywords);
        let entry = MemoryEntry {
            id,
            role: role.to_string(),
            content: content.to_string(),
            zone,
            keywords,
            timestamp: Utc::now().to_rfc3339(),
            supersedes: supersedes.clone(), // 2026-08-21 修复：新条目应记录它替代谁（原硬编码 None，链断裂）
            superseded_by: None,
            belief_score: belief,
            loaded_count: 0,
            referenced_count: 0,
            last_effective_at: None,
        };
        self.entries.push(entry);
        self.rebuild_zone_cache();
        if self.entries.len() > MAX_ENTRIES {
            self.trim();
        }
    }

    fn detect_zone(&self, role: &str, content: &str) -> MemoryZone {
        let lower = content.to_lowercase();
        if role != "user" && role != "assistant" {
            return MemoryZone::Core;
        }
        if lower.contains("important") || lower.contains("always") || lower.contains("remember")
            || lower.contains("never forget") || lower.contains("critical")
            || lower.contains("rule") || lower.contains("must")
        {
            return MemoryZone::Core;
        }
        if lower.contains("task") || lower.contains("project") || lower.contains("file")
            || lower.contains("command") || lower.contains("config")
        {
            return MemoryZone::Work;
        }
        MemoryZone::Episode
    }

    fn trim(&mut self) {
        let zone_names = ["core", "work", "episode", "general"];
        for zone_name in &zone_names {
            let indices: Vec<usize> = self.zones.get(*zone_name)
                .cloned().unwrap_or_default();
            if indices.len() > MAX_ENTRIES_PER_ZONE {
                let mut candidates: Vec<(usize, f64)> = indices.into_iter()
                    .filter(|&i| {
                        let e = &self.entries[i];
                        if e.is_active() && e.zone.priority() >= MemoryZone::Work.priority() {
                            return false;
                        }
                        true
                    })
                    .map(|i| {
                        let e = &self.entries[i];
                        let base_eff = if e.loaded_count > 0 {
                            e.referenced_count as f64 / e.loaded_count as f64
                        } else {
                            0.0
                        };
                        // Time decay: entries not referenced recently get penalized
                        let decay = if let Some(ref last_ref) = e.last_effective_at {
                            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(last_ref) {
                                let days = (Utc::now() - dt.with_timezone(&Utc)).num_days() as f64;
                                (1.0 - days / 60.0).max(0.3) // 60-day decay, floor at 0.3
                            } else {
                                1.0
                            }
                        } else {
                            // Never referenced: use loaded_count as proxy
                            if e.loaded_count > 0 { 0.5 } else { 0.3 }
                        };
                        let eff = base_eff * decay;
                        // Belief Entropy: entries with low coherence get penalized
                        let belief_factor = 0.5 + e.belief_score * 0.5; // 0.5-1.0 range
                        let eff = eff * belief_factor;
                        (i, eff)
                    })
                    .collect();
                candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                let keep = self.entries.len().min(MAX_ENTRIES);
                let to_remove: std::collections::HashSet<usize> = candidates
                    .iter()
                    .take(self.entries.len().saturating_sub(keep))
                    .map(|(i, _)| *i)
                    .collect();
                if !to_remove.is_empty() {
                    self.entries = self.entries.iter()
                        .enumerate()
                        .filter(|(i, _)| !to_remove.contains(i))
                        .map(|(_, e)| e.clone())
                        .collect();
                    self.rebuild_zone_cache();
                }
            }
        }
    }

    pub fn active_entries(&self) -> Vec<&MemoryEntry> {
        self.entries.iter().filter(|e| e.is_active()).collect()
    }

    pub fn active_by_zone(&self, zone: MemoryZone) -> Vec<&MemoryEntry> {
        self.entries.iter()
            .filter(|e| e.is_active() && e.zone == zone)
            .collect()
    }

    pub fn record_loaded(&mut self, id: &str) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
            entry.loaded_count += 1;
        }
    }

    pub fn record_referenced(&mut self, id: &str) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
            entry.referenced_count += 1;
            entry.last_effective_at = Some(Utc::now().to_rfc3339());
        }
    }

    pub fn effectiveness_stats(&self) -> Vec<(String, f64, u32, u32)> {
        self.entries.iter()
            .filter(|e| e.is_active())
            .map(|e| {
                let eff = if e.loaded_count > 0 {
                    (e.referenced_count as f64 / e.loaded_count as f64 * 100.0 * 100.0).round() / 100.0
                } else {
                    0.0
                };
                (e.id.clone(), eff, e.loaded_count, e.referenced_count)
            })
            .collect()
    }

    pub fn recent_dialog(&self, n: usize) -> String {
        let recent: Vec<&MemoryEntry> = self.entries.iter()
            .rev()
            .filter(|e| e.is_active())
            .take(n)
            .collect();
        if recent.is_empty() {
            return String::new();
        }
        let mut lines = Vec::new();
        for e in recent.iter().rev() {
            let label = if e.role == "user" { "user" } else { "assistant" };
            let content = e.content.chars().take(100).collect::<String>();
            lines.push(format!("[{}] {}", label, content));
        }
        lines.join("\n")
    }

    pub fn zone_summary(&self) -> String {
        let mut parts = Vec::new();
        let core = self.active_by_zone(MemoryZone::Core);
        if !core.is_empty() {
            parts.push(format!("Core ({} entries):", core.len()));
            for e in &core {
                parts.push(format!("  - {}", e.content.chars().take(120).collect::<String>()));
            }
            parts.push(String::new());
        }
        let work = self.active_by_zone(MemoryZone::Work);
        if !work.is_empty() {
            parts.push(format!("Work ({} entries):", work.len()));
            for e in &work {
                parts.push(format!("  - {}", e.content.chars().take(120).collect::<String>()));
            }
            parts.push(String::new());
        }
        parts.join("\n")
    }

    pub fn search_by_keyword(&self, query: &str) -> Vec<&MemoryEntry> {
        let query_keywords = extract_keywords(query);
        if query_keywords.is_empty() {
            return Vec::new();
        }
        let mut results: Vec<&MemoryEntry> = self
            .entries
            .iter()
            .filter(|e| e.is_active())
            .filter(|e| {
                query_keywords
                    .iter()
                    .any(|kw| e.content.contains(kw.as_str()))
            })
            .collect();
        results.sort_by_key(|e| std::cmp::Reverse(e.zone.priority()));
        results
    }

    pub fn search_recent(&self, hours: u64) -> Vec<&MemoryEntry> {
        let cutoff = Utc::now() - chrono::Duration::hours(hours as i64);
        self.entries
            .iter()
            .filter(|e| e.is_active())
            .filter(|e| {
                chrono::DateTime::parse_from_rfc3339(&e.timestamp)
                    .map(|dt| dt > cutoff)
                    .unwrap_or(false)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find_by_content<'a>(mem: &'a Memory, content: &str) -> &'a MemoryEntry {
        mem.entries.iter().find(|e| e.content == content).expect("条目应存在")
    }

    #[test]
    fn zones_priority_order() {
        assert!(MemoryZone::Core.priority() > MemoryZone::Work.priority());
        assert!(MemoryZone::Work.priority() > MemoryZone::General.priority());
        assert!(MemoryZone::General.priority() > MemoryZone::Episode.priority());
    }

    #[test]
    fn supersedes_chain_links_entries() {
        let mut mem = Memory::new();
        let old_id = "mem-old-1".to_string();
        mem.entries.push(MemoryEntry {
            id: old_id.clone(), role: "user".into(), content: "旧方案".into(),
            zone: MemoryZone::Work, timestamp: Utc::now().to_rfc3339(),
            supersedes: None, superseded_by: None, loaded_count: 0, referenced_count: 0,
            keywords: vec![], last_effective_at: None, belief_score: 0.5,
        });
        mem.add_with_zone("user", "新方案（替代旧）", MemoryZone::Work, Some(old_id.clone()));
        let new = find_by_content(&mem, "新方案（替代旧）");
        assert_eq!(new.supersedes.as_deref(), Some(old_id.as_str()));
        let old = mem.entries.iter().find(|e| e.id == old_id).unwrap();
        assert_eq!(old.superseded_by.as_deref(), Some(new.id.as_str()));
    }

    #[test]
    fn zone_filtering_returns_only_zone() {
        let mut mem = Memory::new();
        mem.add_with_zone("user", "核心工作记忆", MemoryZone::Core, None);
        mem.add_with_zone("user", "普通对话", MemoryZone::General, None);
        let core = mem.active_by_zone(MemoryZone::Core);
        assert_eq!(core.len(), 1);
        assert_eq!(core[0].content, "核心工作记忆");
    }

    #[test]
    fn belief_score_bounded() {
        let recent = vec!["ai".to_string(), "模型".to_string()];
        let entry = vec!["ai".to_string(), "进化".to_string()];
        let s = MemoryEntry::compute_belief(&recent, &entry);
        assert!((0.0..=1.0).contains(&s), "belief 必须 [0,1]，got {}", s);
    }

    #[test]
    fn effectiveness_tracks_loads() {
        let mut mem = Memory::new();
        mem.add_with_zone("user", "高频记忆条目测试", MemoryZone::Work, None);
        let id = find_by_content(&mem, "高频记忆条目测试").id.clone();
        mem.record_loaded(&id);
        mem.record_loaded(&id);
        mem.record_referenced(&id);
        let stats = mem.effectiveness_stats();
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].2, 2, "loaded_count 应为 2");
        assert_eq!(stats[0].3, 1, "referenced_count 应为 1");
    }
}
