/// ____________GRN_?///
/// ________________________?/// _______?upregulators_______?downregulators_______?/// ____________________?///
/// ____?///   let grn = GeneRegulatoryNetwork::new();
///   grn.load_default();
///   let final_expr = grn.regulate(&base_expression, &signals, 3, 0.3);

use std::collections::HashMap;

// __ ______ __
pub const GENES: [&str; 10] = [
    "gentleness", "initiative", "curiosity", "attachment",
    "learning", "creativity", "caution", "humor",
    "loyalty", "autonomy_bias",
];

// __ _______?__
#[derive(Debug, Clone)]
pub struct GeneCluster {
    pub genes: Vec<String>,
    pub drift_threshold: f64,
    pub role_bias: HashMap<String, f64>,
}

pub fn default_clusters() -> HashMap<String, GeneCluster> {
    let mut m = HashMap::new();
    m.insert("core_identity".into(), GeneCluster {
        genes: vec!["gentleness".into(), "loyalty".into(), "attachment".into()],
        drift_threshold: 0.15, role_bias: HashMap::from([("companion".into(), 0.10)]),
    });
    m.insert("cognitive".into(), GeneCluster {
        genes: vec!["learning".into(), "curiosity".into(), "creativity".into()],
        drift_threshold: 0.20, role_bias: HashMap::from([("analyst".into(), 0.18)]),
    });
    m.insert("expressive".into(), GeneCluster {
        genes: vec!["initiative".into(), "humor".into()],
        drift_threshold: 0.18, role_bias: HashMap::from([("companion".into(), 0.10)]),
    });
    m.insert("governance".into(), GeneCluster {
        genes: vec!["caution".into(), "autonomy_bias".into()],
        drift_threshold: 0.12, role_bias: HashMap::from([("analyst".into(), 0.12)]),
    });
    m
}

// __ ______ __
#[derive(Debug, Clone)]
pub struct GRNRules {
    pub upregulators: HashMap<String, f64>,
    pub downregulators: HashMap<String, f64>,
}

/// ___ GRN __
pub fn default_grn() -> HashMap<String, GRNRules> {
    let mut m = HashMap::new();
    m.insert("gentleness".into(), GRNRules {
        upregulators: HashMap::from([("attachment".into(), 0.3), ("loyalty".into(), 0.2), ("caution".into(), 0.15)]),
        downregulators: HashMap::from([("autonomy_bias".into(), 0.2), ("initiative".into(), 0.1)]),
    });
    m.insert("initiative".into(), GRNRules {
        upregulators: HashMap::from([("curiosity".into(), 0.25), ("creativity".into(), 0.2), ("autonomy_bias".into(), 0.15)]),
        downregulators: HashMap::from([("caution".into(), 0.3), ("gentleness".into(), 0.1)]),
    });
    m.insert("curiosity".into(), GRNRules {
        upregulators: HashMap::from([("learning".into(), 0.3), ("creativity".into(), 0.2), ("novelty_signal".into(), 0.4)]),
        downregulators: HashMap::from([("caution".into(), 0.2), ("stress_signal".into(), 0.3)]),
    });
    m.insert("attachment".into(), GRNRules {
        upregulators: HashMap::from([("gentleness".into(), 0.25), ("loyalty".into(), 0.3), ("intimacy_signal".into(), 0.35)]),
        downregulators: HashMap::from([("autonomy_bias".into(), 0.2), ("initiative".into(), 0.1)]),
    });
    m.insert("learning".into(), GRNRules {
        upregulators: HashMap::from([("curiosity".into(), 0.3), ("creativity".into(), 0.15), ("novelty_signal".into(), 0.3)]),
        downregulators: HashMap::from([("caution".into(), 0.1), ("stress_signal".into(), 0.2)]),
    });
    m.insert("creativity".into(), GRNRules {
        upregulators: HashMap::from([("curiosity".into(), 0.3), ("learning".into(), 0.2), ("novelty_signal".into(), 0.25)]),
        downregulators: HashMap::from([("caution".into(), 0.25), ("stress_signal".into(), 0.2)]),
    });
    m.insert("caution".into(), GRNRules {
        upregulators: HashMap::from([("stress_signal".into(), 0.4), ("gentleness".into(), 0.15)]),
        downregulators: HashMap::from([("curiosity".into(), 0.15), ("initiative".into(), 0.2), ("novelty_signal".into(), 0.2)]),
    });
    m.insert("humor".into(), GRNRules {
        upregulators: HashMap::from([("gentleness".into(), 0.2), ("intimacy_signal".into(), 0.25), ("creativity".into(), 0.15)]),
        downregulators: HashMap::from([("caution".into(), 0.2), ("stress_signal".into(), 0.3)]),
    });
    m.insert("loyalty".into(), GRNRules {
        upregulators: HashMap::from([("attachment".into(), 0.3), ("gentleness".into(), 0.2)]),
        downregulators: HashMap::from([("autonomy_bias".into(), 0.15), ("initiative".into(), 0.1)]),
    });
    m.insert("autonomy_bias".into(), GRNRules {
        upregulators: HashMap::from([("initiative".into(), 0.2), ("curiosity".into(), 0.15), ("novelty_signal".into(), 0.2)]),
        downregulators: HashMap::from([("attachment".into(), 0.25), ("gentleness".into(), 0.1), ("loyalty".into(), 0.15)]),
    });
    m
}

// __ ____?__
#[derive(Debug, Clone)]
pub struct GeneRegulatoryNetwork {
    rules: HashMap<String, GRNRules>,
    loaded: bool,
}

