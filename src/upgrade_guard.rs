// upgrade_guard.rs — 升级守卫（从旧版 python 移植 2026-08-20）
//
// 2026-08-12 教训：升级 OpenClaw 8.1-beta.1 造成诗微信离线 + 数据库/配置不可逆升级，
// 花了一晚上回滚。根因：没查兼容性就升了最新 beta。
//
// 规则（写进机制，不靠自觉）：
//   1. beta 版本升级前必须查：npm 发布时间、known issues、依赖兼容
//   2. 升级前必须备份：config + state db + agent db + 插件目录
//   3. 升级后必须验证：health + 微信连接，全过才算成功
//   4. 回滚不是万能：schema 升级是单向的——更要谨慎
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecheckResult {
    pub pkg: String,
    pub current: String,
    pub target: String,
    pub publish_time: Option<String>,
    pub is_prerelease: bool,
    pub key_deps: Vec<String>,
    pub checks: Vec<String>,
    pub verdict: String, // OK / CAUTION / BLOCKED / UNKNOWN
}

/// 升级前检查：查 npm registry 目标版本（发布时间 / 是否 prerelease / 关键依赖）
/// 返回 verdict：OK(稳定版) / CAUTION(beta) / BLOCKED(查不到)
pub fn precheck(pkg: &str, _current: &str, target: &str) -> PrecheckResult {
    let mut out = PrecheckResult {
        pkg: pkg.to_string(),
        current: _current.to_string(),
        target: target.to_string(),
        publish_time: None,
        is_prerelease: target.contains('-'),
        key_deps: vec![],
        checks: vec![],
        verdict: "UNKNOWN".into(),
    };

    let url = format!("https://registry.npmjs.org/{}/{}", pkg, target);
    match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .and_then(|c| c.get(&url).send())
    {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<serde_json::Value>() {
                Ok(data) => {
                    if let Some(t) = data["time"][target].as_str() {
                        out.publish_time = Some(t.to_string());
                    }
                    if out.is_prerelease {
                        out.checks.push("⚠️ 目标版本是 prerelease（beta/alpha），有坑风险".into());
                        out.verdict = "CAUTION".into();
                    } else {
                        out.checks.push("✅ 目标版本是稳定版".into());
                        out.verdict = "OK".into();
                    }
                    // 关键依赖（@openclaw 系列）
                    if let Some(deps) = data["dependencies"].as_object() {
                        out.key_deps = deps
                            .iter()
                            .filter(|(k, _)| k.starts_with("@openclaw"))
                            .map(|(k, v)| format!("{}={}", k, v.as_str().unwrap_or("?")))
                            .collect();
                    }
                }
                Err(e) => {
                    out.checks.push(format!("❌ 解析 registry 响应失败: {}", e));
                    out.verdict = "BLOCKED".into();
                }
            }
        }
        Ok(resp) => {
            out.checks.push(format!("❌ 查版本失败: HTTP {}", resp.status()));
            out.verdict = "BLOCKED".into();
        }
        Err(e) => {
            out.checks.push(format!("❌ 查版本失败: {}", e));
            out.verdict = "BLOCKED".into();
        }
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupItem {
    pub name: String,
    pub exists: bool,
    pub path: String,
}

/// 升级前必须备份的项目清单（OpenClaw profile 结构）
pub fn backup_check(openclaw_home: &str) -> Vec<BackupItem> {
    let profile = format!("{}\\profile", openclaw_home);
    let items = [
        ("config", format!("{}\\openclaw.json", profile)),
        ("state_db", format!("{}\\state\\openclaw.sqlite", profile)),
        ("agent_db", format!("{}\\agents\\main\\agent\\openclaw-agent.sqlite", profile)),
        ("weixin_plugin", format!("{}\\npm\\node_modules\\@tencent-weixin", profile)),
    ];
    items
        .iter()
        .map(|(name, path)| BackupItem {
            name: name.to_string(),
            exists: Path::new(path).exists(),
            path: path.clone(),
        })
        .collect()
}

/// 升级后验证三件套：health + 微信（channel exited 检测）
pub struct VerifyResult {
    pub health: String,
    pub weixin: String,
}

pub fn verify(profile_dir: Option<&str>) -> VerifyResult {
    let mut out = VerifyResult { health: "?".into(), weixin: "?".into() };

    // 1. health
    match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .and_then(|c| c.get("http://127.0.0.1:18789/health").send())
    {
        Ok(r) => out.health = if r.status().as_u16() == 200 { "OK".into() } else { format!("HTTP {}", r.status()) },
        Err(e) => out.health = format!("FAIL: {}", e),
    }

    // 2. 微信：stderr 日志有无 "channel exited" / "reading 'logger'"
    let d = profile_dir.unwrap_or(r"D:\linxinyu-system\host\openclaw\profile");
    let err_log = std::path::Path::new(d).join("gw14_stderr.log");
    if let Ok(tail) = fs::read_to_string(&err_log) {
        let tail = tail.chars().rev().take(5000).collect::<String>().chars().rev().collect::<String>();
        let exited = tail.contains("channel exited");
        let logger = tail.contains("reading 'logger'");
        if exited || logger {
            out.weixin = format!("FAIL: {}{}", if exited { "channel exited " } else { "" }, if logger { "logger bug" } else { "" });
        } else {
            out.weixin = "OK（无 channel exited）".into();
        }
    } else {
        out.weixin = "?（无 gw14_stderr.log）".into();
    }
    out
}

use std::fs;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prerelease_detected() {
        let r = precheck("some-pkg", "1.0.0", "2.0.0-beta.1");
        assert!(r.is_prerelease);
        // 网络失败时也应有 verdict（BLOCKED 或 UNKNOWN），不 panic
        assert!(!r.verdict.is_empty());
    }

    #[test]
    fn stable_detected() {
        let r = precheck("some-pkg", "1.0.0", "2.0.0");
        assert!(!r.is_prerelease);
    }

    #[test]
    fn backup_list_shape() {
        let items = backup_check(r"D:\linxinyu-system\host\openclaw");
        assert_eq!(items.len(), 4);
        for it in &items {
            assert!(!it.path.is_empty());
        }
    }

    #[test]
    fn verify_no_panic() {
        let r = verify(None);
        assert!(!r.health.is_empty());
    }
}
