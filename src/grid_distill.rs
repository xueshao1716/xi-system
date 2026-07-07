/// Grid Distill — 实时语义漂移检测与校正
///
/// 核心三件事：
/// 1. 锚点：句子中间插特征标记（intent tag）
/// 2. 漂移检测：对比前后语义距离
/// 3. 实时校正：偏了立刻拽回来，不回头改

use std::collections::VecDeque;

/// 语义锚点：文本片段 + intent标记 + 特征向量
#[derive(Debug, Clone)]
pub struct SemanticAnchor {
    pub features: Vec<f64>,
    pub seq: usize,
    pub intent_tag: String,
    pub text_snippet: String,  // 原文片段，用于回溯
}

/// 漂移检测结果
#[derive(Debug, Clone)]
pub struct DriftResult {
    pub drifted: bool,
    pub distance: f64,
    pub suggestion: String,    // 校正建议
    pub anchor_index: usize,   // 哪个锚点漂移了
}

/// Grid Distiller — 实时漂移检测器
pub struct GridDistiller {
    window: VecDeque<SemanticAnchor>,
    window_size: usize,
    drift_threshold: f64,
    pub adjustments: usize,
    current_seq: usize,
    /// 漂移历史：记录每次漂移的距离和校正建议
    drift_history: Vec<DriftResult>,
}

impl GridDistiller {
    pub fn new(window_size: usize, drift_threshold: f64) -> Self {
        Self {
            window: VecDeque::with_capacity(window_size + 1),
            window_size,
            drift_threshold,
            adjustments: 0,
            current_seq: 0,
            drift_history: Vec::new(),
        }
    }

    /// 核心：喂入文本 + intent标记，返回是否漂移
    /// intent_tag 是"锚点"——插在句子中间的特征标记
    pub fn feed(&mut self, text: &str, intent_tag: &str) -> DriftResult {
        let anchor = self.extract_anchor(text, intent_tag);
        let result = self.detect_drift(&anchor);

        if result.drifted {
            self.adjustments += 1;
            self.drift_history.push(result.clone());
            // 保持历史不超过100条
            if self.drift_history.len() > 100 {
                self.drift_history.remove(0);
            }
        }

        self.window.push_back(anchor);
        if self.window.len() > self.window_size {
            self.window.pop_front();
        }
        self.current_seq += 1;
        result
    }

    /// 提取锚点特征：文本长度 + 词汇密度 + 标点密度 + 情感极性
    fn extract_anchor(&self, text: &str, intent_tag: &str) -> SemanticAnchor {
        let char_count = text.chars().count() as f64;
        let word_count = text.split_whitespace().count() as f64;
        let punct_count = text.chars().filter(|c| c.is_ascii_punctuation()).count() as f64;
        let avg_word_len = if word_count > 0.0 { char_count / word_count } else { 0.0 };
        let punct_density = if char_count > 0.0 { punct_count / char_count } else { 0.0 };

        // 中文字符比例（区分中英文内容）
        let cn_chars = text.chars().filter(|c| *c as u32 >= 0x4E00 && *c as u32 <= 0x9FFF).count() as f64;
        let cn_ratio = if char_count > 0.0 { cn_chars / char_count } else { 0.0 };

        // 特征向量：[长度, 平均词长, 标点密度, 中文比例]
        let features = vec![char_count, avg_word_len, punct_density, cn_ratio];

        SemanticAnchor {
            features,
            seq: self.current_seq,
            intent_tag: intent_tag.to_string(),
            text_snippet: text.chars().take(50).collect(),
        }
    }

