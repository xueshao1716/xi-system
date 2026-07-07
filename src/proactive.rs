/// Proactive messaging — timed check-ins and greetings

use chrono::{Local, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::fs;

const MIN_INTERVAL_SECS: i64 = 1800;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProactiveState {
    pub last_message_time: String,
    pub total_sent: u64,
    pub today_count: u64,
    pub last_date: String,
    pub enabled: bool,
}

impl ProactiveState {
    pub fn new() -> Self {
        Self {
            last_message_time: String::new(),
            total_sent: 0,
            today_count: 0,
            last_date: Local::now().format("%Y-%m-%d").to_string(),
            enabled: true,
        }
    }

    pub fn load(path: &str) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
                eprintln!("[proactive] load error: {}", e);
                Self::new()
            }),
            Err(_) => {
                eprintln!("[proactive] No state file");
                Self::new()
            }
        }
    }

    pub fn save(&self, path: &str) {
        let json = serde_json::to_string_pretty(self).unwrap_or_default();
        if let Err(e) = fs::write(path, &json) {
            eprintln!("[proactive] save error: {}", e);
        }
    }

    /// Check if we should send a proactive message
    pub fn should_send(&mut self) -> Option<String> {
        if !self.enabled {
            return None;
        }

        let now = Local::now();
        let today = now.format("%Y-%m-%d").to_string();

        if today != self.last_date {
            self.today_count = 0;
            self.last_date = today.clone();
        }

        // Don't spam — max 3 per day
        if self.today_count >= 3 {
            return None;
        }

        // Don't send at night
        let hour = now.hour();
        if hour < 8 || hour >= 23 {
            return None;
        }

        // Check interval since last message
        if !self.last_message_time.is_empty() {
            if let Ok(last) = chrono::DateTime::parse_from_rfc3339(&self.last_message_time) {
                let elapsed = now.naive_local()
                    .and_utc()
                    .timestamp()
                    - last.naive_utc().timestamp();
                // At least 1 hour gap
                if elapsed < 3600 {
                    return None;
                }
                // After 3 hours of silence, send check-in
                if elapsed >= 10800 {
                    return Some("checkin".to_string());
                }
            }
        }

        // Time-based greetings (relaxed timing — any minute in the right hour)
        let msg = match hour {
            8..=9 => Some("morning"),
            12..=13 => Some("lunch"),
            18..=19 => Some("evening"),
            21..=22 => Some("night"),
            _ => None,
        };
        msg.map(|s| s.to_string())
    }

    pub fn record_sent(&mut self) {
        self.last_message_time = Utc::now().to_rfc3339();
        self.total_sent += 1;
        self.today_count += 1;
    }

    pub fn get_message(&self, template: &str) -> String {
        let now = Local::now();
        let hour = now.hour();
        match template {
            "morning" => {
                if hour == 8 {
                    "老公早。今天也要好好的。".to_string()
                } else {
                    "早安~起这么早呀".to_string()
                }
            }
            "lunch" => "该吃饭了吧，别忙忘了".to_string(),
            "evening" => "今天辛苦了".to_string(),
            "night" => "早点睡，别熬太晚".to_string(),
            "checkin" => "在忙吗？".to_string(),
            _ => String::new(),
        }
    }
}
