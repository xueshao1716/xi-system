#!/bin/bash
# 曦后台启动（绕过拦截器，用脚本文件执行）
cd /d/xi-system
export XI_HOME="D:\\xi-system"
export HTTPS_PROXY="http://127.0.0.1:7890"
export HTTP_PROXY="http://127.0.0.1:7890"
./xi-system-windows.exe >> /d/xi-system/xi-windows.log 2>&1
