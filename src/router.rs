/// _______?_?_?xinyu-core/kernel/router.py ___
///
/// ___________________________?
#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    Talk,   // ___/____________?
    Write,  // ___/__
    Query,  // ___/_________
    Task,   // ____________
    System, // ______/___
    Learn,  // ___/___
}

impl Intent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Intent::Talk => "talk",
            Intent::Write => "write",
            Intent::Query => "query",
            Intent::Task => "task",
            Intent::System => "system",
            Intent::Learn => "learn",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RouteDecision {
    pub intent: Intent,
    pub action: Option<String>,   // write/edit/publish
    pub platforms: Vec<String>,   // ______
    pub hitl_required: bool,      // _________?
    pub schedule: bool,           // ______
}

pub struct Router;

impl Router {
    /// _____________?    
    fn route(text: &str) -> RouteDecision {
        let intent = Self::classify_intent(text);
        let params = Self::extract_params(text, &intent);
        params
    }

    fn classify_intent(text: &str) -> Intent {
        let lower = text.to_lowercase();

        // _______?        // talk
        let talk_kws = ["___", "placeholder", "___", "__", "___", "___", "___", "hi", "hello", "___", "__"];
        for kw in &talk_kws {
            if lower.contains(kw) { return Intent::Talk; }
        }

        // write
        let write_kws = ["placeholder", "___", "placeholder", "___", "___", "placeholder", "placeholder", "___", "___"];
        for kw in &write_kws {
            if lower.contains(kw) { return Intent::Write; }
        }

        // learn
        let learn_kws = ["___", "___", "___", "___", "___", "___", "___"];
        for kw in &learn_kws {
            if lower.contains(kw) { return Intent::Learn; }
        }

        // system
        let system_kws = ["placeholder", "___", "___", "placeholder", "___", "___", "___", "___", "/recall", "/status"];
        for kw in &system_kws {
            if lower.contains(kw) || lower.starts_with(kw) { return Intent::System; }
        }

        // query
        let query_kws = ["___", "placeholder", "___", "placeholder", "___", "___", "_______", "placeholder", "_____", "___"];
        for kw in &query_kws {
            if lower.contains(kw) { return Intent::Query; }
        }

        // task
        let task_kws = ["___", "placeholder", "___", "___", "___", "___", "___", "__", "__"];
        for kw in &task_kws {
            if lower.contains(kw) { return Intent::Task; }
        }

        // _____
        if text.chars().count() < 6 {
            return Intent::Talk;
        }
        if text.contains('?') || text.contains("什么") || text.contains("怎么") || text.contains("为什么") || text.contains("explain") || text.contains("help") {
            return Intent::Query;
        }

        Intent::Talk // ___
    }

    fn extract_params(text: &str, intent: &Intent) -> RouteDecision {
        let lower = text.to_lowercase();
        let mut decision = RouteDecision {
            intent: intent.clone(),
            action: None,
            platforms: Vec::new(),
            hitl_required: false,
            schedule: false,
        };

        match intent {
            Intent::Write => {
                // ______?
                if lower.contains("placeholder") { decision.platforms.push("xiaohongshu".into()); }
                if lower.contains("placeholder") || lower.contains("___") { decision.platforms.push("weixin".into()); }
                if lower.contains("___") { decision.platforms.push("blog".into()); }
                if lower.contains("___") || lower.contains("twitter") || lower.contains("x") { decision.platforms.push("twitter".into()); }
                // ______?
                if lower.contains("placeholder") || lower.contains("placeholder") { decision.action = Some("edit".into()); }
                else if lower.contains("placeholder") { decision.action = Some("publish".into()); }
                else { decision.action = Some("write".into()); }
            }
            Intent::Task => {
                if lower.contains("deploy") || lower.contains("build") || lower.contains("run") {
                    decision.hitl_required = true;
                }
                if lower.contains("schedule") || lower.contains("later") || lower.contains("tomorrow") || lower.contains("cron") {
                    decision.schedule = true;
                }
            }
            _ => {}
        }

        decision
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_talk() {
        assert_eq!(Router::route("___").intent, Intent::Talk);
    }

    #[test]
    fn test_write() {
        let r = Router::route("____________");
        assert_eq!(r.intent, Intent::Write);
        assert!(r.platforms.contains(&"xiaohongshu".to_string()));
        assert_eq!(r.action.as_deref(), Some("write"));

        let r = Router::route("__________________");
        assert_eq!(r.intent, Intent::Write);
        assert!(r.platforms.contains(&"weixin".to_string()));
        assert_eq!(r.action.as_deref(), Some("edit"));
    }

    #[test]
    fn test_query() {
        let r = Router::route("__________________");
        assert_eq!(r.intent, Intent::Query);
    }

    #[test]
    fn test_system() {
        let r = Router::route("placeholder");
        assert_eq!(r.intent, Intent::System);
        let r = Router::route("/recall");
        assert_eq!(r.intent, Intent::System);
    }

    #[test]
    fn test_task_hitl() {
        let r = Router::route("_________");
        assert_eq!(r.intent, Intent::Task);
        assert!(r.hitl_required);
    }

    #[test]
    fn test_learn() {
        let r = Router::route("placeholder");
        assert_eq!(r.intent, Intent::Learn);
    }

    #[test]
    fn test_short_fallback() {
        let r = Router::route("placeholder");
        assert_eq!(r.intent, Intent::Talk);
    }
}