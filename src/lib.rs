/// _?v2 _?____?///
/// __________?main.rs ____________


pub mod aibody_bridge;
pub mod assets;
pub mod brain;
pub mod ctx2soft;
pub mod pi_skill_loader;
pub mod dream;
pub mod emotion;
pub mod evolution;
pub mod grn;
pub mod matrix_bridge;
pub mod memory;
pub mod organs;
pub mod proactive;
pub mod risk_guard;
pub mod anti_homogenization;
pub mod upgrade_guard;
pub mod relationship;
pub mod working_memory;
pub mod mother;
pub mod reflexion;
pub mod scenario;
pub mod soul;
pub mod throat;
pub mod tools;
pub mod wechat;

/// 曦的工作目录。默认 WSL 路径；Windows 原生运行时设 XI_HOME=D:\xi-system
pub fn xi_home() -> String {
    std::env::var("XI_HOME").unwrap_or_else(|_| "D:\\xi-system".to_string())
}
