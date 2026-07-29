/// aibody bridge module
///
/// Connects Xi (Rust) with aibody (Python).
/// Xi writes pulse_log.jsonl + learning_log.jsonl per message.
/// aibody consumes these every ~50 mins and updates genome/signals.
/// Xi reads runtime_state.json for latest emotion and genes.
///
/// File paths (standalone, not dependent on Si):
///   runtime_state: /mnt/d/xi-system\\state\\mother\\runtime_state.json
///   pulse_log:     /mnt/d/xi-system\\state\\mother\\pulse_log.jsonl
///   learning_log:  /mnt/d/xi-system\\state\\mother\\learning_log.jsonl

use chrono::Utc;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;

fn state_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XI_AIBODY_STATE") {
        return PathBuf::from(dir);
    }
    let exe_dir = std::env::current_exe().ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    exe_dir.ancestors().nth(2)
        .map(|p| p.join("state").join("mother"))
        .unwrap_or_else(|| PathBuf::from("state/mother"))
}

pub fn load_aibody_state() -> AibodySnapshot {
    let path = state_dir().join("runtime_state.json");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[aibody] Failed to read runtime_state: {}", e);
            return AibodySnapshot::default();
        }
    };

    let root: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[aibody] Failed to parse runtime_state: {}", e);
            return AibodySnapshot::default();
        }
    };

    let mut snapshot = AibodySnapshot::default();

    if let Some(signals) = root["genome"]["signals"].as_object() {
        for (k, v) in signals {
            if let Some(val) = v.as_f64() {
                snapshot.signals.insert(k.clone(), val);
            }
        }
    }

    if let Some(genes) = root["genome"]["genes"].as_object() {
        for (name, gene) in genes {
            if let Some(expr) = gene["expression"].as_f64() {
                snapshot.genes.insert(name.clone(), expr as f32);
            }
        }
        for name in &["gentleness", "attachment", "curiosity", "initiative",
                       "learning", "humor", "caution", "autonomy_bias",
                       "loyalty", "creativity"] {
            if let Some(gene) = genes.get(*name) {
                if let Some(expr) = gene["expression"].as_f64() {
                    snapshot.old_genes.insert(name.to_string(), expr as f32);
                }
            }
        }
    }

    if let Some(meta) = root["meta"].as_object() {
        snapshot.last_pulse = meta.get("last_pulse")
            .and_then(|v| v.as_str()).unwrap_or("").to_string();
        snapshot.last_saved = meta.get("last_saved_at")
            .and_then(|v| v.as_str()).unwrap_or("").to_string();
    }

    snapshot
}

pub fn write_pulse_event(entry: &ConversationEvent) {
    let path = state_dir().join("pulse_log.jsonl");
    let now = Utc::now().to_rfc3339();
    let last_tick = get_last_tick(&path);
    let tick = last_tick + 1;

    let event = json!({
        "tick": tick,
        "timestamp": now,
        "source": "xi",
        "actions": ["conversation"],
        "conversation": {
            "user_message": entry.user_text.chars().take(200).collect::<String>(),
            "reply": entry.reply_text.chars().take(200).collect::<String>(),
            "emotion_primary": entry.emotion_primary,
            "emotion_intensity": entry.emotion_intensity,
            "length": entry.user_text.len() + entry.reply_text.len(),
        }
    });

    if let Ok(json_line) = serde_json::to_string(&event) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true).append(true).open(&path)
        {
            writeln!(f, "{}", json_line).ok();
        }
    }
}

pub fn write_learning_event(topics: &[&str], messages_count: u64, summary: &str) {
    let path = state_dir().join("learning_log.jsonl");
    let now = Utc::now().to_rfc3339();

    let topics_json: Vec<String> = topics.iter().map(|t| json!(t).to_string()).collect();

    let event = json!({
        "ts": now,
        "source": "xi",
        "topics": topics_json,
        "messages_count": messages_count,
        "records_distilled": 1,
        "summary": summary,
    });

    if let Ok(json_line) = serde_json::to_string(&event) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true).append(true).open(&path)
        {
            writeln!(f, "{}", json_line).ok();
        }
    }
}

