//! Matrix bridge for XI system
//!
//! Account: @xinyu-xi:myxinyu.xin
//! Password: from config.json (never hardcode)
//! Element: element.myxinyu.xin

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const TOKEN_PATH: &str = "/mnt/d/xi-system/matrix_token.json";

/// Matrix client for XI system
pub struct MatrixClient {
    pub homeserver: String,
    pub user_id: String,
    password: String,
    access_token: Option<String>,
    next_batch: Option<String>,
    pub rooms: Vec<String>,
    pub last_sync: chrono::DateTime<chrono::Utc>,
    pub ready: bool,
    pub pending_messages: Vec<MatrixMessage>,
}

/// A message received from Matrix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixMessage {
    pub room_id: String,
    pub sender: String,
    pub body: String,
    pub event_id: String,
    pub timestamp: u64,
}

// Matrix API response types

#[derive(Deserialize)]
struct LoginResponse {
    access_token: String,
    user_id: String,
    device_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct TokenCache {
    access_token: String,
    user_id: String,
    saved_at: String,
}

#[derive(Deserialize)]
struct SyncResponse {
    next_batch: Option<String>,
    rooms: Option<SyncRooms>,
}

#[derive(Deserialize)]
struct SyncRooms {
    #[serde(rename = "join")]
    join: Option<HashMap<String, JoinedRoom>>,
}

#[derive(Deserialize)]
struct JoinedRoom {
    #[serde(rename = "timeline")]
    timeline: Option<Timeline>,
}

#[derive(Deserialize)]
struct Timeline {
    events: Option<Vec<RoomEvent>>,
    #[serde(rename = "prev_batch")]
    prev_batch: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
struct RoomEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(rename = "event_id")]
    event_id: Option<String>,
    #[serde(rename = "sender")]
    sender: Option<String>,
    #[serde(rename = "origin_server_ts")]
    origin_server_ts: Option<u64>,
    #[serde(rename = "content")]
    content: Option<EventContent>,
    #[serde(rename = "unsigned")]
    unsigned: Option<UnsignedData>,
}

#[derive(Deserialize, Clone, Debug)]
struct EventContent {
    #[serde(rename = "body")]
    body: Option<String>,
    #[serde(rename = "msgtype")]
    msgtype: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
struct UnsignedData {
    age: Option<u64>,
}

#[derive(Serialize)]
struct SendBody {
    msgtype: String,
    body: String,
}

impl MatrixClient {
    /// Create new Matrix client
    pub fn new(user_id: &str, password: &str, homeserver: &str) -> Self {
        MatrixClient {
            homeserver: homeserver.to_string(),
            user_id: user_id.to_string(),
            password: password.to_string(),
            access_token: None,
            next_batch: None,
            rooms: Vec::new(),
            last_sync: chrono::Utc::now(),
            ready: false,
            pending_messages: Vec::new(),
        }
    }

    /// Login to Matrix, using cached token if available
    pub async fn login(&mut self) -> Result<(), String> {
        if let Some(cached) = Self::load_token() {
            self.access_token = Some(cached.access_token);
            self.ready = true;
            self.last_sync = chrono::Utc::now();
            println!("Matrix: logged in with cached token ({})", self.user_id);
            return Ok(());
        }

        println!("Matrix: logging in...");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .no_proxy()
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;

        let body = serde_json::json!({
            "type": "m.login.password",
            "identifier": {
                "type": "m.id.user",
                "user": self.user_id
            },
            "password": self.password,
            "initial_device_display_name": "XI Matrix Bot"
        });

        let url = format!("{}/_matrix/client/v3/login", self.homeserver);
        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Login request failed: {}", e))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if status.is_success() {
            let login: LoginResponse = serde_json::from_str(&text)
                .map_err(|e| format!("Login parse error: {}", e))?;
            self.access_token = Some(login.access_token.clone());
            self.ready = true;
            self.last_sync = chrono::Utc::now();
            Self::save_token(&login.access_token, &self.user_id);
            println!("Matrix: login OK ({})", self.user_id);
            Ok(())
        } else {
            Err(format!("Login failed (HTTP {}): {}", status, text.chars().take(200).collect::<String>()))
        }
    }

    fn load_token() -> Option<TokenCache> {
        std::fs::read_to_string(TOKEN_PATH).ok()
            .and_then(|s| serde_json::from_str::<TokenCache>(&s).ok())
    }

