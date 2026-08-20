/// Health Monitor — 系统运行时自检模块
///
/// 跟踪：运行时长、消息吞吐、最后活跃时间、进程级健康
/// 插在主循环里每 tick 更新，不阻塞，不额外走网络。
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub struct HealthMonitor {
    /// 进程启动时间
    started: Instant,
    /// 最后一条消息到达时间
    last_msg_at: Option<Instant>,
    /// 消息总数
    msg_count: u64,
    /// 5h 窗口内的消息数
    window_msg_count: u64,
    /// 上次 5h 窗口重置时间
    window_start: Instant,
    /// 最后一次心跳检查时间
    last_heartbeat: Instant,
    /// 自上次心跳以来的消息数
    heartbeat_msg_count: u64,
    /// 心跳间隔（秒）
    heartbeat_interval: u64,
    /// 上次保存的进程状态快照
    last_mem_check: Instant,
    /// 缓存的内存数据
    mem_mb: f64,
    /// 进程 pid
    pid: u32,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
            last_msg_at: None,
            msg_count: 0,
            window_msg_count: 0,
            window_start: Instant::now(),
            last_heartbeat: Instant::now(),
            heartbeat_msg_count: 0,
            heartbeat_interval: 300,  // 5分钟
            last_mem_check: Instant::now(),
            mem_mb: 0.0,
            pid: std::process::id(),
        }
    }

    /// 记录一条消息到达
    pub fn record_msg(&mut self) {
        self.last_msg_at = Some(Instant::now());
        self.msg_count += 1;
        self.window_msg_count += 1;
    }

    /// 主循环 tick — 返回是否该触发心跳报告
    pub fn tick(&mut self) -> bool {
        // 5h 窗口滚动
        if self.window_start.elapsed() >= Duration::from_secs(5 * 3600) {
            self.window_start = Instant::now();
            self.window_msg_count = 0;
        }

        // 定期查内存（每 60s 一次）
        if self.last_mem_check.elapsed() >= Duration::from_secs(60) {
            self.last_mem_check = Instant::now();
            self.mem_mb = self.read_mem_mb();
        }

        // 心跳触发
        if self.last_heartbeat.elapsed() >= Duration::from_secs(self.heartbeat_interval) {
            self.last_heartbeat = Instant::now();
            self.heartbeat_msg_count = 0;
            return true;
        }
        false
    }

    /// 生成心跳报告文本
    pub fn heartbeat_line(&self) -> String {
        let uptime = self.started.elapsed();
        let uptime_str = format!(
            "{:02}:{:02}:{:02}",
            uptime.as_secs() / 3600,
            (uptime.as_secs() % 3600) / 60,
            uptime.as_secs() % 60
        );
        let last = self.last_msg_at
            .map(|t| format!("{:.0}s ago", t.elapsed().as_secs_f64()))
            .unwrap_or_else(|| "none".to_string());
        let rate = if self.msg_count > 0 {
            let secs = self.started.elapsed().as_secs_f64();
            if secs > 0.0 {
                format!("{:.1}/h", self.msg_count as f64 / secs * 3600.0)
            } else {
                "计算中".to_string()
            }
        } else {
            "0".to_string()
        };

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        format!(
            r#"{{"ts":{},"pid":{},"uptime":"{}","msg_total":{},"msg_rate":"{}","last_msg":"{}","mem_mb":{:.1}}}"#,
            ts, self.pid, uptime_str, self.msg_count, rate, last, self.mem_mb
        )
    }

    /// 人类可读状态摘要
    pub fn status_text(&self) -> String {
        let uptime = self.started.elapsed();
        let h = uptime.as_secs() / 3600;
        let m = (uptime.as_secs() % 3600) / 60;
        let last = self.last_msg_at
            .map(|t| format!("{:.0}s前", t.elapsed().as_secs_f64()))
            .unwrap_or_else(|| "无".to_string());
        format!(
            "运行 {}h{}m · {} 条消息 · 最后消息 {} · 内存 {:.1}MB · PID {}",
            h, m, self.msg_count, last, self.mem_mb, self.pid
        )
    }

    fn read_mem_mb(&self) -> f64 {
        let path = format!("/proc/{}/status", self.pid);
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        for line in content.lines() {
            if line.starts_with("VmRSS:") {
                if let Some(val) = line.split_whitespace().nth(1) {
                    if let Ok(kb) = val.parse::<f64>() {
                        return kb / 1024.0;
                    }
                }
            }
        }
        0.0
    }
}