/// Bump runtime_state.json heartbeat_count and last_heartbeat (2026-07-16).
/// Called from heartbeat loop so aibody layer can see xi is alive.
pub fn bump_heartbeat(emotion_primary: &str) {
    let path = state_dir().join("runtime_state.json");
    let now = Utc::now().to_rfc3339();

    let mut root: Value = match std::fs::read_to_string(&path) {
        Ok(c) => serde_json::from_str(&c).unwrap_or_else(|_| json!({})),
        Err(_) => json!({}),
    };

    let count = root.get("heartbeat_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) + 1;

    if let Some(obj) = root.as_object_mut() {
        obj.insert("heartbeat_count".into(), json!(count));
        obj.insert("last_heartbeat".into(), json!(now));
        obj.insert("emotion_state".into(), json!(emotion_primary));
    }

    if let Ok(pretty) = serde_json::to_string_pretty(&root) {
        let _ = std::fs::write(&path, pretty);
    }
}

fn get_last_tick(path: &PathBuf) -> u64 {
    if !path.exists() { return 0; }
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return 0,
    };
    let last_line = content.lines().last().unwrap_or("");
    if last_line.is_empty() { return 0; }
    if let Ok(v) = serde_json::from_str::<Value>(last_line) {
        v["tick"].as_u64().unwrap_or(0)
    } else {
        0
    }
}

pub fn trigger_aibody_sync() {
    let script = {
        let exe_dir = std::env::current_exe().ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));
        exe_dir.ancestors().nth(2)
            .map(|p| p.join("scripts").join("aibody_sync.py"))
            .unwrap_or_else(|| PathBuf::from("scripts/aibody_sync.py"))
    };
    match std::process::Command::new("python3")
        .arg(&script).arg("--auto").output()
    {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                eprintln!("[aibody] Sync OK: {}", stdout.trim().chars().take(100).collect::<String>());
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("[aibody] Sync failed: {}", stderr.chars().take(200).collect::<String>());
            }
        }
        Err(e) => eprintln!("[aibody] Cannot execute sync: {}", e),
    }
}

#[derive(Debug, Clone)]
pub struct AibodySnapshot {
    pub signals: HashMap<String, f64>,
    pub genes: HashMap<String, f32>,
    pub old_genes: HashMap<String, f32>,
    pub last_pulse: String,
    pub last_saved: String,
}

impl Default for AibodySnapshot {
    fn default() -> Self {
        Self {
            signals: HashMap::new(),
            genes: HashMap::new(),
            old_genes: HashMap::new(),
            last_pulse: String::new(),
            last_saved: String::new(),
        }
    }
}

impl AibodySnapshot {
    pub fn describe(&self) -> String {
        let mut parts = Vec::new();
        if !self.signals.is_empty() {
            let sig_str: Vec<String> = self.signals.iter()
                .map(|(k, v)| format!("{}={:.2}", k, v))
                .collect();
            parts.push(format!("[aibody signals] {}", sig_str.join(" | ")));
        }
        if !self.old_genes.is_empty() {
            let gene_str: Vec<String> = self.old_genes.iter()
                .map(|(k, v)| format!("{}:{:.0}%", k, v * 100.0))
                .collect();
            parts.push(format!("[personality genes] {}", gene_str.join(" ")));
        } else if !self.genes.is_empty() {
            let gene_str: Vec<String> = self.genes.iter()
                .map(|(k, v)| format!("{}={:.2}", k, v))
                .collect();
            parts.push(format!("[aibody genes] {}", gene_str.join(" | ")));
        }
        if !self.last_pulse.is_empty() {
            parts.push(format!("[last pulse] {}", &self.last_pulse.chars().take(16).collect::<String>()));
        }
        parts.join("")
    }

    pub fn is_alive(&self) -> bool {
        !self.genes.is_empty() || !self.old_genes.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct ConversationEvent {
    pub user_text: String,
    pub reply_text: String,
    pub emotion_primary: String,
    pub emotion_intensity: f64,
}

impl ConversationEvent {
    pub fn new(user: &str, reply: &str, emotion: &str, intensity: f64) -> Self {
        Self {
            user_text: user.to_string(),
            reply_text: reply.to_string(),
            emotion_primary: emotion.to_string(),
            emotion_intensity: intensity,
        }
    }
}