    fn save_token(token: &str, user_id: &str) {
        let cache = TokenCache {
            access_token: token.to_string(),
            user_id: user_id.to_string(),
            saved_at: chrono::Utc::now().to_rfc3339(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&cache) {
            let _ = std::fs::write(TOKEN_PATH, &json);
        }
    }

    fn authed_get(&self, url: &str) -> Result<reqwest::RequestBuilder, String> {
        let token = self.access_token.as_ref().ok_or("no access_token")?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .no_proxy()
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;
        Ok(client
            .get(url)
            .header("Authorization", format!("Bearer {}", token)))
    }

    fn authed_put(&self, url: &str) -> Result<reqwest::RequestBuilder, String> {
        let token = self.access_token.as_ref().ok_or("no access_token")?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .no_proxy()
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;
        Ok(client
            .put(url)
            .header("Authorization", format!("Bearer {}", token)))
    }

    /// Sync with Matrix server
    pub async fn sync(&mut self) -> Result<Vec<MatrixMessage>, String> {
        if !self.ready {
            return Err("not ready".to_string());
        }

        let mut url = format!(
            "{}/_matrix/client/v3/sync?timeout=10000",
            self.homeserver,
        );
        if let Some(ref batch) = self.next_batch {
            url.push_str(&format!("&since={}", batch));
        }

        let resp = self.authed_get(&url)?;
        let resp = resp.send().await
            .map_err(|e| format!("Sync request failed: {}", e))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            if status.as_u16() == 401 {
                self.ready = false;
                self.access_token = None;
                let _ = std::fs::remove_file(TOKEN_PATH);
                return Err("token expired, re-login needed".to_string());
            }
            return Err(format!("Sync failed (HTTP {}): {}", status, text.chars().take(100).collect::<String>()));
        }

        let sync: SyncResponse = serde_json::from_str(&text)
            .map_err(|e| format!("Sync parse error: {}", e))?;

        if let Some(ref batch) = sync.next_batch {
            self.next_batch = Some(batch.clone());
        }
        self.last_sync = chrono::Utc::now();

        let mut messages = Vec::new();
        if let Some(rooms) = sync.rooms {
            if let Some(join) = rooms.join {
                for (room_id, room) in &join {
                    if !self.rooms.contains(room_id) {
                        self.rooms.push(room_id.clone());
                    }
                    if let Some(timeline) = &room.timeline {
                        if let Some(events) = &timeline.events {
                            for event in events {
                                if event.event_type != "m.room.message" {
                                    continue;
                                }
                                let sender = event.sender.clone().unwrap_or_default();
                                let body = event.content.as_ref()
                                    .and_then(|c| c.body.clone())
                                    .unwrap_or_default();
                                let event_id = event.event_id.clone().unwrap_or_default();
                                let ts = event.origin_server_ts.unwrap_or(0);
                                if sender == self.user_id || body.trim().is_empty() {
                                    continue;
                                }
                                messages.push(MatrixMessage {
                                    room_id: room_id.clone(),
                                    sender,
                                    body,
                                    event_id,
                                    timestamp: ts,
                                });
                            }
                        }
                    }
                }
            }
        }

        self.pending_messages.extend(messages.clone());
        Ok(messages)
    }

    /// Send text message to a room
    pub async fn send_text(&self, room_id: &str, text: &str) -> Result<(), String> {
        if !self.ready {
            return Err("not ready".to_string());
        }

        let txn_id = format!("xi-{}", chrono::Utc::now().timestamp_millis());
        let url = format!(
            "{}/_matrix/client/v3/rooms/{}/send/m.room.message/{}",
            self.homeserver,
            url_encode(room_id),
            txn_id,
        );

        let body = SendBody {
            msgtype: "m.text".to_string(),
            body: text.to_string(),
        };

        let resp = self.authed_put(&url)?;
        let resp = resp.json(&body).send().await
            .map_err(|e| format!("Send failed: {}", e))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Send failed (HTTP {}): {}", status, text.chars().take(100).collect::<String>()));
        }
        Ok(())
    }

    /// Pop next pending message
    pub fn pop_message(&mut self) -> Option<MatrixMessage> {
        if self.pending_messages.is_empty() {
            None
        } else {
            Some(self.pending_messages.remove(0))
        }
    }

    /// Invalidate session (clear token)
    pub fn invalidate(&mut self) {
        self.ready = false;
        self.access_token = None;
    }
}

/// URL encode a Matrix room_id
fn url_encode(s: &str) -> String {
    s.replace('#', "%23")
     .replace(':', "%3A")
     .replace('/', "%2F")
     .replace('+', "%2B")
     .replace(' ', "%20")
}
