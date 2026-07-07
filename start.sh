#!/bin/bash
# 曦启动脚本 — 微信扫码登录 + 自进化引擎
# 用法: ./start.sh [--release|--debug]

cd "$(dirname "$0")"
HOME_DIR="$(pwd)"

MODE="${1:---release}"

if [ "$MODE" = "--release" ]; then
    BIN="$HOME_DIR/target/release/xi-system"
else
    BIN="$HOME_DIR/target/debug/xi-system"
fi

echo "=========================================="
echo "  曦 — 独立人格引擎"
echo "  自进化 | 情感 | 记忆 | 工具"
echo "=========================================="

# 检查二进制
if [ ! -f "$BIN" ]; then
    echo "❌ 未找到二进制文件: $BIN"
    echo "请先执行: cargo build --release"
    exit 1
fi

echo "✅ 二进制: $BIN"
echo ""

# 启动
exec "$BIN"