    /// 漂移检测：对比前后锚点的语义距离
    fn detect_drift(&self, anchor: &SemanticAnchor) -> DriftResult {
        if self.window.is_empty() {
            return DriftResult {
                drifted: false,
                distance: 0.0,
                suggestion: String::new(),
                anchor_index: 0,
            };
        }

        let last = self.window.back().unwrap();

        // 1. intent标记变了 → 必然漂移
        if last.intent_tag != anchor.intent_tag {
            let dist = cosine_distance(&last.features, &anchor.features);
            return DriftResult {
                drifted: true,
                distance: dist,
                suggestion: format!("intent从'{}'变到'{}'，话题切换", last.intent_tag, anchor.intent_tag),
                anchor_index: self.window.len(),
            };
        }

        // 2. 同一intent下，语义距离超阈值 → 漂移
        let dist = cosine_distance(&last.features, &anchor.features);
        if dist > self.drift_threshold {
            // 生成校正建议
            let suggestion = if dist > self.drift_threshold * 2.0 {
                format!("严重漂移(dist={:.3})，建议回到intent '{}'的核心话题", dist, anchor.intent_tag)
            } else {
                format!("轻微漂移(dist={:.3})，注意保持在'{}'的范围内", dist, anchor.intent_tag)
            };

            return DriftResult {
                drifted: true,
                distance: dist,
                suggestion,
                anchor_index: self.window.len(),
            };
        }

        DriftResult {
            drifted: false,
            distance: dist,
            suggestion: String::new(),
            anchor_index: 0,
        }
    }

    /// 实时校正：根据漂移结果生成校正后的文本方向
    /// 不回头改，只在前向跑的时候纠错
    pub fn correct(&self, result: &DriftResult, original_intent: &str) -> String {
        if !result.drifted {
            return String::new();
        }

        // 从漂移历史中找到同一intent的最近锚点
        let recent = self.window.iter()
            .rev()
            .find(|a| a.intent_tag == original_intent);

        match recent {
            Some(anchor) => format!(
                "[校正] 回到'{}'方向。参考前文: \"{}...\"",
                original_intent,
                anchor.text_snippet
            ),
            None => format!(
                "[校正] 请继续围绕'{}'展开",
                original_intent
            ),
        }
    }

    /// 漂移报告
    pub fn drift_report(&self) -> String {
        let recent_drifts: Vec<String> = self.drift_history.iter()
            .rev()
            .take(5)
            .map(|d| format!("seq={} dist={:.3} {}", d.anchor_index, d.distance, d.suggestion))
            .collect();

        format!(
            "GridDistill: seq={}, window={}, adjustments={}, recent_drifts=[{}]",
            self.current_seq,
            self.window.len(),
            self.adjustments,
            recent_drifts.join("; ")
        )
    }

    /// 获取漂移率（漂移次数/总锚点数）
    pub fn drift_rate(&self) -> f64 {
        if self.current_seq == 0 {
            return 0.0;
        }
        self.adjustments as f64 / self.current_seq as f64
    }

    pub fn reset(&mut self) {
        self.window.clear();
        self.adjustments = 0;
        self.current_seq = 0;
        self.drift_history.clear();
    }
}

/// Cosine distance (1 - cosine_similarity)
fn cosine_distance(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 1.0;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let nb: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 1.0;
    }
    1.0 - (dot / (na * nb)).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_drift_on_similar_text() {
        let mut d = GridDistiller::new(5, 0.35);
        let r1 = d.feed("你好，今天天气不错", "conversation");
        assert!(!r1.drifted);
        let r2 = d.feed("是啊，阳光明媚", "conversation");
        assert!(!r2.drifted);
    }

    #[test]
    fn test_drift_on_topic_switch() {
        let mut d = GridDistiller::new(5, 0.35);
        d.feed("今天天气真好，适合出去玩", "conversation");
        let r = d.feed("The algorithm uses dynamic programming with O(n^2) complexity", "code");
        assert!(r.drifted);
        assert!(r.suggestion.contains("intent"));
    }

    #[test]
    fn test_drift_rate() {
        let mut d = GridDistiller::new(5, 0.35);
        d.feed("hello", "a");
        d.feed("world", "a");
        d.feed("completely different topic about quantum physics", "b");
        assert!(d.adjustments > 0);
        assert!(d.drift_rate() > 0.0);
    }

    #[test]
    fn test_correction_suggestion() {
        let mut d = GridDistiller::new(5, 0.35);
        d.feed("讨论AI Agent架构", "tech");
        let r = d.feed("今天中午吃什么", "tech");
        if r.drifted {
            let correction = d.correct(&r, "tech");
            assert!(correction.contains("校正"));
        }
    }
}
