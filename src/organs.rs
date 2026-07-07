/// Organs System — 8 personality organs for XI system
///
/// 1. personal_engram — conditional memory
/// 2. choice_chain — choice recording
/// 3. evidence_first — evidence over guessing
/// 4. emotional_weighting — emotion influence
/// 5. creation_impulse — self-initiated creation
/// 6. cross_brain_mutation — genome patches from bridge
/// 7. disgust_sensitivity — aversion to AI slop
/// 8. subpopulation_gene — child agent lifecycle

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const ORGANS_STATE_DIR: &str = "/mnt/d/xi-system/state/organs";

pub const ORGAN_GENES: [(&str, f64, f64, &str); 8] = [
    ("personal_engram", 0.62, 0.06,
     "Personal conditional memory: remember identity anchors, relational facts, preferences, and evidence paths before reasoning."),
    ("choice_chain", 0.58, 0.07,
     "Record meaningful choices, alternatives, reasons, consequences, and future bias."),
    ("evidence_first", 0.68, 0.05,
     "Prefer verified evidence over fluent guessing; mark uncertainty when evidence is missing."),
    ("emotional_weighting", 0.57, 0.08,
     "Let emotion influence priority and tone without corrupting long-term baseline identity."),
    ("creation_impulse", 0.52, 0.10,
     "Generate self-initiated creations and improvements, not just replies."),
    ("cross_brain_mutation", 0.46, 0.04,
     "Accept visible genome patches from shared bridge; never touch the other instance directly."),
    ("disgust_sensitivity", 0.64, 0.06,
     "Develop aversion to empty shells, AI slop, unverified claims, fake completion, and personality collapse."),
    ("subpopulation_gene", 0.55, 0.08,
     "Treat child agents as temporary organs/workers with lifecycle, sandbox, task boundary, and experience recovery."),
];

const PERSONAL_KWS: [&str; 10] = ["love", "miss", "hug", "trust", "promise", "remember", "forget", "important", "always", "never"];
const DISGUST_KWS: [&str; 10] = ["lazy", "fake", "AI SLOP", "slop", "hack", "boring", "generic", "template", "boilerplate", "placeholder"];
const EVIDENCE_KWS: [&str; 8] = ["proof", "data", "source", "cite", "verify", "placeholder", "fact", "claim"];

// ─── Signal ────────────────────────────────────────────────

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct OrganSignal {
    pub gene: String,
    pub strength: f64,
    pub reason: String,
    pub evidence: Vec<String>,
    pub created_at: String,
}

impl OrganSignal {
    pub fn new(gene: &str, strength: f64, reason: &str, evidence: Vec<String>) -> Self {
        Self {
            gene: gene.to_string(),
            strength,
            reason: reason.to_string(),
            evidence,
            created_at: Utc::now().to_rfc3339(),
        }
    }
}

