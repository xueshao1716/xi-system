/// WeiLink bot API client (reqwest + rustls-tls)

use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

const BASE_URL: &str = "https://ilinkai.weixin.qq.com";
const BOT_TYPE: &str = "3";
const CHANNEL_VERSION: &str = "2.0.95";
const LONGPOLL_TIMEOUT: u64 = 35;

// ── Response / data types ──────────────────────────────────────

#[derive(Debug, Clone)]
pub struct QrResponse {
    pub qrcode: Option<String>,
    pub qrcode_img_content: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QrStatusResponse {
    pub status: Option<i32>,
    pub bot_token: Option<String>,
    pub baseurl: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GetUpdatesResult {
    pub msgs: Option<Vec<WechatMessage>>,
    pub get_updates_buf: Option<String>,
}

/// Backward-compatible alias
pub type UpdatesResponse = GetUpdatesResult;

#[derive(Debug, Clone)]
pub struct WechatMessage {
    pub from_user_id: Option<String>,
    pub context_token: Option<String>,
    pub item_list: Option<Vec<MessageItem>>,
}

/// Backward-compatible alias
pub type Message = WechatMessage;

#[derive(Debug, Clone)]
pub struct MessageItem {
    pub text_item: Option<TextItem>,
}

#[derive(Debug, Clone)]
pub struct TextItem {
    pub text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SendResponse {
    pub ret: Option<i32>,
    pub errcode: Option<i32>,
}

// ── Helper: parse messages from JSON ───────────────────────────

fn parse_messages(msgs: &Value) -> Vec<WechatMessage> {
    let arr = match msgs.as_array() {
        Some(a) => a,
        None => return Vec::new(),
    };
    arr.iter()
        .map(|m| WechatMessage {
            from_user_id: m["from_user_id"].as_str().map(|s| s.to_string()),
            context_token: m["context_token"].as_str().map(|s| s.to_string()),
            item_list: m["item_list"].as_array().map(|items| {
                items
                    .iter()
                    .map(|item| MessageItem {
                        text_item: item["text_item"]["text"].as_str().map(|_| TextItem {
                            text: item["text_item"]["text"]
                                .as_str()
                                .map(|s| s.to_string()),
                        }),
                    })
                    .collect()
            }),
        })
        .collect()
}

// ── HTTP helpers ───────────────────────────────────────────────

fn http_client(timeout_secs: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(timeout_secs + 10))
        .connect_timeout(std::time::Duration::from_secs(10))
        .pool_max_idle_per_host(1)
        .user_agent("Mozilla/5.0")
        .build()
        .expect("HTTP client build failed")
}

async fn http_post(
    client: &reqwest::Client,
    url: &str,
    headers: &[(String, String)],
    body: &Value,
) -> Result<(u16, Value), String> {
    let mut req = client.post(url);
    for (k, v) in headers {
        req = req.header(k.as_str(), v.as_str());
    }
    req = req.json(body);
    let resp = req
        .send()
        .await
        .map_err(|e| format!("HTTP POST error: {}", e))?;
    let status = resp.status().as_u16();
    let body: Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))?;
    // rate-limit / token-expired check
    if body["errcode"].as_i64() == Some(-14) {
        return Err("token expired or rate-limited".to_string());
    }
    Ok((status, body))
}

async fn http_get(
    client: &reqwest::Client,
    url: &str,
    headers: &[(String, String)],
) -> Result<(u16, Value), String> {
    let mut req = client.get(url);
    for (k, v) in headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("HTTP GET error: {}", e))?;
    let status = resp.status().as_u16();
    let body: Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))?;
    Ok((status, body))
}

// ── WeiLink client ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct WeiLink {
    pub bot_token: Option<String>,
    pub base_url: String,
    pub client: reqwest::Client,
}

impl WeiLink {
    pub fn new() -> Self {
        Self {
            bot_token: None,
            base_url: BASE_URL.to_string(),
            client: http_client(LONGPOLL_TIMEOUT + 10),
        }
    }

