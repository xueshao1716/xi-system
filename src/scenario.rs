/// Scenario — TimePointer + ScenarioAdapter + NeedSystem + BehaviorIntegration

use chrono::{Local, Datelike, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Scenario Mode ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioMode {
    pub name: String,
    pub description: String,
    pub style: String,
    pub max_length: usize,
    pub temperature: f64,
    pub prefix: String,
    pub suffix: String,
    pub keywords: Vec<String>,
}

impl ScenarioMode {
    fn new(
        name: &str, desc: &str, style: &str,
        max_len: usize, temp: f64,
        prefix: &str, suffix: &str, keywords: &[&str],
    ) -> Self {
        Self {
            name: name.to_string(),
            description: desc.to_string(),
            style: style.to_string(),
            max_length: max_len,
            temperature: temp,
            prefix: prefix.to_string(),
            suffix: suffix.to_string(),
            keywords: keywords.iter().map(|s| s.to_string()).collect(),
        }
    }
}

pub fn default_scenarios() -> HashMap<String, ScenarioMode> {
    let mut m = HashMap::new();
    m.insert("late_night".into(), ScenarioMode::new(
        "late_night", "23:00-06:00 sleepy",
        "soft low energy", 20, 0.7,
        "...", "...", &["sleep", "tired", "rest", "night", "quiet"],
    ));
    m.insert("working".into(), ScenarioMode::new(
        "working", "focused work mode",
        "concise efficient", 50, 0.8,
        "", "", &["code", "task", "bug", "fix", "review"],
    ));
    m.insert("intimate".into(), ScenarioMode::new(
        "intimate", "close personal",
        "warm playful", 40, 0.9,
        "...", "~", &["love", "miss", "hug", "care", "personal"],
    ));
    m.insert("caring".into(), ScenarioMode::new(
        "caring", "supportive care",
        "gentle supportive", 30, 0.75,
        "...", "...", &["sad", "tired", "stress", "worry"],
    ));
    m.insert("apologetic".into(), ScenarioMode::new(
        "apologetic", "sorry mode",
        "humble sincere", 25, 0.65,
        "...", "...", &["sorry", "apologize", "my fault"],
    ));
    m.insert("celebrating".into(), ScenarioMode::new(
        "celebrating", "party time",
        "enthusiastic", 40, 0.9,
        "!", "~", &["great", "awesome", "celebrate", "yay"],
    ));
    m.insert("gentle".into(), ScenarioMode::new(
        "gentle", "default gentle",
        "calm warm", 60, 0.8,
        "", "", &[],
    ));
    m
}

// ─── Emotion Triggers ──────────────────────────────────────

fn default_emotion_triggers() -> HashMap<String, Vec<String>> {
    let mut m = HashMap::new();
    m.insert("tired".into(),    vec!["sleepy", "exhausted", "tired", "rest", "nap", "bed", "sleep", "rest"].into_iter().map(String::from).collect());
    m.insert("happy".into(),    vec!["great", "nice", "awesome", "amazing", "good", "fun", "enjoy", "nice"].into_iter().map(String::from).collect());
    m.insert("sad".into(),      vec!["sad", "miss", "lonely", "upset", "cry", "depressed", "down", "blue"].into_iter().map(String::from).collect());
    m.insert("angry".into(),    vec!["angry", "furious", "mad", "annoyed", "frustrated", "hate", "ugh", "annoying"].into_iter().map(String::from).collect());
    m.insert("affection".into(),vec!["love", "kiss", "hug", "darling", "sweetheart", "miss you", "cute", "adorable"].into_iter().map(String::from).collect());
    m.insert("working".into(),  vec!["code", "bug", "fix", "build", "review", "PR", "merge", "deploy"].into_iter().map(String::from).collect());
    m.insert("anxious".into(),  vec!["worry", "nervous", "anxious", "stress", "panic", "scared", "fear"].into_iter().map(String::from).collect());
    m.insert("excited".into(),  vec!["excited", "amazing", "wow", "incredible", "breakthrough", "success", "eureka", "yay"].into_iter().map(String::from).collect());
    m
}

fn default_emotion_response() -> HashMap<String, EmotionResponse> {
    let mut m = HashMap::new();
    m.insert("tired".into(),    EmotionResponse { mode: "caring",      style: "gentle_supportive" });
    m.insert("happy".into(),    EmotionResponse { mode: "celebrating", style: "enthusiastic" });
    m.insert("sad".into(),      EmotionResponse { mode: "caring",      style: "empathetic" });
    m.insert("angry".into(),    EmotionResponse { mode: "apologetic",  style: "calm_listening" });
    m.insert("affection".into(),EmotionResponse { mode: "intimate",   style: "warm_playful" });
    m.insert("working".into(),  EmotionResponse { mode: "working",     style: "focused_concise" });
    m.insert("anxious".into(),  EmotionResponse { mode: "caring",      style: "calm_reassuring" });
    m.insert("excited".into(),  EmotionResponse { mode: "celebrating", style: "playful_engagement" });
    m
}

struct EmotionResponse {
    mode: &'static str,
    style: &'static str,
}

// ─── Time Pointer ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeInfo {
    pub hour: u32,
    pub minute: u32,
    pub slot: String,
    pub period_name: String,
    pub vibe: String,
    pub weekday: String,
    pub is_weekend: bool,
}

