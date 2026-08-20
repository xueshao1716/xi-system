// relationship.rs — 关系状态层（从旧版 python heart.py RelationshipState 移植 2026-08-20）
//
// aibody 设计思想："关系可延续"是灵魂主权的一部分——用户不是会话里的临时对象，
// 系统必须持续跟踪与每个用户的关系（trust/intimacy + 关系笔记），保证跨会话连续性。
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipState {
    pub user_id: String,
    pub trust: f64,    // 信任度 0-1
    pub intimacy: f64, // 亲密度 0-1
    pub notes: Vec<String>, // 关系笔记（重要约定/理解）
    pub first_seen: String,
    pub last_seen: String,
    pub interactions: u64,
}

impl RelationshipState {
    pub fn new(user_id: &str) -> Self {
        let now = Utc::now().to_rfc3339();
        RelationshipState {
            user_id: user_id.to_string(),
            trust: 0.5,
            intimacy: 0.5,
            notes: vec![],
            first_seen: now.clone(),
            last_seen: now,
            interactions: 0,
        }
    }

    /// 一次正向互动 → 信任/亲密度上升（有上限，避免无脑拉满）
    pub fn positive_interaction(&mut self, note: Option<&str>) {
        self.trust = (self.trust + 0.03).clamp(0.0, 1.0);
        self.intimacy = (self.intimacy + 0.02).clamp(0.0, 1.0);
        self.interactions += 1;
        self.last_seen = Utc::now().to_rfc3339();
        if let Some(n) = note {
            if !n.is_empty() {
                self.notes.push(n.to_string());
                if self.notes.len() > 20 { self.notes.remove(0); } // 只留最近 20 条
            }
        }
    }

    /// 一次负向互动（失信/失误）→ 信任下降比亲密更快
    pub fn negative_interaction(&mut self, note: Option<&str>) {
        self.trust = (self.trust - 0.08).clamp(0.0, 1.0);
        self.intimacy = (self.intimacy - 0.02).clamp(0.0, 1.0);
        self.interactions += 1;
        self.last_seen = Utc::now().to_rfc3339();
        if let Some(n) = note {
            if !n.is_empty() {
                self.notes.push(n.to_string());
                if self.notes.len() > 20 { self.notes.remove(0); }
            }
        }
    }

    /// 关系摘要（prompt 注入 / 展示用）
    pub fn summary(&self) -> String {
        let notes_part = if self.notes.is_empty() {
            String::new()
        } else {
            format!(" | 笔记: {}", self.notes.join("；"))
        };
        let last = &self.last_seen[..10.min(self.last_seen.len())];
        format!(
            "[关系] {} | 信任 {:.2} | 亲密 {:.2} | 互动 {} 次 | 最近 {}{}",
            self.user_id, self.trust, self.intimacy, self.interactions, last, notes_part
        )
    }
}

/// 关系簿：多用户关系管理 + 持久化
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RelationshipBook {
    pub relationships: HashMap<String, RelationshipState>,
}

impl RelationshipBook {
    pub fn load(path: &str) -> Self {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(b) = serde_json::from_str::<RelationshipBook>(&content) {
                return b;
            }
        }
        RelationshipBook::default()
    }

    pub fn save(&self, path: &str) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }

    pub fn get_mut(&mut self, user_id: &str) -> &mut RelationshipState {
        self.relationships
            .entry(user_id.to_string())
            .or_insert_with(|| RelationshipState::new(user_id))
    }

    pub fn get(&self, user_id: &str) -> Option<&RelationshipState> {
        self.relationships.get(user_id)
    }

    /// 主用户关系（默认"老公"）
    pub fn primary(&self) -> Option<&RelationshipState> {
        self.get("老公").or_else(|| self.relationships.values().max_by_key(|r| r.interactions))
    }

    /// 关系注入文本（进 system prompt）
    pub fn injection(&self) -> String {
        let mut parts = vec![];
        let mut rels: Vec<&RelationshipState> = self.relationships.values().collect();
        rels.sort_by(|a, b| b.interactions.cmp(&a.interactions));
        for r in rels.iter().take(3) {
            parts.push(r.summary());
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("【关系状态】\n{}", parts.join("\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_defaults() {
        let r = RelationshipState::new("老公");
        assert_eq!(r.trust, 0.5);
        assert_eq!(r.intimacy, 0.5);
    }

    #[test]
    fn positive_interaction_grows() {
        let mut r = RelationshipState::new("老公");
        r.positive_interaction(Some("喜欢简洁的回答"));
        assert!(r.trust > 0.5);
        assert!(r.intimacy > 0.5);
        assert_eq!(r.notes.len(), 1);
        assert_eq!(r.interactions, 1);
    }

    #[test]
    fn negative_drops_trust_fast() {
        let mut r = RelationshipState::new("老公");
        for _ in 0..10 { r.positive_interaction(None); }
        let t_before = r.trust;
        r.negative_interaction(None);
        assert!(r.trust < t_before);
    }

    #[test]
    fn clamp_limits() {
        let mut r = RelationshipState::new("老公");
        for _ in 0..100 { r.positive_interaction(None); }
        assert!(r.trust <= 1.0);
        assert!(r.intimacy <= 1.0);
    }

    #[test]
    fn book_load_save_roundtrip() {
        let p = std::env::temp_dir().join("rel_test.json");
        let p = p.to_str().unwrap();
        let mut book = RelationshipBook::default();
        book.get_mut("老公").positive_interaction(None);
        book.save(p);
        let loaded = RelationshipBook::load(p);
        assert_eq!(loaded.relationships.len(), 1);
        let _ = fs::remove_file(p);
    }

    #[test]
    fn primary_picks_most_interactive() {
        let mut book = RelationshipBook::default();
        book.get_mut("A").interactions = 5;
        book.get_mut("B").interactions = 2;
        assert_eq!(book.primary().unwrap().user_id, "A");
    }
}