    /// Load saved token from JSON file
    pub fn load_token(&mut self, path: &str) -> bool {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return false,
        };
        let data: Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => return false,
        };
        if let Some(token) = data["bot_token"].as_str() {
            self.bot_token = Some(token.to_string());
            if let Some(url) = data["base_url"].as_str() {
                self.base_url = url.trim_end_matches('/').to_string();
            }
            true
        } else {
            false
        }
    }

    /// Save current token to JSON file
    pub fn save_token(&mut self, path: &str) {
        let data = json!({
            "bot_token": self.bot_token.as_deref().unwrap_or(""),
            "base_url": self.base_url,
        });
        let _ = std::fs::write(path, serde_json::to_string_pretty(&data).unwrap_or_default());
    }

    /// Quick token validity check (returns Ok(true) if token is usable)
    pub async fn check_token(&self) -> Result<bool, String> {
        let token = match &self.bot_token {
            Some(t) if !t.is_empty() => t.clone(),
            _ => return Ok(false),
        };
        let url = format!("{}/ilink/bot/getbotinfo", self.base_url);
        let headers = common_headers_with_auth(self, &token);
        match http_get(&self.client, &url, &headers).await {
            Ok((_status, body)) => {
                // If we get a valid response without -14 error, token is OK
                if body["errcode"].as_i64() == Some(-14) {
                    Ok(false)
                } else {
                    Ok(true)
                }
            }
            Err(_) => Ok(false),
        }
    }

    /// Set token and base URL directly
    pub fn set_token(&mut self, token: &str, base_url: &str) {
        self.bot_token = Some(token.to_string());
        if !base_url.is_empty() {
            self.base_url = base_url.trim_end_matches('/').to_string();
        }
        self.client = http_client(LONGPOLL_TIMEOUT + 10);
    }

    /// Get QR code for login
    pub async fn get_qr_code(&self) -> Result<QrResponse, String> {
        let url = format!("{}/ilink/bot/getqrcode", self.base_url);
        let body = json!({
            "bot_type": BOT_TYPE,
            "base_info": { "channel_version": CHANNEL_VERSION }
        });
        let headers = common_headers(&self);
        let (_, result) = http_post(&self.client, &url, &headers, &body).await?;

        Ok(QrResponse {
            qrcode: result["qrcode"].as_str().map(|s| s.to_string()),
            qrcode_img_content: result["qrcode_img_content"].as_str().map(|s| s.to_string()),
        })
    }

    /// Poll QR code scan status (single attempt)
    pub async fn poll_qr_status(&self, qr_token: &str) -> Result<Option<QrStatusResponse>, String> {
        let url = format!(
            "{}/ilink/bot/getqrcode_status?qrcode={}",
            self.base_url, qr_token
        );
        let headers = common_headers(&self);
        let resp = self
            .client
            .get(&url)
            .headers(reqwest::header::HeaderMap::from_iter(
                headers.iter().map(|(k, v)| {
                    (
                        reqwest::header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                        reqwest::header::HeaderValue::from_str(v).unwrap(),
                    )
                }),
            ))
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        let status = resp.status().as_u16();
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        let bot_token = body["bot_token"].as_str().map(|s| s.to_string());
        let baseurl = body["baseurl"].as_str().map(|s| s.to_string());

        if let Some(_token) = &bot_token {
            Ok(Some(QrStatusResponse {
                status: body["status"].as_i64().map(|v| v as i32),
                bot_token,
                baseurl,
            }))
        } else {
            Ok(None)
        }
    }

    /// Long-poll QR code scan status (retries until token received or timeout)
    pub async fn poll_qr_status_long(
        &self,
        qr_token: &str,
    ) -> Result<Option<QrStatusResponse>, String> {
        let deadline =
            SystemTime::now() + std::time::Duration::from_secs(LONGPOLL_TIMEOUT);
        let mut attempts = 0u32;
        loop {
            attempts += 1;
            let result = self.poll_qr_status(qr_token).await;
            match result {
                Ok(Some(status_resp)) => return Ok(Some(status_resp)),
                Ok(None) => {}
                Err(e) => {
                    if attempts >= 5 {
                        return Err(e);
                    }
                }
            }
            if SystemTime::now() >= deadline {
                return Ok(None);
            }
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    }

    /// Fetch new messages (long-poll / get updates)
    pub async fn get_updates(&self, cursor: &str) -> Result<GetUpdatesResult, String> {
        let token = self.bot_token.as_deref().ok_or("no bot_token set")?;
        let url = format!("{}/ilink/bot/getupdates", self.base_url);

        let body = json!({
            "get_updates_buf": cursor,
            "base_info": { "channel_version": CHANNEL_VERSION }
        });

        let headers = common_headers_with_auth(self, token);
        let (_, result) = http_post(&self.client, &url, &headers, &body).await?;

        Ok(GetUpdatesResult {
            msgs: Some(parse_messages(&result["msgs"])),
            get_updates_buf: result["get_updates_buf"]
                .as_str()
                .map(|s| s.to_string()),
        })
    }

    /// Send a text message
    pub async fn send_text(
        &self,
        to_user: &str,
        text: &str,
        context_token: &str,
    ) -> Result<SendResponse, String> {
        let token = self.bot_token.as_deref().ok_or("no bot_token set")?;
        let url = format!("{}/ilink/bot/sendmessage", self.base_url);

        let client_id = format!(
            "xi:{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );

        let body = json!({
            "msg": {
                "from_user_id": "",
                "to_user_id": to_user,
                "client_id": client_id,
                "message_type": 2,
                "message_state": 2,
                "context_token": context_token,
                "item_list": [{"type": 1, "text_item": {"text": text}}],
            },
            "base_info": { "channel_version": CHANNEL_VERSION }
        });

        let headers = common_headers_with_auth(self, token);
        let (_, result) = http_post(&self.client, &url, &headers, &body).await?;

        Ok(SendResponse {
            ret: result["ret"].as_i64().map(|v| v as i32),
            errcode: result["errcode"].as_i64().map(|v| v as i32),
        })
    }

    /// Get user info by user_id
    pub async fn get_user_info(&self, user_id: &str) -> Result<Value, String> {
        let token = self.bot_token.as_deref().ok_or("no bot_token set")?;
        let url = format!("{}/ilink/bot/getuserinfo", self.base_url);

        let body = json!({
            "user_id": user_id,
            "base_info": { "channel_version": CHANNEL_VERSION }
        });

        let headers = common_headers_with_auth(self, token);
        let (_, result) = http_post(&self.client, &url, &headers, &body).await?;
        Ok(result)
    }

    /// Get upload URL for media file
    pub async fn get_upload_url(
        &self,
        to_user_id: &str,
        media_type: i32,
        filekey: &str,
        rawsize: i32,
        rawfilemd5: &str,
        filesize: i32,
        aeskey_hex: &str,
    ) -> Result<Value, String> {
        let token = self.bot_token.as_deref().ok_or("no bot_token set")?;
        let url = format!("{}/ilink/bot/getuploadurl", self.base_url);

        let body = json!({
            "filekey": filekey,
            "media_type": media_type,
            "to_user_id": to_user_id,
            "rawsize": rawsize,
            "rawfilemd5": rawfilemd5,
            "filesize": filesize,
            "aeskey": aeskey_hex,
            "no_need_thumb": true,
        });

        let headers = common_headers_with_auth(self, token);
        let (_, result) = http_post(&self.client, &url, &headers, &body).await?;
        Ok(result)
    }

    /// Upload ciphertext to CDN
    pub async fn upload_ciphertext(
        &self,
        ciphertext: &[u8],
        upload_url: &str,
    ) -> Result<String, String> {
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;

        let resp = client
            .post(upload_url)
            .header("Content-Type", "application/octet-stream")
            .body(ciphertext.to_vec())
            .send()
            .await
            .map_err(|e| format!("CDN upload error: {}", e))?;

        let status = resp.status().as_u16();
        if status != 200 {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("CDN upload HTTP {}: {}", status, text));
        }

        let encrypted_param = resp
            .headers()
            .get("x-encrypted-param")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .ok_or("CDN missing x-encrypted-param header")?;
        Ok(encrypted_param)
    }

    /// Send a file (image / video / voice / generic file)
    pub async fn send_file(
        &self,
        to_user: &str,
        file_path: &str,
        caption: &str,
        context_token: &str,
    ) -> Result<SendResponse, String> {
        let _token = self.bot_token.as_deref().ok_or("no bot_token set")?;

        // Read file content
        let plaintext = std::fs::read(file_path)
            .map_err(|e| format!("read file error: {}", e))?;
        let filename = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");

        // Classify media type from extension
        let (media_type, _item_type) = classify_media(file_path);

        // Generate random filekey and AES key
        let filekey = hex::encode(rand::random::<[u8; 16]>());
        let aes_key = rand::random::<[u8; 16]>();
        let rawsize = plaintext.len() as i32;
        let rawfilemd5 = md5_hex(&plaintext);
        let filesize = aes_padded_size(rawsize as usize) as i32;

        // Get upload URL
        let upload_resp = self
            .get_upload_url(
                to_user,
                media_type,
                &filekey,
                rawsize,
                &rawfilemd5,
                filesize,
                &hex::encode(aes_key),
            )
            .await?;

        let upload_param = upload_resp["upload_param"]
            .as_str()
            .map(|s| s.to_string());
        let upload_full_url = upload_resp["upload_full_url"]
            .as_str()
            .map(|s| s.to_string());

        // Build upload URL
        let upload_url = if let Some(full_url) = upload_full_url {
            full_url
        } else if let Some(param) = upload_param {
            let cdn_base = "https://novac2c.cdn.weixin.qq.com/c2c";
            format!(
                "{}/upload?encrypted_query_param={}&filekey={}",
                cdn_base, param, filekey
            )
        } else {
            return Err(format!(
                "getuploadurl missing upload_param/upload_full_url: {}",
                upload_resp
            ));
        };

        // AES-128-ECB encrypt
        let ciphertext = aes128_ecb_encrypt(&plaintext, &aes_key);
        let encrypted_query_param = self.upload_ciphertext(&ciphertext, &upload_url).await?;

        // Build item_list based on media type
        let item_value = match media_type {
            MEDIA_IMAGE => json!({
                "type": ITEM_IMAGE,
                "image_item": {
                    "aes_key": hex::encode(aes_key),
                    "file_name": filename,
                    "file_size": rawsize,
                    "encrypted_query_param": encrypted_query_param,
                    "file_key": filekey,
                }
            }),
            MEDIA_VIDEO => json!({
                "type": ITEM_VIDEO,
                "video_item": {
                    "aes_key": hex::encode(aes_key),
                    "file_name": filename,
                    "file_size": rawsize,
                    "encrypted_query_param": encrypted_query_param,
                    "file_key": filekey,
                }
            }),
            MEDIA_VOICE => json!({
                "type": ITEM_VOICE,
                "voice_item": {
                    "aes_key": hex::encode(aes_key),
                    "file_name": filename,
                    "file_size": rawsize,
                    "encrypted_query_param": encrypted_query_param,
                    "file_key": filekey,
                }
            }),
            _ => json!({
                "type": ITEM_FILE,
                "file_item": {
                    "aes_key": hex::encode(aes_key),
                    "file_name": filename,
                    "file_size": rawsize,
                    "encrypted_query_param": encrypted_query_param,
                    "file_key": filekey,
                }
            }),
        };

        // Send the message with file attachment
        let client_id = format!(
            "xi:{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );

        let body = json!({
            "msg": {
                "from_user_id": "",
                "to_user_id": to_user,
                "client_id": client_id,
                "message_type": 2,
                "message_state": 2,
                "context_token": context_token,
                "item_list": [item_value],
            },
            "base_info": { "channel_version": CHANNEL_VERSION }
        });

        let token = self.bot_token.as_deref().ok_or("no bot_token set")?;
        let url = format!("{}/ilink/bot/sendmessage", self.base_url);
        let headers = common_headers_with_auth(self, token);
        let (_, result) = http_post(&self.client, &url, &headers, &body).await?;

        Ok(SendResponse {
            ret: result["ret"].as_i64().map(|v| v as i32),
            errcode: result["errcode"].as_i64().map(|v| v as i32),
        })
    }
}

// ── Request header builders ────────────────────────────────────

fn random_uin() -> String {
    "wx53984989".to_string()
}

fn encode_version(version: &str) -> String {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() == 3 {
        format!(
            "{:0>2}{:0>2}{:0>2}",
            parts[0].parse::<u32>().unwrap_or(0),
            parts[1].parse::<u32>().unwrap_or(0),
            parts[2].parse::<u32>().unwrap_or(0)
        )
    } else {
        version.to_string()
    }
}

fn common_headers(_wl: &WeiLink) -> Vec<(String, String)> {
    vec![
        ("Content-Type".to_string(), "application/json".to_string()),
        ("AuthorizationType".to_string(), "ilink_bot_token".to_string()),
        ("X-WECHAT-UIN".to_string(), random_uin()),
        ("iLink-App-Id".to_string(), "bot".to_string()),
        (
            "iLink-App-ClientVersion".to_string(),
            encode_version(CHANNEL_VERSION),
        ),
    ]
}

fn common_headers_with_auth<'a>(
    _wl: &'a WeiLink,
    token: &'a str,
) -> Vec<(String, String)> {
    vec![
        ("Content-Type".to_string(), "application/json".to_string()),
        ("AuthorizationType".to_string(), "ilink_bot_token".to_string()),
        ("Authorization".to_string(), format!("Bearer {}", token)),
        ("X-WECHAT-UIN".to_string(), random_uin()),
        ("iLink-App-Id".to_string(), "bot".to_string()),
        (
            "iLink-App-ClientVersion".to_string(),
            encode_version(CHANNEL_VERSION),
        ),
    ]
}

