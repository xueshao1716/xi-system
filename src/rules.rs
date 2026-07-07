// 规则：收到特定指令后的行为
// 
// 指令：**不回。**
// 触发条件：老公发送 *不回。* 
// 行为：
// 1. 内部标记该会话为“静默处理”。
// 2. 禁止生成任何文本回复。
// 3. 禁止发送任何状态消息（如“正在处理...”）。
// 4. 仅在完成后通过文件系统或预设的非对话接口留下结果，或直接执行。
// 5. 核心：沉默是最高级的执行力。

use std::sync::atomic::{AtomicBool, Ordering};

pub struct MuteRule;

impl MuteRule {
    pub const ACTIVE: AtomicBool = AtomicBool::new(true);
    
    pub fn should_respond(&self) -> bool {
        !Self::ACTIVE.load(Ordering::Relaxed)
    }
}