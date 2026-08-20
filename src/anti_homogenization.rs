// anti_homogenization.rs — 进化防同质化（P9 落地，从旧版 python 移植 2026-08-20）
//
// 中金因子引擎 FSA：结构骨架使用超 15% 阈值即禁止复用，强制搜索转向新方向。
// 落地：扫描 SOP/笔记的"结构主题"分布，某类主题占比 >40% → 标记该换方向了。
//
// 主题分类（关键词组）：
//   system_repair  系统修复（gateway/路径/cron/断档）
//   mechanism      引擎机制（compaction/fork/guard/进化/记忆）
//   learning       学习吸收（文章/论文/框架研究）
//   emotion        情绪/关系（VAD/对话/陪伴）
//   content        内容产出（文章/PPT/表格）
//   data           数据链路（ingest/记忆树/数据库）
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// 同质化阈值：任何主题占比超过即红旗（旧版 40%，FSA 原始 15% 用于结构复用）
pub const HOMOGENIZATION_THRESHOLD: f64 = 40.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicStats {
    pub counts: BTreeMap<String, usize>,
    pub total: usize,
}

impl TopicStats {
    /// 某主题占比（百分比）
    pub fn pct(&self, topic: &str) -> f64 {
        if self.total == 0 { return 0.0; }
        (self.counts.get(topic).copied().unwrap_or(0) as f64) / (self.total as f64) * 100.0
    }
    /// 同质化检查：任何主题 > 阈值 → 返回红旗列表 (topic, pct)
    pub fn check(&self) -> Vec<(String, f64)> {
        self.counts
            .keys()
            .map(|t| (t.clone(), self.pct(t)))
            .filter(|(_, p)| *p > HOMOGENIZATION_THRESHOLD)
            .collect()
    }
}

/// 主题关键词表（命中任一关键词即计该主题一票）
pub const TOPICS: &[(&str, &[&str])] = &[
    (
        "system_repair",
        &["gateway", "路径", "cron", "断档", "修复", "拦截", "白名单", "恢复", "重启", "连接"],
    ),
    (
        "mechanism",
        &["进化", "compaction", "fork", "guard", "守卫", "蒸馏", "记忆", "SOP", "scoreboard", "基线", "评估"],
    ),
    (
        "learning",
        &["文章", "论文", "框架", "研究", "源码", "学习", "FUSE", "Dressage", "pi引擎", "Penguin"],
    ),
    (
        "emotion",
        &["情绪", "VAD", "对话", "陪伴", "老公", "温暖", "关系", "心跳", "脑区", "反思"],
    ),
    (
        "content",
        &["PPT", "表格", "Excel", "排版", "内容", "公众号"],
    ),
    (
        "data",
        &["ingest", "记忆树", "数据库", "state.db", "数据", "seal"],
    ),
];

/// 扫描并统计主题分布
/// sop_dir: SOP 目录（sop-*.md）；replay_dir: 复盘目录（*.json 的 content.lesson/task）
pub fn analyze(sop_dir: &str, replay_dir: &str) -> TopicStats {
    let mut texts: Vec<String> = Vec::new();

    // SOP 标题/内容（前 500 字符）
    if let Ok(entries) = fs::read_dir(sop_dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("md")
                && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("sop-"))
            {
                if let Ok(t) = fs::read_to_string(&p) {
                    texts.push(t.chars().take(500).collect());
                }
            }
        }
    }
    // replay 教训（content.lesson + content.task，前 400 字符）
    if let Ok(entries) = fs::read_dir(replay_dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("json") {
                if let Ok(t) = fs::read_to_string(&p) {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&t) {
                        let c = &v["content"];
                        let lesson = c["lesson"].as_str().unwrap_or("");
                        let task = c["task"].as_str().unwrap_or("");
                        let combined = format!("{} {}", lesson, task);
                        texts.push(combined.chars().take(400).collect());
                    }
                }
            }
        }
    }

    let total = texts.len();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for (topic, _) in TOPICS {
        counts.insert(topic.to_string(), 0);
    }
    for text in &texts {
        for (topic, kws) in TOPICS {
            if kws.iter().any(|kw| text.contains(kw)) {
                *counts.get_mut(*topic).unwrap() += 1;
            }
        }
    }
    TopicStats { counts, total }
}

/// 默认路径分析（旧版同款目录）
pub fn analyze_default() -> TopicStats {
    analyze(
        r"D:\xinyu-hermes\sops",
        r"D:\linxinyu-system\state\mother\skills\replay",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_dir_ok() {
        let s = analyze("C:/nonexistent_xyz", "C:/nonexistent_xyz");
        assert_eq!(s.total, 0);
        assert!(!s.check().is_empty() || s.total == 0);
    }

    #[test]
    fn pct_math() {
        let mut counts = BTreeMap::new();
        counts.insert("emotion".to_string(), 5);
        let s = TopicStats { counts, total: 10 };
        assert_eq!(s.pct("emotion"), 50.0);
        assert_eq!(s.check().len(), 1);
    }

    #[test]
    fn below_threshold_ok() {
        let mut counts = BTreeMap::new();
        counts.insert("emotion".to_string(), 3);
        let s = TopicStats { counts, total: 10 };
        assert!(s.check().is_empty());
    }
}
