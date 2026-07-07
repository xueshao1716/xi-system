/// ______ _?______
///
/// __"______placeholder
/// ______ _?______ _?______?_?______
////// __________?
///   1. ____________ web_get_______________?
///   2. _____________________
///   3. ____________________________________?
////// 2026.6 _________________________________?

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

// __ _______?______________________________
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sight {
    pub url: String,
    pub title: String,
    pub content: String,
    pub source_type: SourceType,
    pub length: usize,
    pub quality: QualityGuess,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SourceType {
    WeChatArticle,
    TechnicalBlog,
    NewsPage,
    SocialFeed,
    SearchResult,
    ApiResponse,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QualityGuess {
    High,
    Medium,
    Low,
    Trash,
}

// __ ______ ________________________________
pub struct Eyes {
    /// ____________?20 ___
    recent_sights: VecDeque<Sight>,
    /// ___________________?
    remembered: Vec<Sight>,
    /// ___________________
    is_looking: bool,
}

impl Eyes {
    pub fn new() -> Self {
        Self {
            recent_sights: VecDeque::with_capacity(20),
            remembered: Vec::new(),
            is_looking: false,
        }
    }

    /// _______?_?___ + __ + __
    pub async fn look(&mut self, url: &str) -> Sight {
        self.is_looking = true;
        // 1. _?
        let raw = self.fetch(url).await;
        // 2. __
        let title = self.extract_title(&raw, url);
        let body = self.extract_body(&raw);
        // 3. ____
        let source_type = self.classify_url(url);
        // 4. _?
        let quality = self.judge(&body, &title, &source_type);
        let sight = Sight {
            url: url.to_string(),
            title,
            content: body,
            source_type,
            length: raw.len(),
            quality,
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        };
        // ________
        self.recent_sights.push_back(sight.clone());
        if self.recent_sights.len() > 20 {
            self.recent_sights.pop_front();
        }
        self.is_looking = false;
        sight
    }

    /// _________
    pub fn remember(&mut self, url: &str) -> bool {
        if let Some(sight) = self.recent_sights.iter().find(|s| s.url == url) {
            // _________
            if !self.remembered.iter().any(|r| r.url == url) {
                self.remembered.push(sight.clone());
            }
            true
        } else {
            false
        }
    }

    /// ___________
    pub fn recent_summary(&self) -> Vec<String> {
        self.recent_sights.iter().rev().take(5).map(|s| {
            format!("  [{:?}] {} _?{} ({}_?",
                s.quality, s.title, s.url, s.content.len())
        }).collect()
    }

    /// _________?
    fn remembered_summary(&self) -> Vec<String> {
        self.remembered.iter().map(|s| {
            format!("  __ {} _?{}", s.title, s.url)
        }).collect()
    }

    // __ ______ __________________________
    /// ______
    async fn fetch(&self, url: &str) -> String {
        // _______?fetch ___
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build().ok();
        if let Some(client) = client {
            if let Ok(resp) = client.get(url).send().await {
                if let Ok(text) = resp.text().await {
                    if text.len() > 100 { // _________
                        return text;
                    }
                }
            }
        }
        String::new()
    }

    /// ______
    fn extract_title(&self, html: &str, fallback_url: &str) -> String {
        // _?<title> __
        if let Some(start) = html.find("<title>") {
            let from = start + 7;
            if let Some(end) = html[from..].find("</title>") {
                let t = html[from..from + end].trim();
                if !t.is_empty() {
                    return t.to_string();
                }
            }
        }
        // _?og:title __
        if let Some(start) = html.find("property=\"og:title\"") {
            let search = &html[start..];
            if let Some(content_start) = search.find("content=\"") {
                let from = content_start + 9;
                if let Some(end) = search[from..].find('"') {
                    let t = search[from..from + end].trim();
                    if !t.is_empty() {
                        return t.to_string();
                    }
                }
            }
        }
        // _?#activity-name (____? __
        if let Some(start) = html.find("id=\"activity-name\"") {
            let search = &html[start..];
            if let Some(gt) = search.find('>') {
                let from = gt + 1;
                if let Some(lt) = search[from..].find('<') {
                    let t = search[from..from + lt].trim();
                    if !t.is_empty() {
                        return t.to_string();
                    }
                }
            }
        }
        // ___
        fallback_url.split('/').last().unwrap_or("______").to_string()
    }

    /// _________________________?
    fn extract_body(&self, html: &str) -> String {
        // __________?js_content
        if let Some(start) = html.find("id=\"js_content\"") {
            let search = &html[start..];
            if let Some(gt) = search.find('>') {
                let from = gt + 1;
                if let Some(end) = search.rfind("</div>") {
                    let raw = &search[from..end];
                    return self.strip_html(raw);
                }
            }
        }
        // _________ script/style___ body __________?
        let no_scripts = self.remove_tags(html, &["script", "style", "noscript", "svg"]);
        let body = self.get_body(&no_scripts);
        let text = self.strip_html(&body);
        // ___________________________
        if text.len() < 50 {
            self.strip_html(&no_scripts)
        } else {
            text
        }
    }

    /// _______________
    fn remove_tags(&self, html: &str, tags: &[&str]) -> String {
        let mut result = html.to_string();
        for tag in tags {
            while let Some(start) = result.find(&format!("<{}", tag)) {
                let search = &result[start..];
                if let Some(end) = search.find(&format!("</{}>", tag)) {
                    let close = end + tag.len() + 3;
                    result.replace_range(start..start + close, "");
                } else {
                    break;
                }
            }
        }
        result
    }

    /// _?<body> ___
    fn get_body<'a>(&self, html: &'a str) -> &'a str {
        if let Some(start) = html.find("<body") {
            let from = html[start..].find('>').map(|i| start + i + 1).unwrap_or(0);
            if let Some(end) = html[from..].find("</body>") {
                return &html[from..from + end];
            }
        }
        html
    }

    /// _?HTML __________?
    fn strip_html(&self, text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut in_tag = false;
        let mut in_entity = false;
        let mut entity_buf = String::new();
        for c in text.chars() {
            match c {
                '<' => in_tag = true,
                '>' if in_tag => in_tag = false,
                '&' if !in_tag => {
                    in_entity = true;
                    entity_buf.clear();
                }
                ';' if in_entity => {
                    in_entity = false;
                    let decoded = match entity_buf.as_str() {
                        "amp" => "&",
                        "lt" => "<",
                        "gt" => ">",
                        "quot" => "\"",
                        "#39" | "apos" => "'",
                        "nbsp" => " ",
                        _ => "",
                    };
                    result.push_str(decoded);
                }
                _ if !in_tag && !in_entity => {
                    result.push(c);
                }
                _ if in_entity => {
                    entity_buf.push(c);
                }
                _ => {}
            }
        }
        // _________
        let lines: Vec<&str> = result.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        // ________________________
        let mut clean = String::new();
        let mut para = String::new();
        for line in lines {
            if line.len() < 80 && !para.is_empty() {
                para.push(' ');
                para.push_str(line);
            } else {
                if !para.is_empty() {
                    clean.push_str(&para);
                    clean.push('\n');
                }
                para = line.to_string();
            }
        }
        if !para.is_empty() {
            clean.push_str(&para);
        }
        clean
    }

    /// ______
    fn classify_url(&self, url: &str) -> SourceType {
        if url.contains("mp.weixin.qq.com") {
            SourceType::WeChatArticle
        } else if url.contains("github.com") || url.contains("arxiv.org")
            || url.contains("rust-lang.org") || url.contains("docs.rs")
        {
            SourceType::TechnicalBlog
        } else if url.contains("zhihu.com") || url.contains("douban.com")
            || url.contains("x.com") || url.contains("weibo.com")
        {
            SourceType::SocialFeed
        } else if url.contains("news") || url.contains("36kr")
            || url.contains("huxiu")
        {
            SourceType::NewsPage
        } else {
            SourceType::Unknown
        }
    }

    /// _________
    fn judge(&self, body: &str, title: &str, source: &SourceType) -> QualityGuess {
        let len = body.len();
        // ___ = ___
        if len < 100 {
            return QualityGuess::Trash;
        }
        // ___________________?
        if len < 500 {
            return QualityGuess::Low;
        }
        // __________?= _______?
        let high_signals = ["tutorial", "guide", "paper", "implementation", "deep dive"];
        let has_high_signal = high_signals.iter().any(|s| {
            title.to_lowercase().contains(s)
                || body[..body.len().min(500)].to_lowercase().contains(s)
        });
        // __________?= ___
        let low_signals = ["placeholder", "test"];
        let has_low_signal = low_signals.iter().any(|s| title.contains(s));
        match (has_high_signal, has_low_signal, len) {
            (true, false, _) => QualityGuess::High,
            (false, true, _) => QualityGuess::Low,
            (_, _, l) if l > 3000 => QualityGuess::Medium,
            _ => QualityGuess::Low,
        }
    }
}

// __ ___ __________________________________
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_wechat() {
        let eyes = Eyes::new();
        assert_eq!(
            eyes.classify_url("https://mp.weixin.qq.com/s/something"),
            SourceType::WeChatArticle
        );
    }

    #[test]
    fn test_classify_github() {
        let eyes = Eyes::new();
        assert_eq!(
            eyes.classify_url("https://github.com/user/repo"),
            SourceType::TechnicalBlog
        );
    }

    #[test]
    fn test_judge_trash() {
        assert_eq!(
            Eyes::new().judge("short", "test", &SourceType::Unknown),
            QualityGuess::Trash
        );
    }

    #[test]
    fn test_strip_html() {
        let eyes = Eyes::new();
        let result = eyes.strip_html("<p>Hello <b>world</b></p>");
        assert!(result.contains("Hello"));
        assert!(result.contains("world"));
    }
}
