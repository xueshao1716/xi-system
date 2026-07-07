#!/bin/bash
# 曦看门狗 — 崩溃自动重启
# 用法: nohup ./watchdog.sh > watchdog.log 2>&1 &

cd "$(dirname "$0")"
HOME_DIR="$(pwd)"
BIN="$HOME_DIR/target/release/xi-system"

echo "曦看门狗已启动"
echo "PID: $$"
echo "日志: $HOME_DIR/watchdog.log"
echo ""

RESTART_DELAY=5
COUNT=0

while true; do
    COUNT=$((COUNT + 1))
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] 第 ${COUNT} 次启动曦..."
    
    # 检查二进制是否存在
    if [ ! -f "$BIN" ]; then
        echo "❌ 未找到二进制，等 30 秒再试..."
        sleep 30
        continue
    fi
    
    # 启动（阻塞直到退出）
    "$BIN"
    
    EXIT_CODE=$?
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] ⚠ 曦已退出 (exit code: $EXIT_CODE)"
    
    if [ $EXIT_CODE -eq 0 ]; then
        echo "   正常退出，不重启"
        exit 0
    fi
    
    echo "   异常退出，${RESTART_DELAY} 秒后重启..."
    sleep "$RESTART_DELAY"
done
