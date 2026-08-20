// evolution/harsh_env.rs —— 恶劣环境引擎（2026-08-21 从 evolution.rs 拆出）
use chrono::Utc;
use serde::{Deserialize, Serialize};
use super::assets::AssetStore;
use super::{EvolutionState, EvolutionAsset};
use std::collections::HashMap;

// HarshEnvironment — 恶劣环境引擎 (2026-07-15 移植自 evo_agent 809行母本)
//
// 核心洞察 (老公 2026-07-15): "淘汰是进化的引擎"
//
// 三阶段恶化模型:
//   温和期 (severity < 0.3):  cull_threshold ≈ 0.35 (保护新生)
//   恶化期 (severity < 0.6):  cull_threshold ≈ 0.55 (筛掉平庸)
//   极端期 (severity ≥ 0.6):  cull_threshold ≈ 0.75 (只留精英)
//
// 应用到曦: 不淘汰 Agent (曦只有一个), 而是淘汰 EvolutionAsset (基因/胶囊):
//   - 严酷期动态提高 AssetStore.prune 的分数门槛
//   - 停滞检测触发变异率提升
//   - 淘汰历史写盘, 老公能看 "这一代进化压力有多大"
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarshEnv {
    pub severity: f64,           // 当前严酷度 [0.0, max_severity]
    pub growth: f64,             // 每 advance 增长速率
    pub max_severity: f64,       // 严酷度上限
    pub volatility: f64,         // 随机波动幅度
    pub generation: u64,         // advance 计数
    pub cull_history: Vec<CullRecord>,
    pub best_score_history: Vec<f64>,   // 用于停滞检测
    pub stagnation_threshold: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CullRecord {
    pub generation: u64,
    pub timestamp: String,
    pub severity: f64,
    pub cull_threshold: f64,
    pub culled_count: usize,
    pub survived_count: usize,
    pub phase: String,
}

impl HarshEnv {
    pub fn new() -> Self {
        Self {
            severity: 0.1,
            growth: 0.03,
            max_severity: 0.85,
            volatility: 0.1,
            generation: 0,
            cull_history: Vec::new(),
            best_score_history: Vec::new(),
            stagnation_threshold: 20,
        }
    }

    /// 推进一代, 返回当前环境参数
    pub fn advance(&mut self) -> (f64, f64, String) {
        self.generation += 1;
        // 加速恶化: 世代越大 growth 越快
        let gen_factor = 1.0 + 0.02 * self.generation as f64;
        self.severity = (self.severity + self.growth * gen_factor).min(self.max_severity);
        // 随机波动
        let noise: f64 = (rand_range() - 0.5) * self.volatility * 0.2;
        let eff = (self.severity + noise).clamp(0.0, 1.0);
        // 逆境逼强: 环境越恶劣 → 淘汰阈值越高
        let harshness = eff * 0.8;
        let cull_thr = 0.35 + harshness * 0.5;
        (eff, cull_thr, self.phase())
    }

    pub fn phase(&self) -> String {
        if self.severity < 0.3 { "温和".to_string() }
        else if self.severity < 0.6 { "恶化".to_string() }
        else { "极端".to_string() }
    }

    /// 记录一次淘汰
    pub fn record_cull(&mut self, cull_thr: f64, culled: usize, survived: usize) {
        self.cull_history.push(CullRecord {
            generation: self.generation,
            timestamp: Utc::now().to_rfc3339(),
            severity: self.severity,
            cull_threshold: cull_thr,
            culled_count: culled,
            survived_count: survived,
            phase: self.phase(),
        });
        // 只保留最近 100 条
        if self.cull_history.len() > 100 {
            let drop = self.cull_history.len() - 100;
            self.cull_history.drain(0..drop);
        }
    }

    /// 更新最优分历史 (用于停滞检测)
    pub fn record_best_score(&mut self, score: f64) {
        self.best_score_history.push(score);
        if self.best_score_history.len() > 200 {
            self.best_score_history.remove(0);
        }
    }

    /// 检测停滞: 最近 N 代最优分变化 < 0.01
    pub fn detect_stagnation(&self) -> bool {
        if self.best_score_history.len() < self.stagnation_threshold {
            return false;
        }
        let start = self.best_score_history.len() - self.stagnation_threshold;
        let recent = &self.best_score_history[start..];
        let max = recent.iter().cloned().fold(f64::MIN, f64::max);
        let min = recent.iter().cloned().fold(f64::MAX, f64::min);
        (max - min) < 0.01
    }

    /// 严酷期建议的变异率倍数
    pub fn mutation_boost(&self) -> f64 {
        if self.detect_stagnation() { 2.0 }
        else if self.severity >= 0.6 { 1.5 }
        else if self.severity >= 0.3 { 1.2 }
        else { 1.0 }
    }

    /// 持久化
    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }

    pub fn load(path: &str) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    pub fn summary(&self) -> String {
        format!("HarshEnv[gen={}, sev={:.3}, phase={}, culls={}, stagnant={}]",
            self.generation, self.severity, self.phase(),
            self.cull_history.len(), self.detect_stagnation())
    }
}

