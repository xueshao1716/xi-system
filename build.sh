#!/bin/bash
# build.sh — 曦的一键编译脚本
# 用法: bash build.sh [debug|release]
# 
# 在 WSL 或 Windows Git Bash 里都能跑

set -e

MODE=${1:-release}
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$SCRIPT_DIR"

echo "🔨 编译 xi-system ($MODE)..."
echo "   项目目录: $PROJECT_DIR"

cd "$PROJECT_DIR"

if [ "$MODE" = "debug" ]; then
    cargo check --bin xi-system
    echo "✅ Debug check 通过"
else
    cargo build --release --bin xi-system
    echo "✅ Release 编译完成"
    echo "   输出: target/release/xi-system"
    ls -lh target/release/xi-system 2>/dev/null
fi

echo ""
echo "重启服务: systemctl --user restart xi-system.service"
