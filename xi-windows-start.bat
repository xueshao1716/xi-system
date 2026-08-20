@echo off
rem 曦（xi-system）Windows 原生启动脚本 — 开机/登录自启用
cd /d D:\xi-system

set "XI_HOME=D:\xi-system"
set "HTTPS_PROXY=http://127.0.0.1:7890"
set "HTTP_PROXY=http://127.0.0.1:7890"
set "ALL_PROXY=http://127.0.0.1:7890"

rem 清理旧进程（避免微信 token 双实例冲突）
taskkill /F /IM xi-system-windows.exe >nul 2>&1
timeout /t 2 /nobreak >nul

rem 后台启动并把日志写入文件
start "" /min cmd /c ""D:\xi-system\xi-system-windows.exe" > "D:\xi-system\xi-windows.log" 2>&1"