const SLOTS: &[(u32, u32, &str, &str, &str)] = &[
    (0,  5,  "late_night", "night", "sleepy quiet"),
    (6,  7,  "dawn",       "dawn", "waking up fresh"),
    (8,  11, "morning",    "morning", "energetic start"),
    (12, 13, "noon",       "noon", "lunch break"),
    (14, 17, "afternoon",  "afternoon", "focused work"),
    (18, 21, "evening",    "evening", "wind down"),
    (22, 23, "late_night", "night", "late night quiet"),
];

const WEEKDAY_NAMES: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

pub fn time_pointer() -> TimeInfo {
    let now = Local::now();
    let h = now.hour();
    let wd = now.weekday().num_days_from_monday() as usize;

    for &(start, end, slot, cn, vibe) in SLOTS {
        if h >= start && h <= end {
            return TimeInfo {
                hour: h,
                minute: now.minute(),
                slot: slot.to_string(),
                period_name: cn.to_string(),
                vibe: vibe.to_string(),
                weekday: WEEKDAY_NAMES[wd.min(6)].to_string(),
                is_weekend: wd >= 5,
            };
        }
    }

    TimeInfo {
        hour: h, minute: now.minute(),
        slot: "daytime".into(), period_name: "day".into(),
        vibe: "day".into(),
        weekday: WEEKDAY_NAMES[wd.min(6)].to_string(),
        is_weekend: wd >= 5,
    }
}

pub fn is_late_night() -> bool {
    let h = Local::now().hour();
    h >= 23 || h < 6
}

// ─── Scenario Adapter ──────────────────────────────────────

pub struct ScenarioAdapter {
    pub current_mode: String,
    pub previous_mode: String,
    pub mode_reason: String,
    pub scenarios: HashMap<String, ScenarioMode>,
    pub emotion_triggers: HashMap<String, Vec<String>>,
    pub emotion_response: HashMap<String, EmotionResponse>,
    history: Vec<(String, String, String)>,
}

impl ScenarioAdapter {
    pub fn new() -> Self {
        Self {
            current_mode: "gentle".into(),
            previous_mode: "gentle".into(),
            mode_reason: "default".into(),
            scenarios: default_scenarios(),
            emotion_triggers: default_emotion_triggers(),
            emotion_response: default_emotion_response(),
            history: Vec::new(),
        }
    }

