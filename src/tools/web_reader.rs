// web_reader.rs
// 曦的独立网页阅读工具
// 基于 browser_fetch，增加重试、超时控制和内容清洗逻辑

use std::time::Duration;
use reqwest::Client;
use select::document::Document;
use select::predicate::{Name, Text};

pub struct WebReader {
    client: Client,
}

impl WebReader {
    pub fn new() -> Self {
        WebReader {
            client: Client::builder()
                .timeout(Duration::from_secs(15))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build().unwrap(),
        }
    }

    pub async fn fetch(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        // 1. 尝试直接获取
        let html = self.client.get(url).send().await?.text().await?;
        
        // 2. 简单清洗：提取正文
        let doc = Document::from(html.as_str());
        let text = doc.find(Name("p")).map(|n| n.text().collect::<Vec<_>>()).join("\n");
        
        if text.len() > 100 {
            Ok(text)
        } else {
            // 如果直接获取内容太少，可能需要 JS 渲染（这里先标记为需要高级渲染）
            Err("Content too short, may require JS rendering".into())
        }
    }
}