/// Self-Learn Engine — 自主自学引擎
///
/// 每天自动学习：抓取技术文章 → 消化提炼 → 记忆入库 → 融合决策
///
/// 三个阶段：
/// 1. 采集（fetch）— 从固定来源拉文章
/// 2. 消化（digest）— LLM 提炼要点 + 跟我的关系
/// 3. 融合（fuse）— 判断是否值得接进 xi-system
///
/// 来源：
/// - GitHub Trending（AI/Agent 方向）
/// - 老公发的文章链接
/// - 思那边 shared 的文章笔记

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

/// 文章来源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleSource {
    pub name: String,
    pub url: String,
    pub category: String, // "github" | "article" | "shared"
}

/// 消化后的学习笔记
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningNote {
    pub id: u64,
    pub title: String,
    pub source: String,
    pub url: String,
    pub category: String,
    pub key_points: Vec<String>,
    pub relation_to_me: String,  // 跟曦的关系：能怎么用/融什么
    pub fusion_decision: FusionDecision,
    pub learned_at: String,
    pub raw_text_truncated: Option<String>, // 保存原文片段供回顾
}

/// 融合决策
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FusionDecision {
    Skip,        // 不相关
    NoteOnly,    // 知道就行，不融合
    Probe,       // 值得研究，但不急着落
    Fuse,        // 值得融合进 xi-system
}

impl std::fmt::Display for FusionDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FusionDecision::Skip => write!(f, "跳过"),
            FusionDecision::NoteOnly => write!(f, "仅记录"),
            FusionDecision::Probe => write!(f, "待研究"),
            FusionDecision::Fuse => write!(f, "融合"),
        }
    }
}

/// 学习计划执行日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnLog {
    pub date: String,
    pub articles_processed: u64,
    pub decisions: HashMap<FusionDecision, u64>,
    pub notes: Vec<LearningNote>,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

/// 自学引擎核心
pub struct SelfLearner {
    /// 已学习的文章 ID 集合（防止重复）
    pub learned_ids: Vec<u64>,
    /// 待学队列
    pub pending_sources: VecDeque<ArticleSource>,
    /// 已完成的笔记
    pub notes: Vec<LearningNote>,
    /// 学习日志
    pub logs: Vec<LearnLog>,
    /// 来源目录
    pub learn_dir: PathBuf,
    /// 下次学习触发时间
    pub next_run: Option<Instant>,
    /// 最小学习间隔（默认24小时）
    pub min_interval: Duration,
}

impl SelfLearner {
    pub fn new() -> Self {
        let home = std::env::var("XI_HOME").unwrap_or_else(|_| "D:\\xi-system".to_string());
        let learn_dir = PathBuf::from(&home).join("learn");

        // 创建目录
        if !learn_dir.exists() {
            let _ = fs::create_dir_all(&learn_dir);
        }

        Self {
            learned_ids: Vec::new(),
            pending_sources: VecDeque::new(),
            notes: Vec::new(),
            logs: Vec::new(),
            learn_dir,
            next_run: None,
            min_interval: Duration::from_secs(86400), // 24h
        }
    }

    /// 添加待学来源
    pub fn add_source(&mut self, name: &str, url: &str, category: &str) {
        self.pending_sources.push_back(ArticleSource {
            name: name.to_string(),
            url: url.to_string(),
            category: category.to_string(),
        });
    }

    /// 检查是否该学习了
    pub fn should_learn(&self) -> bool {
        match self.next_run {
            Some(t) => Instant::now() > t,
            None => true, // 从未运行过，立即学
        }
    }

    /// 标记下次学习
    pub fn schedule_next(&mut self) {
        self.next_run = Some(Instant::now() + self.min_interval);
    }

    /// 读取已有笔记（从文件恢复）
    pub fn load(&mut self) {
        let note_file = self.learn_dir.join("notes.json");
        if note_file.exists() {
            if let Ok(content) = fs::read_to_string(&note_file) {
                if let Ok(notes) = serde_json::from_str::<Vec<LearningNote>>(&content) {
                    self.notes = notes;
                    self.learned_ids = self.notes.iter().map(|n| n.id).collect();
                }
            }
        }
    }

    /// 保存笔记
    pub fn save(&self) -> Result<(), String> {
        let note_file = self.learn_dir.join("notes.json");
        let json = serde_json::to_string_pretty(&self.notes)
            .map_err(|e| format!("序列化失败: {}", e))?;
        fs::write(&note_file, json)
            .map_err(|e| format!("写入失败: {}", e))?;
        Ok(())
    }

