//! 测试 rig 的 DeepSeek 直连
use std::env;
use rig::providers::openai;
use rig::client::CompletionClient;

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

    println!("正在连接 DeepSeek (rig reqwest 0.13)...");
    println!("API Key: sk-{}...", &api_key[..5]);

    let client = openai::CompletionsClient::builder()
        .api_key(&api_key)
        .base_url("https://api.deepseek.com/v1")
        .build();

    match client {
        Ok(c) => {
            println!("客户端构建成功");
            let agent = c.agent("deepseek-chat")
                .preamble("You are a helpful assistant.")
                .build();

            println!("正在调用 prompt...");
            match agent.prompt("Say hello in one word.").await {
                Ok(reply) => println!("✅ 回复: {}", reply),
                Err(e) => println!("❌ 错误: {:?}", e),
            }
        }
        Err(e) => println!("❌ 客户端构建失败: {:?}", e),
    }
}
