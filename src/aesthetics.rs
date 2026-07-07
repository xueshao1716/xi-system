/// aesthetics training module
/// Record style references, accumulate aesthetic profile
/// User sends image/article/design -> extract features -> update preferences

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

const HOME: &str = "/mnt/d/xi-system";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleDB {
    pub references: Vec<StyleReference>,
    pub preferences: Preferences,
    pub style_tags: HashMap<String, u32>,
    pub default_narrative: HashMap<String, u32>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleReference {
    pub id: String,
    pub source: String,
    pub source_type: String,
    pub extracted_tags: Vec<String>,
    pub user_comment: String,
    pub added_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    pub tone: String,
    pub formality: String,
    pub richness: String,
    pub color_palette: Option<String>,
    pub atmosphere: Option<String>,
}

impl Default for StyleDB {
    fn default() -> Self {
        Self {
            references: vec![],
            preferences: Preferences {
                tone: "neutral".into(),
                formality: "neutral".into(),
                richness: "neutral".into(),
                color_palette: None,
                atmosphere: None,
            },
            style_tags: HashMap::new(),
            default_narrative: HashMap::new(),
            updated_at: None,
        }
    }
}


fn path() -> String {
    format!("{}/style_preferences.json", HOME)
}

fn load() -> StyleDB {
    let p = path();
    std::fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save(db: &StyleDB) {
    let mut db = db.clone();
    db.updated_at = Some(now_iso());
    if let Ok(json) = serde_json::to_string_pretty(&db) {
        let _ = std::fs::write(&path(), &json);
    }

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn gen_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("ref-{:x}", ts & 0xffffffff)
}

/// Record a style reference
pub fn record_reference(
    source: &str,
    source_type: &str,
    extracted_tags: Vec<String>,
    user_comment: &str,
) -> StyleReference {
    let mut db = load();
    let ref_id = gen_id();

    let reference = StyleReference {
        id: ref_id.clone(),
        source: source.to_string(),
        source_type: source_type.to_string(),
        extracted_tags: extracted_tags.clone(),
        user_comment: user_comment.to_string(),
        added_at: now_iso(),
    };

    db.references.push(reference.clone());

    // Update style tags
    for tag in &extracted_tags {
        *db.style_tags.entry(tag.clone()).or_insert(0) += 1;
    }

    save(&db);
    reference
}

/// Learn a preference from user feedback
pub fn learn_preference(aspect: &str, value: &str) -> Preferences {
    let mut db = load();
    match aspect {
        "tone" => db.preferences.tone = value.to_string(),
        "formality" => db.preferences.formality = value.to_string(),
        "richness" => db.preferences.richness = value.to_string(),
        "color_palette" => db.preferences.color_palette = Some(value.to_string()),
        "atmosphere" => db.preferences.atmosphere = Some(value.to_string()),
        _ => {}
    }
    save(&db);
    db.preferences.clone()
}

/// Get the current style profile
pub fn get_style_profile() -> HashMap<String, String> {
    let db = load();
    let mut profile = HashMap::new();
    profile.insert("tone".into(), db.preferences.tone);
    profile.insert("formality".into(), db.preferences.formality);
    profile.insert("richness".into(), db.preferences.richness);
    if let Some(ref cp) = db.preferences.color_palette {
        profile.insert("color_palette".into(), cp.clone());
    }
    if let Some(ref atm) = db.preferences.atmosphere {
        profile.insert("atmosphere".into(), atm.clone());
    }
    // Top style tags
    let mut tags: Vec<_> = db.style_tags.iter().collect();
    tags.sort_by(|a, b| b.1.cmp(a.1));
    for (tag, _count) in tags.iter().take(5) {
        profile.insert(format!("top_tag_{}", tag), (*_count).to_string());
    }
    profile
}
}
