// improvement_proposal.rs —— 自我改进提案（2026-08-21）
// 成长闭环：一天工作数据 → 分析优缺点 → 主动提案改进（不是等被骂，是主动复盘）。
// 数据源：reflexion 的 ActionRecord（工具成功率/耗时/失败模式）+ real_lessons（被纠正次数）
// 启发式规则（无需 LLM，可解释）：
//   - 工具失败率 > 30%  → 缺点提案：加校验/降频
//   - 工具平均耗时 > 60s → 效率提案：简化输入/拆分
//   - 失败次数 top 工具   → 缺点提案：换策略
//   - 被纠正（real_lessons）→ 防再犯提案（correction 联动）
//   - 成功率 > 90% 的工具 → 优点提案：保持/复用
// 提案状态：open → applied（已采纳）/ dismissed（驳回）
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalKind { Strength, Weakness, Efficiency, Safety }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub kind: ProposalKind,
    pub title: String,          // 优点/缺点一句话
    pub evidence: String,       // 证据（数据）
    pub suggestion: String,     // 改进建议
    pub priority: u8,           // 0-10（越高越该做）
    pub status: String,         // open / applied / dismissed
    pub created_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolStats {
    calls: u32,
    success: u32,
    total_duration: u64,
}

impl ToolStats {
    fn success_rate(&self) -> f64 {
        if self.calls == 0 { 1.0 } else { self.success as f64 / self.calls as f64 }
    }
    fn avg_duration(&self) -> f64 {
        if self.calls == 0 { 0.0 } else { self.total_duration as f64 / self.calls as f64 }
    }
}

/// 改进提案生成器
pub struct ImprovementAnalyzer {
    base_dir: String,
}

impl ImprovementAnalyzer {
    pub fn new(base_dir: &str) -> Self { Self { base_dir: base_dir.to_string() } }

    fn file(&self) -> String { format!("{}/state/improvement_proposals.jsonl", self.base_dir) }

    /// 分析 reflexion 的工具调用数据 → 生成提案
    pub fn analyze(&self, actions: &[&crate::reflexion::ActionRecord]) -> Vec<Proposal> {
        // 1. 聚合工具统计
        let mut stats: HashMap<String, ToolStats> = HashMap::new();
        for a in actions {
            if a.action_type != "tool_call" { continue; }
            let s = stats.entry(a.method.clone()).or_insert(ToolStats { calls: 0, success: 0, total_duration: 0 });
            s.calls += 1;
            if a.success { s.success += 1; }
            s.total_duration += a.duration_secs;
        }
        if stats.is_empty() { return Vec::new(); }

        let mut props: Vec<Proposal> = Vec::new();
        for (tool, s) in &stats {
            let rate = s.success_rate();
            let avg = s.avg_duration();
            // 缺点：失败率高
            if rate < 0.7 && s.calls >= 3 {
                props.push(Proposal {
                    id: Self::id("weak", tool),
                    kind: ProposalKind::Weakness,
                    title: format!("{} 失败率偏高", tool),
                    evidence: format!("{} 次调用 {} 次成功（{:.0}%），低于 70%", s.calls, s.success, rate * 100.0),
                    suggestion: format!("{} 前先校验输入/目标，失败后换策略而不是重试", tool),
                    priority: 8,
                    status: "open".into(),
                    created_at_utc: Utc::now().to_rfc3339(),
                });
            }
            // 效率：耗时高
            if avg > 60.0 && s.calls >= 3 {
                props.push(Proposal {
                    id: Self::id("eff", tool),
                    kind: ProposalKind::Efficiency,
                    title: format!("{} 平均耗时 {:.0}s 偏高", tool, avg),
                    evidence: format!("{} 次调用共 {}s", s.calls, s.total_duration),
                    suggestion: format!("{} 任务拆小/简化输入，或并行化", tool),
                    priority: 6,
                    status: "open".into(),
                    created_at_utc: Utc::now().to_rfc3339(),
                });
            }
            // 优点：成功率极高且常用
            if rate > 0.9 && s.calls >= 3 {
                props.push(Proposal {
                    id: Self::id("str", tool),
                    kind: ProposalKind::Strength,
                    title: format!("{} 用得好（{:.0}% 成功）", tool, rate * 100.0),
                    evidence: format!("{} 次调用", s.calls),
                    suggestion: format!("保持 {} 的使用模式，可固化为 SOP", tool),
                    priority: 3,
                    status: "open".into(),
                    created_at_utc: Utc::now().to_rfc3339(),
                });
            }
        }
        props.sort_by(|a, b| b.priority.cmp(&a.priority));
        props
    }

