// TLS 连接测试 — 使用项目的 reqwest 配置
use std::env;

#[tokio::main]
async fn main() {
    let api_key = env::var("XI_API_KEY").unwrap_or_else(|_| {
        let home = env::var("HOME").unwrap_or_else(|_| "/mnt/d/xi-system".to_string());
        let config_path = format!("{}/config.json", home);
        let config: serde_json::Value = std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        config["llm"]["api_key"].as_str().unwrap_or("").to_string()
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("构建客户端失败");

    let body = serde_json::json!({
        "model": "deepseek-chat",
        "messages": [{"role": "user", "content": "Say hi in one word."}],
        "max_tokens": 5
    });

    println!("=== 测试 1: POST 到 DeepSeek ===");
    let resp = client
        .post("https://api.deepseek.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await;

    match resp {
        Ok(r) => {
            let status = r.status();
            let text = r.text().await.unwrap_or_default();
            println!("状态: {}", status);
            println!("响应(前200字): {}", &text[..text.len().min(200)]);
            if status.is_success() {
                println!("✅ TLS + API 测试通过!");
            }
        }
        Err(e) => {
            println!("❌ 请求失败: {:?}", e);
            // 检查是否是 TLS 错误
            let err_str = format!("{:?}", e);
            if err_str.contains("tls") || err_str.contains("certificate") || err_str.contains("cert") {
                println!("   → 可能是 TLS 证书问题");
            }
        }
    }

    println!("\n=== 测试 2: HTTPS GET (验证网络) ===");
    let resp2 = client.get("https://api.deepseek.com/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await;
    match resp2 {
        Ok(r) => println!("GET 状态: {}", r.status()),
        Err(e) => println!("GET 失败: {:?}", e),
    }

    println!("\n=== 测试 3: SSL_CERT_FILE 环境变量 ===");
    println!("SSL_CERT_FILE: {:?}", env::var("SSL_CERT_FILE").unwrap_or_default());
}
