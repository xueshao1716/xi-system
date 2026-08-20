#!/bin/bash
# run-xi.sh — 曦的启动包装：动态获取 Windows 宿主 IP 配置代理后启动
set -e

HOST_IP=$(ip route | awk '/default/ {print $3; exit}')
if [ -n "$HOST_IP" ]; then
    export HTTPS_PROXY="http://$HOST_IP:7890"
    export HTTP_PROXY="http://$HOST_IP:7890"
    export ALL_PROXY="http://$HOST_IP:7890"
fi
export XI_HOME=/mnt/d/xi-system
# 调试：记录环境
{ echo "=== $(date) ==="; echo "HOST_IP=$HOST_IP"; env | grep -iE 'proxy|XI_HOME'; } > /mnt/d/xi-system/proxy-env-debug.txt 2>&1
cd /mnt/d/xi-system
exec ./target/release/xi-system