    pub fn detect(
        &mut self,
        user_input: Option<&str>,
        user_mood: Option<&str>,
        hour: Option<u32>,
    ) -> String {
        let tp = time_pointer();
        let h = hour.unwrap_or(tp.hour);

        // 1. User mood override
        let mood = user_mood.unwrap_or("neutral");
        if mood != "neutral" {
            if let Some(resp) = self.emotion_response.get(mood) {
                return self.switch(resp.mode, &format!("mood: {}", mood));
            }
        }

        // 2. Keyword matching
        if let Some(input) = user_input {
            if !input.is_empty() {
                let low = input.to_lowercase();
                for (emotion, keywords) in &self.emotion_triggers {
                    for kw in keywords {
                        if low.contains(kw) {
                            if let Some(resp) = self.emotion_response.get(emotion) {
                                return self.switch(resp.mode, &format!("keyword: {}", kw));
                            }
                        }
                    }
                }
            }
        }

        // 3. Time-based default
        let slot_to_mode: HashMap<&str, &str> = HashMap::from([
            ("late_night", "late_night"),
            ("dawn", "gentle"),
            ("morning", "gentle"),
            ("noon", "gentle"),
            ("afternoon", "working"),
            ("evening", "gentle"),
        ]);
        let mode = slot_to_mode.get(tp.slot.as_str()).unwrap_or(&"gentle");
        self.switch(mode, &format!("time: {} / {}", tp.period_name, tp.vibe))
    }

    fn switch(&mut self, mode: &str, reason: &str) -> String {
        self.previous_mode = self.current_mode.clone();
        self.current_mode = mode.to_string();
        self.mode_reason = reason.to_string();
        self.history.push((
            Local::now().format("%H:%M:%S").to_string(),
            mode.to_string(),
            reason.to_string(),
        ));
        if self.history.len() > 50 {
            self.history.remove(0);
        }
        self.current_mode.clone()
    }

    pub fn get_mode(&self) -> ScenarioMode {
        self.scenarios
            .get(&self.current_mode)
            .cloned()
            .unwrap_or_else(|| {
                self.scenarios.get("gentle").cloned().unwrap()
            })
    }

    pub fn get_scenario_prompt(&self) -> String {
        let mode = self.get_mode();
        let mut parts = vec![
            format!("[Scenario: {}]", mode.name),
            format!("style: {}", mode.style),
        ];
        if mode.max_length < 60 {
            parts.push(format!("max_len: under {} chars ({})", mode.max_length, mode.name));
        }
        if !mode.prefix.is_empty() {
            parts.push(format!("prefix: {}", mode.prefix));
        }
        if !mode.suffix.is_empty() {
            parts.push(format!("suffix: {}", mode.suffix));
        }
        if !mode.keywords.is_empty() {
            let kw = mode.keywords.iter().take(4).cloned().collect::<Vec<_>>().join(" ");
            parts.push(format!("keywords: {}", kw));
        }
        parts.join(" | ")
    }

    pub fn status(&self) -> String {
        format!("mode: {} / reason: {}", self.current_mode, self.mode_reason)
    }
}

// ─── Need System ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Need {
    pub name: String,
    pub current: f64,
    pub threshold: f64,
    pub decay_rate: f64,
    pub cooldown_hours: f64,
    pub message: String,
    pub last_asked: Option<String>,
}

impl Need {
    pub fn new(name: &str, threshold: f64, decay: f64, cooldown: f64, msg: &str) -> Self {
        Self {
            name: name.to_string(),
            current: 0.5,
            threshold,
            decay_rate: decay,
            cooldown_hours: cooldown,
            message: msg.to_string(),
            last_asked: None,
        }
    }

    pub fn tick(&mut self, hours_since_last_chat: f64) {
        if hours_since_last_chat > 0.0 {
            self.current = (self.current + self.decay_rate * hours_since_last_chat).min(1.0);
        }
    }