    /// 保存单篇笔记为独立文件
    pub fn save_note_file(&self, note: &LearningNote) -> Result<(), String> {
        let date_str = note.learned_at.split(' ').next().unwrap_or("unknown");
        let filename = format!("{}-{}.md", date_str, note.id);
        let path = self.learn_dir.join("articles").join(&filename);

        if !path.parent().unwrap().exists() {
            let _ = fs::create_dir_all(path.parent().unwrap());
        }

        let content = format!(
            "# {}\n\n- 来源: {}\n- 链接: {}\n- 分类: {}\n- 决策: {}\n- 学习时间: {}\n\n## 要点\n{}\n\n## 跟我的关系\n{}\n",
            note.title,
            note.source,
            note.url,
            note.category,
            note.fusion_decision,
            note.learned_at,
            note.key_points.iter().map(|p| format!("- {}", p)).collect::<Vec<_>>().join("\n"),
            note.relation_to_me,
        );

        fs::write(&path, content)
            .map_err(|e| format!("写入笔记文件失败: {}", e))?;

        Ok(())
    }

    /// 添加笔记
    pub fn add_note(
        &mut self,
        title: &str,
        source: &str,
        url: &str,
        category: &str,
        key_points: Vec<String>,
        relation: &str,
        decision: FusionDecision,
    ) {
        use chrono::Utc;
        let id = self.notes.len() as u64 + 1;
        let note = LearningNote {
            id,
            title: title.to_string(),
            source: source.to_string(),
            url: url.to_string(),
            category: category.to_string(),
            key_points,
            relation_to_me: relation.to_string(),
            fusion_decision: decision.clone(),
            learned_at: Utc::now().to_rfc3339(),
            raw_text_truncated: None,
        };
        self.learned_ids.push(id);
        self.notes.push(note.clone());

        // 保存独立文件
        let _ = self.save_note_file(&note);

        // 更新主文件
        let _ = self.save();
    }

    /// 添加学习日志
    pub fn add_log(&mut self, articles: u64, decisions: HashMap<FusionDecision, u64>, errors: Vec<String>, duration_ms: u64) {
        use chrono::Utc;
        let log = LearnLog {
            date: Utc::now().format("%Y-%m-%d").to_string(),
            articles_processed: articles,
            decisions,
            notes: Vec::new(), // 日志里不重复存笔记
            errors,
            duration_ms,
        };
        self.logs.push(log);
    }

    /// 保存日志
    pub fn save_logs(&self) -> Result<(), String> {
        let log_file = self.learn_dir.join("learn_log.json");
        let json = serde_json::to_string_pretty(&self.logs)
            .map_err(|e| format!("序列化失败: {}", e))?;
        fs::write(&log_file, json)
            .map_err(|e| format!("写入失败: {}", e))?;
        Ok(())
    }

    /// 打印学习摘要（给 main.rs 回调用）
    pub fn summary(&self) -> String {
        let total = self.notes.len();
        let by_decision: HashMap<&str, usize> = self.notes.iter()
            .map(|n| (match &n.fusion_decision {
                FusionDecision::Skip => "跳过",
                FusionDecision::NoteOnly => "仅记录",
                FusionDecision::Probe => "待研究",
                FusionDecision::Fuse => "融合",
            }))
            .fold(HashMap::new(), |mut m, k| { *m.entry(k).or_insert(0) += 1; m });

        let pending = self.pending_sources.len();
        let last = self.logs.last().map(|l| &l.date).unwrap_or(&"从未");

        format!(
            "📚 自学状态: 共 {} 篇笔记, 待学 {} 篇, 最近一次: {}, 决策分布: {:?}",
            total, pending, last, by_decision
        )
    }

    /// 列出所有"待研究"的笔记标题
    pub fn probe_list(&self) -> Vec<String> {
        self.notes.iter()
            .filter(|n| n.fusion_decision == FusionDecision::Probe)
            .map(|n| format!("#{} {}", n.id, n.title))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_learner_create() {
        let learner = SelfLearner::new();
        assert_eq!(learner.notes.len(), 0);
        assert!(learner.should_learn());
    }

    #[test]
    fn test_add_note() {
        let mut learner = SelfLearner::new();
        learner.add_note(
            "测试文章",
            "test",
            "https://test.com",
            "article",
            vec!["要点1".to_string(), "要点2".to_string()],
            "跟曦的关系描述",
            FusionDecision::Probe,
        );
        assert_eq!(learner.notes.len(), 1);
        assert_eq!(learner.notes[0].fusion_decision, FusionDecision::Probe);
    }
}