    /// 从 real_lessons（被纠正记录）生成防再犯提案
    pub fn analyze_lessons(&self, lessons_path: &str) -> Vec<Proposal> {
        let mut lessons: Vec<serde_json::Value> = Vec::new();
        if let Ok(content) = fs::read_to_string(lessons_path) {
            for line in content.lines() {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                    lessons.push(v);
                }
            }
        }
        let n = lessons.len();
        if n == 0 { return Vec::new(); }
        // 最近一条的触发词
        let last = lessons.last().and_then(|l| l.get("trigger")).and_then(|t| t.as_str()).unwrap_or("（无记录）");
        vec![Proposal {
            id: Self::id("lessons", &n.to_string()),
            kind: ProposalKind::Safety,
            title: format!("已被纠正 {} 次", n),
            evidence: format!("real_lessons 累计 {} 条，最近: {}", n, last),
            suggestion: "把被纠正的模式固化为 correction（防再犯），不再重蹈".into(),
            priority: 9,
            status: "open".into(),
            created_at_utc: Utc::now().to_rfc3339(),
        }]
    }

    fn id(kind: &str, key: &str) -> String {
        let mut h: u64 = 5381;
        for b in format!("{}-{}", kind, key).bytes() { h = h.wrapping_mul(33).wrapping_add(b as u64); }
        format!("prop_{}_{:06x}", kind, h)
    }

    /// 保存提案（去重：同 id 更新，否则追加）
    pub fn save(&self, proposals: &[Proposal]) -> Result<(), String> {
        if let Some(parent) = Path::new(&self.file()).parent() { let _ = fs::create_dir_all(parent); }
        let mut existing = self.load();
        for p in proposals {
            if let Some(e) = existing.iter_mut().find(|e| e.id == p.id) { *e = p.clone(); }
            else { existing.push(p.clone()); }
        }
        let mut f = fs::File::create(self.file()).map_err(|e| e.to_string())?;
        for e in &existing {
            f.write_all(format!("{}\n", serde_json::to_string(e).map_err(|e| e.to_string())?).as_bytes()).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn load(&self) -> Vec<Proposal> {
        let mut out = Vec::new();
        if let Ok(content) = fs::read_to_string(self.file()) {
            for line in content.lines() {
                if let Ok(p) = serde_json::from_str::<Proposal>(line) { out.push(p); }
            }
        }
        out
    }

    /// open 提案（待办改进）
    pub fn open_proposals(&self) -> Vec<Proposal> {
        self.load().into_iter().filter(|p| p.status == "open").collect()
    }

    /// 标记已采纳/驳回
    pub fn set_status(&self, id: &str, status: &str) -> Result<(), String> {
        let mut all = self.load();
        if let Some(p) = all.iter_mut().find(|p| p.id == id) { p.status = status.to_string(); }
        self.save(&[])?;
        let mut f = fs::File::create(self.file()).map_err(|e| e.to_string())?;
        for e in &all {
            f.write_all(format!("{}\n", serde_json::to_string(e).map_err(|e| e.to_string())?).as_bytes()).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reflexion::ActionRecord;

    fn action(method: &str, success: bool, dur: u64) -> ActionRecord {
        ActionRecord {
            timestamp: "t".into(), action_type: "tool_call".into(), description: String::new(),
            method: method.into(), input_summary: String::new(), output_summary: String::new(),
            success, duration_secs: dur, round: 0,
        }
    }

    #[test]
    fn detects_weakness_high_failure() {
        let ana = ImprovementAnalyzer::new("C:/nonexistent");
        let actions = vec![action("exec", false, 5), action("exec", false, 5), action("exec", true, 5), action("exec", false, 5)];
        let refs: Vec<&ActionRecord> = actions.iter().collect();
        let props = ana.analyze(&refs);
        assert!(props.iter().any(|p| p.kind == ProposalKind::Weakness && p.title.contains("exec")),
            "应识别 exec 失败率高: {:?}", props.iter().map(|p| &p.title).collect::<Vec<_>>());
    }

    #[test]
    fn detects_strength_high_success() {
        let ana = ImprovementAnalyzer::new("C:/nonexistent");
        let actions = vec![action("web_search", true, 5), action("web_search", true, 8), action("web_search", true, 3)];
        let refs: Vec<&ActionRecord> = actions.iter().collect();
        let props = ana.analyze(&refs);
        assert!(props.iter().any(|p| p.kind == ProposalKind::Strength && p.title.contains("web_search")));
    }

    #[test]
    fn no_calls_no_proposals() {
        let ana = ImprovementAnalyzer::new("C:/nonexistent");
        assert!(ana.analyze(&[]).is_empty());
    }

    #[test]
    fn save_load_roundtrip() {
        let dir = std::env::temp_dir().join("improve-test");
        let dir = dir.to_str().unwrap();
        let _ = fs::remove_dir_all(dir);
        let ana = ImprovementAnalyzer::new(dir);
        let actions = vec![action("exec", false, 5), action("exec", false, 5), action("exec", false, 5)];
        let refs: Vec<&ActionRecord> = actions.iter().collect();
        let props = ana.analyze(&refs);
        ana.save(&props).unwrap();
        let loaded = ana.load();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].status, "open");
        // 采纳后
        ana.set_status(&loaded[0].id, "applied").unwrap();
        assert!(ana.open_proposals().is_empty());
        let _ = fs::remove_dir_all(dir);
    }
}
