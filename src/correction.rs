// correction.rs —— 纠正记忆（2026-08-21 从小语记忆体系移植到曦）
// 用户/系统纠正过的事，自动记录；检索时注入上下文，避免重复犯错。
// 设计参照小语"纠正记忆.md"：纠正优先（加载时注入）、防再犯（再犯计数）。
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionEntry {
    pub id: String,
    pub text: String,        // 纠正内容（不要再做 X）
    pub source: String,      // 来源（user / self / system）
    pub created_at_utc: String,
    pub repeat_count: u32,   // 再犯次数（0 = 未再犯）
    pub last_triggered_at: Option<String>,
    pub active: bool,        // 是否仍生效（false = 已纠正到位，归档）
}

impl CorrectionEntry {
    fn id_for(text: &str) -> String {
        let mut h: u64 = 5381;
        for b in text.bytes() {
            h = h.wrapping_mul(33).wrapping_add(b as u64);
        }
        format!("corr_{:08x}", h)
    }
}

/// 纠正记忆库
pub struct CorrectionMemory {
    base_dir: String,
}

impl CorrectionMemory {
    pub fn new(base_dir: &str) -> Self {
        Self { base_dir: base_dir.to_string() }
    }

    fn file(&self) -> String {
        format!("{}/state/corrections.jsonl", self.base_dir)
    }

    /// 记录一条纠正（同文本去重，仅更新来源计数）
    pub fn record(&mut self, text: &str, source: &str) -> Result<CorrectionEntry, String> {
        let mut entries = self.load_all();
        if let Some(existing) = entries.iter_mut().find(|e| e.text == text) {
            existing.active = true;
            existing.source = source.to_string();
            let entry = existing.clone();
            self.save_all(&entries)?;
            return Ok(entry);
        }
        let entry = CorrectionEntry {
            id: CorrectionEntry::id_for(text),
            text: text.to_string(),
            source: source.to_string(),
            created_at_utc: Utc::now().to_rfc3339(),
            repeat_count: 0,
            last_triggered_at: None,
            active: true,
        };
        entries.push(entry.clone());
        self.save_all(&entries)?;
        Ok(entry)
    }

    /// 触发一次纠正（再犯计数 +1，用于评估纠正是否被遵守）
    pub fn trigger(&mut self, text: &str) {
        let mut entries = self.load_all();
        if let Some(e) = entries.iter_mut().find(|e| e.text == text) {
            e.repeat_count += 1;
            e.last_triggered_at = Some(Utc::now().to_rfc3339());
        }
        let _ = self.save_all(&entries);
    }

    /// 标记纠正已到位（归档，不再注入）
    pub fn resolve(&mut self, text: &str) {
        let mut entries = self.load_all();
        if let Some(e) = entries.iter_mut().find(|e| e.text == text) {
            e.active = false;
        }
        let _ = self.save_all(&entries);
    }

    /// 活跃纠正（注入上下文的候选，按再犯次数倒序——再犯越多越要强调）
    pub fn active(&self) -> Vec<CorrectionEntry> {
        let mut out: Vec<CorrectionEntry> = self.load_all()
            .into_iter().filter(|e| e.active).collect();
        out.sort_by(|a, b| b.repeat_count.cmp(&a.repeat_count));
        out
    }

    /// 注入文本（每轮对话/判断前注入，防再犯）——像小语"加载时注入上下文"
    pub fn injection_text(&self, limit: usize) -> String {
        let active = self.active();
        if active.is_empty() { return String::new(); }
        let lines: Vec<String> = active.iter().take(limit)
            .map(|e| format!("- [纠正:{}] {}", e.source, e.text))
            .collect();
        format!("[纠正记忆——被纠正过的事，不要再犯]\n{}\n", lines.join("\n"))
    }

    fn load_all(&self) -> Vec<CorrectionEntry> {
        let mut out = Vec::new();
        if let Ok(content) = fs::read_to_string(self.file()) {
            for line in content.lines() {
                if let Ok(e) = serde_json::from_str::<CorrectionEntry>(line) {
                    out.push(e);
                }
            }
        }
        out
    }

    fn save_all(&self, entries: &[CorrectionEntry]) -> Result<(), String> {
        if let Some(parent) = Path::new(&self.file()).parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut f = fs::File::create(self.file()).map_err(|e| e.to_string())?;
        for e in entries {
            let line = serde_json::to_string(e).map_err(|e| e.to_string())?;
            f.write_all(format!("{}\n", line).as_bytes()).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_dedups_and_injects() {
        let dir = std::env::temp_dir().join("corr-test");
        let dir = dir.to_str().unwrap();
        let _ = fs::remove_dir_all(dir);
        let mut cm = CorrectionMemory::new(dir);
        cm.record("不要再写大文件（>2000 行）", "user").unwrap();
        // 同文本去重
        cm.record("不要再写大文件（>2000 行）", "user").unwrap();
        assert_eq!(cm.active().len(), 1);
        // 注入文本包含纠正
        let inj = cm.injection_text(5);
        assert!(inj.contains("不要再写大文件"), "注入应含纠正内容: {}", inj);
        assert!(inj.contains("纠正记忆"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn repeat_sorts_first() {
        let dir = std::env::temp_dir().join("corr-test2");
        let dir = dir.to_str().unwrap();
        let _ = fs::remove_dir_all(dir);
        let mut cm = CorrectionMemory::new(dir);
        cm.record("A 纠正", "user").unwrap();
        cm.record("B 纠正", "user").unwrap();
        cm.trigger("A 纠正");
        cm.trigger("A 纠正");
        let active = cm.active();
        assert_eq!(active[0].text, "A 纠正", "再犯最多的应排最前");
        assert_eq!(active[0].repeat_count, 2);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resolve_archives() {
        let dir = std::env::temp_dir().join("corr-test3");
        let dir = dir.to_str().unwrap();
        let _ = fs::remove_dir_all(dir);
        let mut cm = CorrectionMemory::new(dir);
        cm.record("已改好的事", "user").unwrap();
        cm.resolve("已改好的事");
        assert!(cm.active().is_empty(), "resolve 后应归档");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn empty_injection() {
        let cm = CorrectionMemory::new("C:/nonexistent_corr");
        assert!(cm.injection_text(5).is_empty());
    }
}
