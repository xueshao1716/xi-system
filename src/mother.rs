// mother.rs — 母体层 / 土壤层 / 进化层（aibody 设计思想落地 2026-08-20）
//
// aibody 宪法三层结构第二层：这是系统真正的中心——
//   lineage     遗传谱系：一代代母体节点，可追溯
//   inheritance 继承：后代继承祖先的基因快照与经验
//   drift       漂移：后代相对父代的偏离（创新/变异），可视化"她怎么变了"
//   governance  治理：进化提案的审批/拒绝/回滚记录（不自动吞噬）
//
// "生长、进化、遗传、存活"——不是效率，是这套生态的关键词。
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

/// 一代母体（谱系上的一个节点）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotherNode {
    pub id: String,
    pub generation: u64,
    pub parent_id: Option<String>,
    /// 该代形成的人格基因快照（继承 + 漂移后）
    pub genes_snapshot: HashMap<String, f64>,
    /// 该代继承的经验指纹（来自祖先）
    pub inherited_experiences: Vec<String>,
    /// 该代相对父代的漂移（基因 → 变化量），负值=回落
    pub drift_vector: HashMap<String, f64>,
    /// 该代沉淀的人格特质描述（一句话）
    pub trait_note: String,
    pub created_at: String,
}

impl MotherNode {
    fn new(id: &str, generation: u64, parent_id: Option<String>) -> Self {
        MotherNode {
            id: id.to_string(),
            generation,
            parent_id,
            genes_snapshot: HashMap::new(),
            inherited_experiences: vec![],
            drift_vector: HashMap::new(),
            trait_note: String::new(),
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

/// 治理事件（提案的审批/回滚）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceEvent {
    pub proposal_id: String,
    pub action: String, // approve / reject / rollback
    pub summary: String,
    pub ts: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MotherLayer {
    pub lineage: Vec<MotherNode>,
    pub governance_log: Vec<GovernanceEvent>,
}

impl MotherLayer {
    pub fn load(path: &str) -> Self {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(m) = serde_json::from_str::<MotherLayer>(&content) {
                return m;
            }
        }
        MotherLayer::default()
    }

    pub fn save(&self, path: &str) {
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }

    pub fn generation(&self) -> u64 {
        self.lineage.len() as u64
    }

    /// 出生一代：继承父代基因快照 + 应用漂移（基因 += drift，clamp 0-1）
    /// 无父代（gen 0）→ 用 base_genes 初始化
    pub fn spawn_child(&mut self, base_genes: &HashMap<String, f64>, drift: &HashMap<String, f64>) -> &MotherNode {
        let gen = self.generation();
        let parent_id = self.lineage.last().map(|n| n.id.clone());
        let mut node = MotherNode::new(&format!("m-{}", gen + 1), gen + 1, parent_id);

        // 继承：父代基因快照为基础
        let base: HashMap<String, f64> = if let Some(parent) = self.lineage.last() {
            parent.genes_snapshot.clone()
        } else {
            base_genes.clone()
        };
        // 应用漂移
        for (gene, &val) in &base {
            let d = drift.get(gene).copied().unwrap_or(0.0);
            node.genes_snapshot.insert(gene.clone(), (val + d).clamp(0.0, 1.0));
        }
        // 漂移向量记录（只记有变化的基因）
        for (gene, &d) in drift {
            if d.abs() > 1e-6 {
                node.drift_vector.insert(gene.clone(), d);
            }
        }
        // 继承经验指纹
        if let Some(parent) = self.lineage.last() {
            node.inherited_experiences = parent.inherited_experiences.clone();
        }
        self.lineage.push(node);
        self.lineage.last().unwrap()
    }

    /// 给当前代补经验指纹（进化后沉淀的）
    pub fn record_experience(&mut self, experience_hash: &str) {
        if let Some(cur) = self.lineage.last_mut() {
            if !cur.inherited_experiences.contains(&experience_hash.to_string()) {
                cur.inherited_experiences.push(experience_hash.to_string());
            }
        }
    }

    /// 给当前代写人格特质描述
    pub fn record_trait(&mut self, note: &str) {
        if let Some(cur) = self.lineage.last_mut() {
            cur.trait_note = note.to_string();
        }
    }

    /// 治理：记录一次进化提案决策
    pub fn govern(&mut self, proposal_id: &str, action: &str, summary: &str) {
        self.governance_log.push(GovernanceEvent {
            proposal_id: proposal_id.to_string(),
            action: action.to_string(),
            summary: summary.to_string(),
            ts: Utc::now().to_rfc3339(),
        });
    }

    /// 回滚当前代（rollback 后移除最后一个节点，回到父代状态）
    pub fn rollback_current(&mut self, reason: &str) -> bool {
        if let Some(removed) = self.lineage.pop() {
            self.govern("rollback", "rollback", &format!("回滚 {}（{}）", removed.id, reason));
            true
        } else {
            false
        }
    }

    /// 母体层注入（进 system prompt，让模型感知自己的谱系）
    pub fn injection(&self) -> String {
        let gen = self.generation();
        if gen == 0 {
            return String::new();
        }
        let cur = self.lineage.last().unwrap();
        let mut drift_desc: Vec<String> = cur
            .drift_vector
            .iter()
            .map(|(g, d)| format!("{} {:+.2}", g, d))
            .collect();
        drift_desc.sort();
        let trait_part = if cur.trait_note.is_empty() { String::new() } else { format!(" | 特质: {}", cur.trait_note) };
        let drift_part = if drift_desc.is_empty() { "无".to_string() } else { drift_desc.join(", ") };
        let gov_part = if self.governance_log.is_empty() { String::new() } else { format!(" | 治理 {} 次", self.governance_log.len()) };
        format!(
            "【母体层】第 {} 代 | 继承 {} 条经验{} | 漂移: {}{}",
            gen, cur.inherited_experiences.len(), trait_part, drift_part, gov_part
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_genes() -> HashMap<String, f64> {
        let mut g = HashMap::new();
        g.insert("gentleness".to_string(), 0.6);
        g.insert("curiosity".to_string(), 0.7);
        g
    }

    #[test]
    fn lineage_grows() {
        let mut mother = MotherLayer::default();
        assert_eq!(mother.generation(), 0);
        mother.spawn_child(&base_genes(), &HashMap::new());
        assert_eq!(mother.generation(), 1);
        mother.spawn_child(&base_genes(), &HashMap::new());
        assert_eq!(mother.generation(), 2);
    }

    #[test]
    fn inheritance_and_drift() {
        let mut mother = MotherLayer::default();
        mother.spawn_child(&base_genes(), &HashMap::new());
        // 第二代：漂移 curiosity +0.2
        let mut drift = HashMap::new();
        drift.insert("curiosity".to_string(), 0.2);
        let node = mother.spawn_child(&base_genes(), &drift);
        assert!((node.genes_snapshot["curiosity"] - 0.9).abs() < 1e-6); // 0.7 + 0.2
        assert!((node.genes_snapshot["gentleness"] - 0.6).abs() < 1e-6); // 继承不漂移
        assert!((node.drift_vector["curiosity"] - 0.2).abs() < 1e-6);
    }

    #[test]
    fn rollback_returns_to_parent() {
        let mut mother = MotherLayer::default();
        mother.spawn_child(&base_genes(), &HashMap::new());
        let gen2 = mother.generation();
        mother.spawn_child(&base_genes(), &HashMap::new());
        assert_eq!(mother.generation(), gen2 + 1);
        assert!(mother.rollback_current("测试失败"));
        assert_eq!(mother.generation(), gen2);
    }

    #[test]
    fn governance_and_experience() {
        let mut mother = MotherLayer::default();
        mother.spawn_child(&base_genes(), &HashMap::new());
        mother.record_experience("exp-001");
        mother.record_trait("更主动");
        mother.govern("p-1", "approve", "批准主动基因");
        let inj = mother.injection();
        assert!(inj.contains("第 1 代"));
        assert!(inj.contains("继承 1 条经验"));
        assert!(inj.contains("更主动"));
        assert!(inj.contains("治理 1 次"));
    }
}
