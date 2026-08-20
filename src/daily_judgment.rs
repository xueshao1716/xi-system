// daily_judgment.rs —— 每日判断器（2026-08-21 从 aibody daily_judgment.py 移植到曦）
// 每天产出一条自己的判断（judgment + risk_note），三路持久化：
//   1. state/daily_judgment.json（当前）
//   2. state/daily_judgments.jsonl（日志可追溯）
//   3. 去重：同一天只产一条
// 与 reflexion 的区别：reflexion 是被动反思（复盘已发生），daily_judgment 是主动判断（今天该怎样）
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyJudgment {
    pub id: String,
    pub date: String,          // YYYY-MM-DD
    pub judgment: String,      // 判断内容（今天该怎样/什么值得做）
    pub risk_note: String,     // 风险声明
    pub created_at_utc: String,
    pub source: String,
}

impl DailyJudgment {
    fn id_for(date: &str, judgment: &str) -> String {
        // 简单 hash（不引额外 crate）
        let mut h: u64 = 5381;
        for b in format!("{}-{}", date, judgment).bytes() {
            h = h.wrapping_mul(33).wrapping_add(b as u64);
        }
        format!("daily_judgment_{}_{:08x}", date.replace('-', ""), h)
    }
}

/// 判断器：外部传入判断文本 + 风险声明，持久化（去重）
pub struct DailyJudgmentStore {
    base_dir: String,
}

impl DailyJudgmentStore {
    pub fn new(base_dir: &str) -> Self {
        Self { base_dir: base_dir.to_string() }
    }

    fn json_file(&self) -> String {
        format!("{}/state/daily_judgment.json", self.base_dir)
    }
    fn jsonl_file(&self) -> String {
        format!("{}/state/daily_judgments.jsonl", self.base_dir)
    }

    fn today(&self) -> String {
        Utc::now().format("%Y-%m-%d").to_string()
    }

    /// 今天是否已有判断
    pub fn has_judgment_today(&self) -> bool {
        if let Ok(content) = fs::read_to_string(self.json_file()) {
            if let Ok(dj) = serde_json::from_str::<DailyJudgment>(&content) {
                return dj.date == self.today();
            }
        }
        false
    }

    /// 记录一条判断（同一天已有则跳过）
    pub fn judge(&self, judgment_text: &str, risk_note: &str) -> Result<DailyJudgment, String> {
        let today = self.today();
        if self.has_judgment_today() {
            return Err(format!("今天({})已有判断，跳过", today));
        }
        let record = DailyJudgment {
            id: DailyJudgment::id_for(&today, judgment_text),
            date: today.clone(),
            judgment: judgment_text.to_string(),
            risk_note: risk_note.to_string(),
            created_at_utc: Utc::now().to_rfc3339(),
            source: "xi-daily-judgment".into(),
        };
        // 1. 当前判断
        if let Some(parent) = Path::new(&self.json_file()).parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(&record).map_err(|e| e.to_string())?;
        fs::write(self.json_file(), json).map_err(|e| e.to_string())?;
        // 2. 日志 jsonl（追加）
        let line = serde_json::to_string(&record).map_err(|e| e.to_string())?;
        if let Some(parent) = Path::new(&self.jsonl_file()).parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::OpenOptions::new()
            .create(true).append(true)
            .open(self.jsonl_file())
            .map_err(|e| e.to_string())?
            .write_all(format!("{}\n", line).as_bytes())
            .map_err(|e| e.to_string())?;
        Ok(record)
    }

    /// 最近判断列表（供回溯/注入）
    pub fn recent(&self, n: usize) -> Vec<DailyJudgment> {
        let mut out = Vec::new();
        if let Ok(content) = fs::read_to_string(self.jsonl_file()) {
            for line in content.lines().rev().take(n) {
                if let Ok(dj) = serde_json::from_str::<DailyJudgment>(line) {
                    out.push(dj);
                }
            }
        }
        out
    }
}

use std::io::Write;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn judge_persists_and_dedups() {
        let dir = std::env::temp_dir().join("dj-test");
        let dir = dir.to_str().unwrap();
        let _ = fs::remove_dir_all(dir);
        let store = DailyJudgmentStore::new(dir);
        let r = store.judge("今天该把进化门拆干净", "低风险");
        assert!(r.is_ok());
        // 同日第二条被跳过
        let r2 = store.judge("第二条不该进", "中风险");
        assert!(r2.is_err());
        assert!(r2.unwrap_err().contains("已有判断"));
        // 持久化验证
        let jsonl = fs::read_to_string(format!("{}/state/daily_judgments.jsonl", dir)).unwrap();
        assert!(jsonl.contains("今天该把进化门拆干净"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn recent_returns_latest() {
        let dir = std::env::temp_dir().join("dj-test2");
        let dir = dir.to_str().unwrap();
        let _ = fs::remove_dir_all(dir);
        let store = DailyJudgmentStore::new(dir);
        let _ = store.judge("判断A", "");
        let recent = store.recent(5);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].judgment, "判断A");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn empty_store_ok() {
        let store = DailyJudgmentStore::new("C:/nonexistent_xyz");
        assert!(!store.has_judgment_today());
        assert!(store.recent(3).is_empty());
    }
}