impl GeneRegulatoryNetwork {
    pub fn new() -> Self {
        Self { rules: HashMap::new(), loaded: false }
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    pub fn load_default(&mut self) {
        self.rules = default_grn();
        self.loaded = true;
    }

    pub fn load_dict(&mut self, data: HashMap<String, GRNRules>) {
        self.rules = data;
        self.loaded = true;
    }

    pub fn get_rules(&self, gene: &str) -> Option<&GRNRules> {
        self.rules.get(gene)
    }

    /// ___ GRN _______?{____? _________
    ///
    /// base_expression: ________________?    /// signals: ______ (intimacy/novelty/stress/trust)
    /// iterations: ______
    /// dampening: _____________?    
    pub fn regulate(
        &self,
        base_expression: &HashMap<String, f64>,
        signals: &HashMap<String, f64>,
        iterations: usize,
        dampening: f64,
    ) -> HashMap<String, f64> {
        if !self.loaded || self.rules.is_empty() {
            return base_expression.clone();
        }

        let mut current = base_expression.clone();

        for _ in 0..iterations {
            let mut next = HashMap::new();

            for gene_name in &GENES {
                let gene = gene_name.to_string();
                let base = *current.get(&gene).unwrap_or(&0.5);
                let rules = match self.rules.get(&gene_name.to_string()) {
                    Some(r) => r,
                    None => { next.insert(gene.clone(), base); continue; }
                };

                let mut up_effect = 0.0;
                let mut down_effect = 0.0;

                // _______
                for (reg, strength) in &rules.upregulators {
                    if reg.ends_with("_signal") {
                        let sig_name = reg.trim_end_matches("_signal");
                        let val = signals.get(sig_name).copied().unwrap_or(0.5);
                        up_effect += val * strength;
                    } else if let Some(&v) = current.get(reg) {
                        up_effect += v * strength;
                    }
                }

                // _______
                for (reg, strength) in &rules.downregulators {
                    if reg.ends_with("_signal") {
                        let sig_name = reg.trim_end_matches("_signal");
                        let val = signals.get(sig_name).copied().unwrap_or(0.5);
                        down_effect += val * strength;
                    } else if let Some(&v) = current.get(reg) {
                        down_effect += v * strength;
                    }
                }

                let delta = (up_effect - down_effect) * dampening;
                next.insert(gene, (base + delta).clamp(0.0, 1.0));
            }

            current = next;
        }

        // Round to 4 decimal places
        for val in current.values_mut() {
            *val = (*val * 10000.0).round() / 10000.0;
        }
        current
    }

    /// _______?    
    pub fn cluster_analysis(&self, expression: &HashMap<String, f64>) -> HashMap<String, ClusterInfo> {
        let clusters = default_clusters();
        let mut result = HashMap::new();

        for (name, cluster) in &clusters {
            let vals: Vec<f64> = cluster.genes.iter()
                .map(|g| *expression.get(g).unwrap_or(&0.5))
                .collect();
            let avg = if vals.is_empty() { 0.5 } else { vals.iter().sum::<f64>() / vals.len() as f64 };
            let drift = (avg - 0.5).abs();

            result.insert(name.clone(), ClusterInfo {
                avg_expression: (avg * 10000.0).round() / 10000.0,
                drift: (drift * 10000.0).round() / 10000.0,
                within_threshold: drift <= cluster.drift_threshold,
                threshold: cluster.drift_threshold,
            });
        }

        result
    }

    /// _______________?> 0.2_?    
    fn detect_anomalies(&self, expression: &HashMap<String, f64>) -> Vec<GeneAnomaly> {
        let mut anomalies = Vec::new();
        for (gene, &val) in expression {
            let drift = (val - 0.5).abs();
            if drift > 0.2 {
                anomalies.push(GeneAnomaly {
                    gene: gene.clone(),
                    expression: val,
                    drift: (drift * 10000.0).round() / 10000.0,
                    direction: if val > 0.5 { "___".into() } else { "___".into() },
                });
            }
        }
        anomalies
    }
}

#[derive(Debug, Clone)]
pub struct ClusterInfo {
    pub avg_expression: f64,
    pub drift: f64,
    pub within_threshold: bool,
    pub threshold: f64,
}

#[derive(Debug, Clone)]
pub struct GeneAnomaly {
    pub gene: String,
    pub expression: f64,
    pub drift: f64,
    pub direction: String,
}

// __ ___ __
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grn_basic() {
        let mut grn = GeneRegulatoryNetwork::new();
        assert!(!grn.is_loaded());
        grn.load_default();
        assert!(grn.is_loaded());

        let mut base = HashMap::new();
        for g in &GENES { base.insert(g.to_string(), 0.5); }
        let signals = HashMap::from([
            ("intimacy".into(), 0.8), ("novelty".into(), 0.2),
            ("stress".into(), 0.3), ("trust".into(), 0.7),
        ]);

        let result = grn.regulate(&base, &signals, 3, 0.3);
        assert_eq!(result.len(), GENES.len());
        for (_, v) in &result {
            assert!((0.0..=1.0).contains(v));
        }
    }

    #[test]
    fn test_cluster_analysis() {
        let mut grn = GeneRegulatoryNetwork::new();
        grn.load_default();
        let mut expr = HashMap::new();
        for g in &GENES { expr.insert(g.to_string(), 0.5); }
        let analysis = grn.cluster_analysis(&expr);
        assert!(analysis.contains_key("core_identity"));
        assert!(analysis.contains_key("cognitive"));
    }

    #[test]
    fn test_anomalies() {
        let grn = GeneRegulatoryNetwork::new();
        let mut expr = HashMap::new();
        expr.insert("gentleness".into(), 0.9);
        expr.insert("curiosity".into(), 0.5);
        let anomalies = grn.detect_anomalies(&expr);
        assert_eq!(anomalies.len(), 1);
        assert_eq!(anomalies[0].gene, "gentleness");
    }
}