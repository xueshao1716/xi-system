use reqwest::Client;

/// ___ DeepSeek API
pub async fn call_deepseek_direct(
    http_client: &Client,
    api_key: &str,
    model: &str,
    messages: Vec<serde_json::Value>,
    tools: &[serde_json::Value],
) -> Result<String, String> {
    let mut body = serde_json::json!({
        "model": model,
        "messages": messages,
        "max_tokens": 2048,
    });

    if !tools.is_empty() {
        body["tools"] = serde_json::json!(tools);
    }

    let resp = http_client
        .post("https://api.deepseek.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP ______: {:?}", e))?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("API ___ [{}]: {}", status, &text[..text.len().min(200)]));
    }

    // ____
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("JSON ____: {}", e))?;

    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    Ok(content)
}