/// 简易随机数 (0..1), 避免引入 rand crate 依赖
fn rand_range() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos()).unwrap_or(0);
    (nanos as f64 / u32::MAX as f64).fract()
}

impl AssetStore {
    /// 严酷环境版 prune: 分数门槛跟随环境动态调整
    /// 温和期: 门槛 3.0 (原逻辑)
    /// 恶化期: 门槛 5.0
    /// 极端期: 门槛 7.0
    pub fn prune_under_harsh(&mut self, env: &HarshEnv) -> usize {
        let before = self.assets.len();
        let threshold = 3.0 + env.severity * 5.0;  // 3.0 → 7.25
        self.assets.retain(|a| a.score >= threshold || a.validation_count >= 3);
        before - self.assets.len()
    }
}

#[cfg(test)]
mod harsh_env_tests {
    use super::*;

    #[test]
    fn phases_progress() {
        let mut e = HarshEnv::new();
        assert_eq!(e.phase(), "温和");
        // 推 20 代
        for _ in 0..20 { e.advance(); }
        // severity 应该已经升过 0.3
        assert!(e.severity >= 0.3, "sev={}", e.severity);
    }

    #[test]
    fn cull_threshold_grows_with_severity() {
        let mut e = HarshEnv::new();
        e.severity = 0.1;
        let (_s1, thr1, _p1) = e.advance();
        e.severity = 0.8;
        let (_s2, thr2, _p2) = e.advance();
        assert!(thr2 > thr1, "thr1={} thr2={}", thr1, thr2);
    }

    #[test]
    fn stagnation_triggers_mutation_boost() {
        let mut e = HarshEnv::new();
        // 25 代都是同分
        for _ in 0..25 { e.record_best_score(0.5); }
        assert!(e.detect_stagnation());
        assert!(e.mutation_boost() >= 2.0);
    }

    #[test]
    fn proposals_prune_kills_more_under_extreme() {
        use chrono::{Duration, Utc};
        fn make_evo_with_old_proposals() -> EvolutionState {
            let mut evo = EvolutionState::new();
            let old_time = (Utc::now() - Duration::hours(48)).to_rfc3339();
            let new_time = (Utc::now() - Duration::hours(1)).to_rfc3339();
            // 造 5 条老+1 accepted+1 新
            for _ in 0..5 {
                evo.propose_mutation("curiosity", "up", "test-old");
            }
            evo.propose_mutation("loyalty", "up", "test-accepted");
            evo.propose_mutation("humor", "up", "test-new");
            // 按下标覆写 created_at + accepted，绕开 id 撞车
            let n = evo.proposals.len();
            for i in 0..5 {
                evo.proposals[i].created_at = old_time.clone();
                evo.proposals[i].accepted = None;
            }
            evo.proposals[n - 2].created_at = old_time.clone();
            evo.proposals[n - 2].accepted = Some(true);
            evo.proposals[n - 1].created_at = new_time;
            evo.proposals[n - 1].accepted = None;
            evo
        }
        let mut env_mild = HarshEnv::new();
        env_mild.severity = 0.1;
        let mut env_extreme = HarshEnv::new();
        env_extreme.severity = 0.8;
        let mut evo_mild = make_evo_with_old_proposals();
        let mut evo_extreme = make_evo_with_old_proposals();
        let culled_mild = evo_mild.prune_proposals_under_harsh(&env_mild);
        let culled_extreme = evo_extreme.prune_proposals_under_harsh(&env_extreme);
        assert_eq!(culled_mild, 0, "温和期 48h 老 proposals 不该被淘 (cutoff=7d)");
        assert_eq!(culled_extreme, 5, "极端期 5 条老 proposals 全淘");
        assert_eq!(evo_extreme.proposals.len(), 2, "只该剩 accepted + new");
    }

    #[test]
    fn prune_under_harsh_kills_more_at_extreme() {
        fn make_store() -> AssetStore {
            let mut store = AssetStore {
                assets: Vec::new(),
                store_path: "/tmp/_test_store".into(),
            };
            for score_10 in [10, 25, 40, 55, 70, 85] {
                let mut a = EvolutionAsset::new_gene(&format!("r{}", score_10), "t");
                a.score = score_10 as f64 / 10.0;
                a.validation_count = 0;
                store.assets.push(a);
            }
            store
        }
        let mut mild = HarshEnv::new();
        mild.severity = 0.1;
        let mut extreme = HarshEnv::new();
        extreme.severity = 0.8;
        let mut s1 = make_store();
        let mut s2 = make_store();
        let culled_mild = s1.prune_under_harsh(&mild);
        let culled_extreme = s2.prune_under_harsh(&extreme);
        assert!(culled_extreme >= culled_mild);
    }
}