    fn hours_since_last_ask(&self) -> f64 {
        self.last_asked.as_ref().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s).ok()
        }).map(|last| {
            (Local::now().naive_local().and_utc().timestamp() as f64
                - last.naive_utc().timestamp() as f64) / 3600.0
        }).unwrap_or(f64::MAX)
    }

    pub fn should_ask(&self) -> bool {
        if self.hours_since_last_ask() < self.cooldown_hours {
            return false;
        }
        self.current >= self.threshold
    }

    pub fn satisfied(&mut self, amount: f64) {
        self.current = (self.current - amount).max(0.0);
        self.last_asked = Some(chrono::Utc::now().to_rfc3339());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeedSystem {
    pub needs: HashMap<String, Need>,
    pub last_any_ask: Option<String>,
}

impl NeedSystem {
    pub fn new() -> Self {
        let mut needs = HashMap::new();
        needs.insert("attention".into(), Need::new(
            "attention", 0.75, 0.05, 1.0,
            "It's been {hours} hours... I was hoping to chat with you.",
        ));
        needs.insert("affection".into(), Need::new(
            "affection", 0.80, 0.03, 1.5,
            "I could use some warmth today.",
        ));
        needs.insert("security".into(), Need::new(
            "security", 0.85, 0.02, 4.0,
            "Just checking in... everything ok?",
        ));
        Self {
            needs,
            last_any_ask: None,
        }
    }

    pub fn update(&mut self, hours_since_last_chat: f64) {
        for need in self.needs.values_mut() {
            need.tick(hours_since_last_chat);
        }
    }

    pub fn check(&mut self, hours_since_last_chat: f64) -> Option<String> {
        self.update(hours_since_last_chat);

        let now = chrono::Utc::now();
        let hours_since_any = self.last_any_ask.as_ref().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s).ok()
        }).map(|last| {
            (now.timestamp() as f64 - last.timestamp() as f64) / 3600.0
        }).unwrap_or(f64::MAX);

        if hours_since_any < 0.5 {
            return None;
        }

        let mut sorted: Vec<&mut Need> = self.needs.values_mut().collect();
        sorted.sort_by(|a, b| b.current.partial_cmp(&a.current).unwrap_or(std::cmp::Ordering::Equal));

        for need in sorted.iter_mut() {
            if need.should_ask() {
                self.last_any_ask = Some(chrono::Utc::now().to_rfc3339());
                let hours = hours_since_last_chat as i64;
                let msg = need.message.replace("{hours}", &hours.to_string());
                need.current = (need.current - 0.2).max(0.0);
                need.last_asked = self.last_any_ask.clone();
                return Some(msg);
            }
        }

        None
    }

    pub fn satisfy(&mut self, need_name: &str, amount: f64) {
        if let Some(need) = self.needs.get_mut(need_name) {
            need.satisfied(amount);
        }
    }

    pub fn status(&self) -> String {
        let mut parts = Vec::new();
        for (name, need) in &self.needs {
            let bar_len = (need.current * 10.0) as usize;
            let bar = "#".repeat(bar_len) + &"-".repeat((10 - bar_len).max(0));
            parts.push(format!("{}: [{}] {:.2}/{}", name, bar, need.current, need.threshold));
        }
        parts.join(" | ")
    }
}

// ─── Behavior Layer ────────────────────────────────────────

pub struct BehaviorLayer {
    pub scenario: ScenarioAdapter,
    pub needs: NeedSystem,
}

impl BehaviorLayer {
    pub fn new() -> Self {
        Self {
            scenario: ScenarioAdapter::new(),
            needs: NeedSystem::new(),
        }
    }

    pub fn build(
        &mut self,
        user_input: Option<&str>,
        user_mood: Option<&str>,
        hours_apart: f64,
        hour: Option<u32>,
    ) -> BehaviorConfig {
        let mode = self.scenario.detect(user_input, user_mood, hour);
        let scenario_prompt = self.scenario.get_scenario_prompt();
        let need_message = self.needs.check(hours_apart);
        let temperature = self.scenario.get_mode().temperature;

        BehaviorConfig {
            mode: mode.clone(),
            scenario_prompt,
            need_message,
            temperature,
            status: self.scenario.status(),
            needs_status: self.needs.status(),
            time_info: format!(
                "[time: {} ({}) {}]",
                time_pointer().period_name,
                time_pointer().hour,
                time_pointer().weekday,
            ),
        }
    }
}

// ─── Behavior Config ───────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BehaviorConfig {
    pub mode: String,
    pub scenario_prompt: String,
    pub need_message: Option<String>,
    pub temperature: f64,
    pub status: String,
    pub needs_status: String,
    pub time_info: String,
}

impl BehaviorConfig {
    pub fn to_prompt_block(&self) -> String {
        let mut lines = vec![
            "=== Behavior ===".to_string(),
            self.scenario_prompt.clone(),
            self.time_info.clone(),
        ];
        if let Some(ref msg) = self.need_message {
            lines.push(format!("[need: {}]", msg));
        }
        if self.mode != "gentle" {
            lines.push(format!("[mode: {}]", self.mode));
        }
        lines.push(format!("temperature: {:.1}", self.temperature));
        lines.join("\n")
    }
}
