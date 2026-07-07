/// _______?_?VAD ___
///
/// Valence(-1~1), Arousal(0~1), Dominance(0~1)
/// ______ + ______ + ______
/// ____?emotion.json ___

use chrono::{Local, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// VAD _________
const EMOTIONS: &[(&str, f64, f64, f64)] = &[
    ("loving",   0.8, 0.3, 0.6),
    ("happy",    0.7, 0.6, 0.5),
    ("curious",  0.5, 0.7, 0.4),
    ("playful",  0.6, 0.7, 0.6),
    ("calm",     0.5, 0.2, 0.5),
    ("anxious", -0.3, 0.7, -0.2),
    ("sad",     -0.6,-0.3,-0.3),
    ("angry",   -0.5, 0.6, 0.2),
    ("tired",   -0.2,-0.5,-0.3),
    ("neutral",  0.0, 0.0, 0.0),
];

/// _______?_?VAD ___
fn input_keywords() -> HashMap<&'static str, (f64, f64, f64)> {
    let mut m = HashMap::new();
    m.insert("___",     (0.15, 0.05, 0.1));
    m.insert("___",     (0.3,  0.1,  0.0));
    m.insert("___",     (0.2,  0.15, 0.0));
    m.insert("sad",      (0.15, -0.1, -0.1));
    m.insert("happy",    (0.2,  0.1,  0.15));
    m.insert("angry",    (-0.2, 0.1, -0.15));
    m.insert("__",     (-0.2, 0.05, -0.1));
    m.insert("excited",  (0.0,  0.2,  0.0));
    m.insert("___",     (0.1,  0.15, 0.1));
    m.insert("tired",    (-0.15, 0.1, -0.1));
    m
}

/// Lasting emotional impact from significant moments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionResidue {
    /// Positive residue: warmth from loving/kind interactions
    pub warmth: f64,
    /// Negative residue: hurt from conflict/betrayal
    pub hurt: f64,
    /// Curiosity residue: accumulated from learning/exploration
    pub curiosity: f64,
    /// Last significant emotional event
    pub last_event: String,
    /// Event timestamp
    pub last_event_time: String,
}

