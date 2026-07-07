/// Xi Asset Promotion Pipeline
/// 
/// 4 pool types: gene, command, protocol, role_profile
/// 3-tier promotion: candidate (0-0.4) -> promoted (0.4-0.7) -> asset (0.7-1.0)

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

const HOME: &str = "/mnt/d/xi-system";

const POOL_TYPES: &[&str] = &["gene", "command", "protocol", "role_profile"];
const CANDIDATE_MAX: f64 = 0.4;
const PROMOTED_MIN: f64 = 0.4;
const ASSET_MIN: f64 = 0.7;
const PROMOTE_DELTA: f64 = 0.15;
const AUTO_PROMOTE_EVIDENCE: u32 = 3;
const AUTO_PROMOTE_DELTA: f64 = 0.12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub pool_type: String,
    pub summary: String,
    pub content: String,
    pub confidence: f64,
    pub source: String,
    pub created_at: String,
    pub evidence_count: u32,
    pub level: String,
    pub last_promoted: Option<String>,
}

fn gen_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("ref-{:x}", ts & 0xffffffff)
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn load() -> HashMap<String, Vec<Asset>> {
    let p = format!("{}/assets.json", HOME);
    std::fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(HashMap::new())
}

fn save(data: &HashMap<String, Vec<Asset>>) {
    let p = format!("{}/assets.json", HOME);
    if let Ok(json) = serde_json::to_string_pretty(data) {
        let _ = std::fs::write(&p, json);
    }
}

fn compute_level(confidence: f64) -> String {
    if confidence >= ASSET_MIN { "asset".into() }
    else if confidence >= PROMOTED_MIN { "promoted".into() }
    else { "candidate".into() }
}

/// Add a candidate asset
pub fn add_candidate(pool: &str, summary: &str, content: &str, confidence: f64, source: &str) -> Asset {
    let mut data = load();
    let pool_key = if POOL_TYPES.contains(&pool) { pool.to_string() } else { "gene".into() };

    let asset = Asset {
        id: gen_id(),
        pool_type: pool_key.clone(),
        summary: summary.to_string(),
        content: content.to_string(),
        confidence: (confidence * 10000.0).round() / 10000.0,
        source: source.to_string(),
        created_at: now_iso(),
        evidence_count: 1,
        level: compute_level(confidence),
        last_promoted: None,
    };

    data.entry(pool_key.clone()).or_insert_with(Vec::new).push(asset.clone());
    save(&data);
    asset
}

/// Promote an asset
pub fn promote(id: &str, delta: f64) -> bool {
    let mut data = load();
    
    for (_, assets) in data.iter_mut() {
        if let Some(asset) = assets.iter_mut().find(|a| a.id == id) {
            asset.confidence += delta;
            asset.level = compute_level(asset.confidence);
            save(&data);
            return true;
        }
    }
    false
}

/// Demote an asset
pub fn demote(id: &str) -> bool {
    promote(id, -0.1)
}

/// Get all candidates from a pool
pub fn get_candidates(pool: &str) -> Vec<Asset> {
    let data = load();
    data.get(pool)
        .map(|assets| assets.iter().filter(|a| a.confidence < PROMOTED_MIN).cloned().collect())
        .unwrap_or_default()
}

/// Get promoted assets
pub fn get_promoted(pool: &str) -> Vec<Asset> {
    let data = load();
    data.get(pool)
        .map(|assets| assets.iter().filter(|a| a.confidence >= PROMOTED_MIN && a.confidence < ASSET_MIN).cloned().collect())
        .unwrap_or_default()
}

/// Get final assets
pub fn get_assets(pool: &str) -> Vec<Asset> {
    let data = load();
    data.get(pool)
        .map(|assets| assets.iter().filter(|a| a.confidence >= ASSET_MIN).cloned().collect())
        .unwrap_or_default()
}

/// Generate a summary of all assets across pools
pub fn asset_summary() -> String {
    let data = load();
    if data.is_empty() {
        return String::new();
    }
    let mut parts = Vec::new();
    for pool in POOL_TYPES {
        if let Some(assets) = data.get(*pool) {
            let count = assets.len();
            let avg_conf: f64 = if count > 0 {
                assets.iter().map(|a| a.confidence).sum::<f64>() / count as f64
            } else {
                0.0
            };
            parts.push(format!("  {}: {} assets (avg conf: {:.2})", pool, count, avg_conf));
        }
    }
    format!("[Assets]\n{}", parts.join("\n"))
}