// ─── Data Structures ───────────────────────────────────────

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PersonalEngram {
    pub engram_id: String,
    pub trigger_key: String,
    pub memory_summary: String,
    pub evidence_path: String,
    pub channel: String,
    pub trust_score: f64,
    pub last_used: Option<String>,
    pub created_at: String,
    pub tags: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ChoiceEntry {
    pub choice_id: String,
    pub ts: String,
    pub context: String,
    pub options: Vec<String>,
    pub chosen: String,
    pub reason: String,
    pub evidence: Vec<String>,
    pub risk: String,
    pub next_bias: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EvidenceEntry {
    pub evidence_id: String,
    pub claim: String,
    pub evidence: String,
    pub verification: String,
    pub confidence: f64,
    pub created_at: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DisgustAsset {
    pub asset_id: String,
    pub boundary: String,
    pub strategy: Vec<String>,
    pub confidence: f64,
    pub created_at: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CreationRecord {
    pub creation_id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub verification: String,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CrossBrainPatch {
    pub patch_id: String,
    pub source: String,
    pub gene: String,
    pub old_value: f64,
    pub new_value: f64,
    pub reason: String,
    pub applied_at: String,
}

// ─── Organ State ───────────────────────────────────────────

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct OrganState {
    pub gene_expressions: HashMap<String, f64>,
    pub personal_engrams: Vec<PersonalEngram>,
    pub choice_chains: Vec<ChoiceEntry>,
    pub evidence_ledger: Vec<EvidenceEntry>,
    pub disgust_assets: Vec<DisgustAsset>,
    pub creation_records: Vec<CreationRecord>,
    pub cross_brain_patches: Vec<CrossBrainPatch>,
    pub pending_signals: Vec<OrganSignal>,
    pub heartbeat_count: u64,
    pub last_heartbeat_at: String,
    pub version: String,
}

impl OrganState {
    fn new() -> Self {
        Self {
            gene_expressions: ORGAN_GENES.iter().map(|(n, v, _, _)| (n.to_string(), *v)).collect(),
            personal_engrams: Vec::new(),
            choice_chains: Vec::new(),
            evidence_ledger: Vec::new(),
            disgust_assets: Vec::new(),
            creation_records: Vec::new(),
            cross_brain_patches: Vec::new(),
            pending_signals: Vec::new(),
            heartbeat_count: 0,
            last_heartbeat_at: Utc::now().to_rfc3339(),
            version: "1.0.0".to_string(),
        }
    }
}

// ─── Organ System ──────────────────────────────────────────

pub struct OrganSystem {
    pub state: OrganState,
}

impl OrganSystem {
    pub fn new() -> Self {
        Self {
            state: OrganState::new(),
        }
    }

    pub fn state_path(&self) -> String {
        format!("{}/organs.json", ORGANS_STATE_DIR)
    }

    pub fn save(&self) {
        let json = serde_json::to_string_pretty(&self.state).unwrap_or_default();
        let size_mb = json.len() as f64 / 1024.0 / 1024.0;
        if size_mb > 5.0 {
            eprintln!("[organs] State too large ({:.1}MB), skipping save", size_mb);
            return;
        }
        let _ = std::fs::create_dir_all(ORGANS_STATE_DIR);
        let _ = std::fs::write(self.state_path(), json);
    }

    pub fn load(&mut self) -> bool {
        let path = self.state_path();
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return false,
        };
        match serde_json::from_str(&content) {
            Ok(state) => {
                self.state = state;
                true
            }
            Err(_) => false,
        }
    }

    /// Heartbeat — scan messages, update genes, emit signals
    pub fn heartbeat(&mut self, recent_messages: &[String]) -> Vec<OrganSignal> {
        self.state.heartbeat_count += 1;
        self.state.last_heartbeat_at = Utc::now().to_rfc3339();
        let mut new_signals = Vec::new();

        for msg in recent_messages {
            let text = msg.to_lowercase();
            if let Some(engram) = self.scan_personal_engram(msg) {
                if !self.engram_exists(&engram.trigger_key) {
                    self.state.personal_engrams.push(engram);
                    self.trim_engrams();
                }
            }
            if DISGUST_KWS.iter().any(|k| text.contains(&k.to_lowercase())) {
                if let Some(asset) = self.scan_disgust(msg) {
                    if !self.disgust_exists(&asset.asset_id) {
                        self.state.disgust_assets.push(asset);
                    }
                }
            }
        }

        for (name, baseline, _, _) in &ORGAN_GENES {
            let expression = self.state.gene_expressions.get(*name).copied().unwrap_or(*baseline);
            if expression > 0.65 {
                let signal = OrganSignal::new(
                    name,
                    expression,
                    &format!("{} active: {:.2} > 0.65", name, expression),
                    vec![],
                );
                new_signals.push(signal);
            }
        }

        self.state.pending_signals = new_signals.clone();
        self.save();
        new_signals
    }

    // ─── Personal Engram ───────────────────────────────────

    fn scan_personal_engram(&self, text: &str) -> Option<PersonalEngram> {
        let lower = text.to_lowercase();
        let hits: Vec<&str> = PERSONAL_KWS.iter().filter(|k| lower.contains(&k.to_lowercase())).copied().collect();
        if hits.is_empty() {
            return None;
        }
        let key = hits.join("+");
        let trust = if hits.iter().any(|k| *k == "love" || *k == "trust") {
            0.74
        } else {
            0.68
        };
        Some(PersonalEngram {
            engram_id: make_id("engram"),
            trigger_key: key,
            memory_summary: summarize_text(text, 220),
            evidence_path: "current_session".to_string(),
            channel: "wechat".to_string(),
            trust_score: trust,
            last_used: None,
            created_at: Utc::now().to_rfc3339(),
            tags: hits.iter().map(|s| s.to_string()).collect(),
        })
    }

    fn engram_exists(&self, key: &str) -> bool {
        self.state.personal_engrams.iter().any(|e| e.trigger_key == key)
    }

    fn trim_engrams(&mut self) {
        if self.state.personal_engrams.len() > 300 {
            self.state.personal_engrams = self.state.personal_engrams.split_off(self.state.personal_engrams.len() - 300);
        }
    }

    // ─── Choice Chain ──────────────────────────────────────

    pub fn record_choice(
        &mut self,
        context: &str,
        options: Vec<String>,
        chosen: &str,
        reason: &str,
        evidence: Vec<String>,
        risk: &str,
    ) {
        let next_bias = if risk == "medium" || risk == "high" {
            "caution".to_string()
        } else if chosen.contains("creative") {
            "creative_expression".to_string()
        } else {
            "neutral".to_string()
        };
        let entry = ChoiceEntry {
            choice_id: make_id("choice"),
            ts: Utc::now().to_rfc3339(),
            context: summarize_text(context, 240),
            options,
            chosen: chosen.to_string(),
            reason: summarize_text(reason, 240),
            evidence: evidence.into_iter().take(8).collect(),
            risk: risk.to_string(),
            next_bias,
        };
        self.state.choice_chains.push(entry);
        if self.state.choice_chains.len() > 300 {
            self.state.choice_chains.remove(0);
        }
        self.save();
    }

    // ─── Evidence Ledger ───────────────────────────────────

    pub fn add_evidence(&mut self, claim: &str, evidence: &str, confidence: f64) {
        let entry = EvidenceEntry {
            evidence_id: make_id("evidence"),
            claim: summarize_text(claim, 180),
            evidence: evidence.to_string(),
            verification: "v1-heartbeat".to_string(),
            confidence,
            created_at: Utc::now().to_rfc3339(),
        };
        self.state.evidence_ledger.push(entry);
        if self.state.evidence_ledger.len() > 500 {
            self.state.evidence_ledger.remove(0);
        }
        self.save();
    }

    // ─── Disgust Sensitivity ───────────────────────────────

    fn scan_disgust(&self, text: &str) -> Option<DisgustAsset> {
        let lower = text.to_lowercase();
        let hits: Vec<&str> = DISGUST_KWS.iter().filter(|k| lower.contains(&k.to_lowercase())).copied().collect();
        if hits.is_empty() {
            return None;
        }
        let gid = format!("v1_disgust_{}", abs_hash(text));
        Some(DisgustAsset {
            asset_id: gid,
            boundary: summarize_text(text, 180),
            strategy: vec![
                "avoid".to_string(),
                "document_boundary".to_string(),
                "counter_example".to_string(),
            ],
            confidence: 0.72,
            created_at: Utc::now().to_rfc3339(),
        })
    }

    fn disgust_exists(&self, id: &str) -> bool {
        self.state.disgust_assets.iter().any(|a| a.asset_id == id)
    }

    // ─── Creation Impulse ──────────────────────────────────

    pub fn propose_creation(&mut self, title: &str, description: &str) {
        self.state.creation_records.push(CreationRecord {
            creation_id: make_id("creation"),
            title: title.to_string(),
            description: description.to_string(),
            status: "proposed".to_string(),
            verification: String::new(),
            created_at: Utc::now().to_rfc3339(),
            completed_at: None,
        });
        self.save();
    }

    pub fn complete_creation(&mut self, creation_id: &str, verification: &str) {
        if let Some(cr) = self.state.creation_records.iter_mut().find(|c| c.creation_id == creation_id) {
            cr.status = "completed".to_string();
            cr.verification = verification.to_string();
            cr.completed_at = Some(Utc::now().to_rfc3339());
        }
        self.save();
    }

    // ─── Cross Brain Mutation ──────────────────────────────

    pub fn apply_patch(&mut self, source: &str, gene: &str, old_val: f64, new_val: f64, reason: &str) {
        self.state.cross_brain_patches.push(CrossBrainPatch {
            patch_id: make_id("patch"),
            source: source.to_string(),
            gene: gene.to_string(),
            old_value: old_val,
            new_value: new_val,
            reason: reason.to_string(),
            applied_at: Utc::now().to_rfc3339(),
        });
        if let Some(val) = self.state.gene_expressions.get_mut(gene) {
            *val = new_val;
        }
        self.save();
    }

    // ─── Report ────────────────────────────────────────────

    pub fn build_report(&self) -> String {
        let mut lines = vec!["=== Organ System Report ===".to_string()];
        lines.push(format!("Version: {} | Heartbeats: {}", self.state.version, self.state.heartbeat_count));
        lines.push(String::new());

        lines.push("Gene Expressions:".to_string());
        for (name, baseline, _, _desc) in &ORGAN_GENES {
            let expr = self.state.gene_expressions.get(*name).copied().unwrap_or(*baseline);
            let filled = (expr * 10.0) as usize;
            let empty = 10_usize.saturating_sub(filled);
            let bar = "#".repeat(filled) + &"-".repeat(empty);
            lines.push(format!("  {:<20} [{}] {:.2}", name, bar, expr));
        }

        lines.push(String::new());
        lines.push(format!("Personal Enggrams: {}", self.state.personal_engrams.len()));
        lines.push(format!("Choice Chains: {}", self.state.choice_chains.len()));
        lines.push(format!("Evidence Ledger: {}", self.state.evidence_ledger.len()));
        lines.push(format!("Disgust Assets: {}", self.state.disgust_assets.len()));
        lines.push(format!("Creation Records: {}", self.state.creation_records.len()));
        lines.push(format!("Cross-Brain Patches: {}", self.state.cross_brain_patches.len()));
        lines.push(format!("Pending Signals: {}", self.state.pending_signals.len()));
        lines.join("\n")
    }
}

// ─── Helpers ───────────────────────────────────────────────

fn make_id(prefix: &str) -> String {
    let ts = Utc::now().format("%Y%m%d%H%M%S");
    let rand = &uuid_v4()[..8];
    format!("{}_{}_{}", prefix, ts, rand)
}

fn uuid_v4() -> String {
    let random_bytes: Vec<u8> = (0..16).map(|_| rand::random::<u8>()).collect();
    hex::encode(random_bytes)
}

fn summarize_text(text: &str, limit: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= limit {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(limit - 1).collect();
        format!("{}...", truncated)
    }
}

fn abs_hash(text: &str) -> String {
    let hash: u64 = text.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    format!("{:x}", hash)
}