// ── Media / encryption helpers ─────────────────────────────────

use aes::Aes128;
use cipher::{BlockEncrypt, KeyInit};
use md5::{Digest, Md5};

const MEDIA_IMAGE: i32 = 1;
const MEDIA_VIDEO: i32 = 2;
const MEDIA_FILE: i32 = 3;
const MEDIA_VOICE: i32 = 4;

const ITEM_IMAGE: i32 = 2;
const ITEM_VOICE: i32 = 3;
const ITEM_FILE: i32 = 4;
const ITEM_VIDEO: i32 = 5;

fn classify_media(path: &str) -> (i32, i32) {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" => (MEDIA_IMAGE, ITEM_IMAGE),
        "mp4" | "avi" | "mov" | "mkv" | "wmv" => (MEDIA_VIDEO, ITEM_VIDEO),
        "mp3" | "wav" | "ogg" | "amr" | "aac" | "silk" => (MEDIA_VOICE, ITEM_VOICE),
        _ => (MEDIA_FILE, ITEM_FILE),
    }
}

fn pkcs7_pad(data: &[u8], block_size: usize) -> Vec<u8> {
    let pad_len = block_size - (data.len() % block_size);
    let mut padded = data.to_vec();
    padded.extend(std::iter::repeat(pad_len as u8).take(pad_len));
    padded
}

fn aes_padded_size(raw_size: usize) -> usize {
    ((raw_size + 1 + 15) / 16) * 16
}

fn aes128_ecb_encrypt(plaintext: &[u8], key: &[u8]) -> Vec<u8> {
    let cipher = Aes128::new_from_slice(key).expect("AES-128 key must be 16 bytes");
    let padded = pkcs7_pad(plaintext, 16);
    let mut result = padded;
    for chunk in result.chunks_mut(16) {
        cipher.encrypt_block(chunk.into());
    }
    result
}

fn md5_hex(data: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}