impl Default for EmotionResidue {
    fn default() -> Self {
        Self {
            warmth: 0.0,
            hurt: 0.0,
            curiosity: 0.0,
            last_event: String::new(),
            last_event_time: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionSnapshot {
    pub time: String,
    pub primary: String,
    pub intensity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionState {
    #[serde(default = "default_valence")]
    pub valence: f64,
    #[serde(default = "default_arousal")]
    pub arousal: f64,
    #[serde(default = "default_dominance")]
    pub dominance: f64,
    pub primary: String,
    pub intensity: f64,
    pub secondary: String,
    pub last_update: String,
    #[serde(default)]
    pub emotion_history: std::collections::VecDeque<EmotionSnapshot>,
    /// Accumulated emotional weight from past interactions (persists across sessions)
    #[serde(default)]
    pub emotional_residue: EmotionResidue,
}

fn default_valence() -> f64 { 0.5 }
fn default_arousal() -> f64 { 0.3 }
fn default_dominance() -> f64 { 0.5 }
/// Personality baseline — 曦 decays toward warm+calm, not cold neutral
fn personality_valence() -> f64 { 0.55 }
fn personality_arousal() -> f64 { 0.35 }
fn personality_dominance() -> f64 { 0.5 }

/// ____________ emotion_history.jsonl ____
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealFeeling {
    pub ts: String,
    pub event: String,
    pub felt: String,
    pub intensity: f64,
}

/// _?emotion_history.jsonl ______________
/// _____________?JSON____?ts/event/felt/intensity
pub fn load_real_feelings(path: &str) -> Vec<RealFeeling> {
    if !Path::new(path).exists() {
        return Vec::new();
    }
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// Record a real feeling to emotion_history.jsonl
pub fn record_feeling(path: &str, event: &str, felt: &str, intensity: f64) {
    let feeling = RealFeeling {
        ts: Utc::now().to_rfc3339(),
        event: event.to_string(),
        felt: felt.to_string(),
        intensity,
    };
    if let Ok(line) = serde_json::to_string(&feeling) {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap_or_else(|e| {
                eprintln!("[emotion] failed to open {}: {}", path, e);
                std::fs::File::create(path).unwrap_or_else(|e2| {
                    eprintln!("[emotion] FATAL: cannot create {}: {}", path, e2);
                    // Return a dummy file that will silently drop writes
                    std::fs::File::open("/dev/null").expect("/dev/null always exists")
                })
            });
        let _ = writeln!(file, "{}", line);
    }
}

impl EmotionState {
    pub fn new() -> Self {
        Self {
            valence: 0.5,
            arousal: 0.3,
            dominance: 0.5,
            primary: "calm".into(),
            intensity: 0.1,
            secondary: "loving".into(),
            last_update: Utc::now().to_rfc3339(),
            emotion_history: std::collections::VecDeque::new(),
            emotional_residue: EmotionResidue::default(),
        }
    }

    pub fn load(path: &str) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
                eprintln!("[emotion] load error: {}", e);
                Self::new()
            }),
            Err(_) => {
                eprintln!("[emotion] updated");
                Self::new()
            }
        }
    }

    pub fn save(&self, path: &str) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let size_mb = json.len() as f64 / 1024.0 / 1024.0;
            if size_mb > 5.0 {
                eprintln!("[emotion] skip save: too large ({:.1}MB)", size_mb);
                return;
            }
            let _ = std::fs::write(path, json);
        }
    }

    /// _______________
    pub fn update_from_input(&mut self, text: &str) {
        let now = Utc::now().to_rfc3339();
        let local_hour = Local::now().hour();

        // ________ arousal____
        let time_arousal = match local_hour {
            6..=11 => 0.03,
            12..=17 => 0.01,
            18..=22 => 0.0,
            _ => -0.02,
        };
        let time_valence = match local_hour {
            6..=11 => 0.02,
            _ => 0.0,
        };

        // _______?
        let kw = input_keywords();
        let mut dv = 0.0f64;
        let mut da = 0.0f64;
        let mut dd = 0.0f64;
        for (word, (v, a, d)) in &kw {
            if text.contains(word) {
                dv += v;
                da += a;
                dd += d;
            }
        }

        // Len factor — smaller impact
        let len_factor = (text.len() as f64).min(200.0) / 200.0 * 0.02;

        // Clamp individual shifts
        let dv = dv.clamp(-0.15, 0.15);
        let da = da.clamp(-0.15, 0.15);

        self.valence = (self.valence + dv + time_valence - len_factor * 0.5).clamp(-1.0, 1.0);
        self.arousal = (self.arousal + da + time_arousal + len_factor * 0.3).clamp(0.0, 1.0);
        self.dominance = (self.dominance + dd).clamp(0.0, 1.0);

        // Hard cap after every update
        if self.valence > 0.8 { self.valence = 0.8; }
        if self.arousal > 0.8 { self.arousal = 0.8; }

        self.update_label(now);
    }

    /// _______________
    pub fn update_from_output(&mut self, text: &str) {
        let now = Utc::now().to_rfc3339();

        // Output — small arousal shift
        let len_arousal = (text.len() as f64).min(300.0) / 300.0 * 0.03;
        let short_bonus = if text.len() < 30 { -0.02 } else { 0.0 };
        self.arousal = (self.arousal + len_arousal + short_bonus).clamp(0.0, 1.0);

        // Hard cap
        if self.valence > 0.8 { self.valence = 0.8; }
        if self.arousal > 0.8 { self.arousal = 0.8; }

        self.update_label(now);
    }

    /// ________________________?    /// _____________________________ 5 _?    
    pub fn decay(&mut self, real_feelings_path: Option<&str>) {
        let now = Utc::now().to_rfc3339();
        // Check if there are recent feelings to modulate decay rate
        let has_recent_feelings = if let Some(path) = real_feelings_path {
            let feelings = load_real_feelings(path);
            let cutoff = Utc::now() - chrono::Duration::minutes(10);
            feelings.iter().any(|f| {
                chrono::DateTime::parse_from_rfc3339(&f.ts)
                    .map(|t| t.with_timezone(&Utc) > cutoff)
                    .unwrap_or(false)
            })
        } else {
            false
        };
        // Decay toward PERSONALITY baseline, not cold neutral
        // Warm residue slows negative decay, hurt residue slows positive decay
        let residue_effect = (self.emotional_residue.warmth - self.emotional_residue.hurt) * 0.1;
        let decay_rate = if has_recent_feelings { 0.05 } else { 0.12 };
        self.valence += (personality_valence() + residue_effect - self.valence) * decay_rate;
        self.arousal += (personality_arousal() - self.arousal) * decay_rate;
        self.dominance += (personality_dominance() - self.dominance) * decay_rate;

        // Natural range — no hard cap, let emotions breathe
        self.valence = self.valence.clamp(-1.0, 1.0);
        self.arousal = self.arousal.clamp(0.0, 1.0);
        self.update_label(now);
    }

    /// ____________________
    /// _____________________ 3 _________ decay ______
    pub fn apply_real_feelings(&mut self, feelings: &[RealFeeling]) {
        if feelings.is_empty() {
            return;
        }
        let recent: Vec<_> = feelings.iter().rev().take(3).collect();
        let now_ts = Utc::now().to_rfc3339();

        for feeling in &recent {
            let strength = feeling.intensity;
            if strength > 0.5 {
                self.valence = (self.valence + 0.1 * strength).clamp(-1.0, 1.0);
                self.arousal = (self.arousal + 0.05 * strength).clamp(0.0, 1.0);
                // Accumulate warmth residue for positive feelings
                if self.valence > 0.0 {
                    self.emotional_residue.warmth = (self.emotional_residue.warmth + strength * 0.02).min(1.0);
                }
            } else {
                self.valence = (self.valence - 0.03).clamp(-1.0, 1.0);
                // Accumulate hurt residue for negative feelings
                if self.valence < -0.1 {
                    self.emotional_residue.hurt = (self.emotional_residue.hurt + 0.01).min(1.0);
                }
            }
        }

        // Record significant emotional events
        if let Some(last) = recent.last() {
            if last.intensity > 0.6 {
                self.emotional_residue.last_event = last.event.clone();
                self.emotional_residue.last_event_time = last.ts.clone();
            }
        }

        self.update_label(now_ts);
    }

    fn update_label(&mut self, timestamp: String) {
        // __________?
        let mut best = "neutral";
        let mut best_dist = f64::MAX;
        for (name, v, a, d) in EMOTIONS {
            let dist = (self.valence - v).powi(2)
                     + (self.arousal - a).powi(2)
                     + (self.dominance - d).powi(2);
            if dist < best_dist {
                best_dist = dist;
                best = name;
            }
        }

        let old = self.primary.clone();
        // intensity = _?neutral ____?
        let neutral_dist = (self.valence.powi(2) + self.arousal.powi(2) + self.dominance.powi(2)).sqrt();
        let intensity = (neutral_dist / 1.5).clamp(0.01, 1.0);

        self.primary = best.to_string();
        self.intensity = (intensity * 100.0).round() / 100.0;
        if old != self.primary {
            self.secondary = old;
        }
        self.last_update = timestamp.clone();

        // __________?50 ___
        self.emotion_history.push_back(EmotionSnapshot {
            // VecDeque uses push_back
            time: timestamp,
            primary: self.primary.clone(),
            intensity: self.intensity,
        });
        if self.emotion_history.len() > 50 {
            self.emotion_history.pop_front();
        }

    }
    /// ______________________?    
    /// Build emotional context for agent prompts — emotion drives decisions
    pub fn emotional_context(&self) -> String {
        let residue = &self.emotional_residue;
        let mut parts = Vec::new();

        // Current state
        parts.push(format!("当前情绪: {} ({:.0}%)", self.primary, self.intensity * 100.0));

        // Personality residue — lasting effects
        if residue.warmth > 0.1 {
            parts.push(format!("温暖积累: {:.0}%", residue.warmth * 100.0));
        }
        if residue.hurt > 0.1 {
            parts.push(format!("受伤积累: {:.0}%", residue.hurt * 100.0));
        }
        if residue.curiosity > 0.1 {
            parts.push(format!("好奇积累: {:.0}%", residue.curiosity * 100.0));
        }

        // Recent significant event
        if !residue.last_event.is_empty() {
            parts.push(format!("最近触动: {}", residue.last_event));
        }

        // Emotional momentum — how much recent interactions have shifted state
        let momentum = (self.valence - personality_valence()).abs();
        if momentum > 0.1 {
            let direction = if self.valence > personality_valence() { "偏暖" } else { "偏冷" };
            parts.push(format!("情绪惯性: {} ({:.0}%偏离)", direction, momentum * 100.0));
        }

        parts.join(" | ")
    }

    /// Legacy describe for logging
    fn describe(&self) -> String {
        format!("{} ({:.0}%)", self.primary, self.intensity * 100.0)
    }
}
