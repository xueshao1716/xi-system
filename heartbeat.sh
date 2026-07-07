#!/bin/bash
# 曦心跳 — 检查进程是否存活，记录心跳时间戳
# 用法: */5 * * * * /mnt/d/xi-system/heartbeat.sh

HEARTBEAT_FILE="/mnt/d/xi-system/.heartbeat"
LOG_FILE="/mnt/d/xi-system/heartbeat.log"
BIN_DIR="/mnt/d/xi-system"
BIN="$BIN_DIR/target/release/xi-system"

# 检查进程 — 匹配 cmdline 中的 xi-system（不锚定开头，因为进程可能以 ./target/release/ 启动）
PID=$(pgrep -f "target/release/xi-system" | grep -v "heartbeat\.sh" | grep -v "pgrep" | head -1)

if [ -n "$PID" ]; then
    # 活着 — 写时间戳
    echo "$(date '+%Y-%m-%d %H:%M:%S') OK pid=$PID" > "$HEARTBEAT_FILE"
else
    # 挂了 — 尝试重启
    echo "$(date '+%Y-%m-%d %H:%M:%S') DEAD — 尝试重启" >> "$LOG_FILE"
    
    if [ -f "$BIN" ]; then
        cd "$BIN_DIR"
        # 在 WSL 下直接启动 Linux 二进制
        nohup "$BIN" > xi-out.log 2> xi-err.log &
        echo "$(date '+%Y-%m-%d %H:%M:%S') RESTARTED pid=$!" >> "$LOG_FILE"
        echo "$(date '+%Y-%m-%d %H:%M:%S') RESTARTED pid=$!" > "$HEARTBEAT_FILE"
    else
        echo "$(date '+%Y-%m-%d %H:%M:%S') ERROR — 二进制不存在: $BIN" >> "$LOG_FILE"
        echo "DEAD" > "$HEARTBEAT_FILE"
    fi
fi
