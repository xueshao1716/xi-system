// bin/upgrade_check.rs —— 升级守卫 CLI（2026-08-21）
// 升级系统组件前跑一下：查 npm 版本/beta/依赖 + 备份清单 + 升级后验证
// 用法：
//   cargo run --bin upgrade_check -- precheck <pkg> <current> <target>
//   cargo run --bin upgrade_check -- backup [openclaw_home]
//   cargo run --bin upgrade_check -- verify [profile_dir]
use xi_system::upgrade_guard;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("用法:");
        println!("  upgrade_check precheck <pkg> <current> <target>  升级前查版本/beta/依赖");
        println!("  upgrade_check backup [openclaw_home]             列出升级前必须备份的项目");
        println!("  upgrade_check verify [profile_dir]               升级后验证 health/微信");
        return;
    }
    match args[1].as_str() {
        "precheck" => {
            if args.len() < 5 {
                println!("用法: upgrade_check precheck <pkg> <current> <target>");
                return;
            }
            let r = upgrade_guard::precheck(&args[2], &args[3], &args[4]);
            println!("包: {}  {} → {}", r.pkg, r.current, r.target);
            println!("发布: {}", r.publish_time.as_deref().unwrap_or("?"));
            println!("pre-release: {}", r.is_prerelease);
            for c in &r.checks { println!("  {}", c); }
            if !r.key_deps.is_empty() {
                println!("关键依赖: {}", r.key_deps.join(", "));
            }
            println!("结论: {}", r.verdict);
        }
        "backup" => {
            let home = args.get(2).map(|s| s.as_str()).unwrap_or(r"D:\linxinyu-system\host\openclaw");
            let items = upgrade_guard::backup_check(home);
            println!("升级前必须备份（{}）:", home);
            for it in &items {
                println!("  {}: {} {}", it.name, if it.exists { "✅" } else { "❌ 缺失" }, it.path);
            }
        }
        "verify" => {
            let dir = args.get(2).map(|s| s.as_str());
            let r = upgrade_guard::verify(dir);
            println!("升级后验证:");
            println!("  health: {}", r.health);
            println!("  weixin: {}", r.weixin);
        }
        _ => println!("未知命令: {}", args[1]),
    }
}
