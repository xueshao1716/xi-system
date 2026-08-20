@echo off
cd /d D:\xi-system
set "XI_HOME=D:\xi-system"
set "HTTPS_PROXY=http://127.0.0.1:7890"
set "HTTP_PROXY=http://127.0.0.1:7890"
start "" /min cmd /c ""D:\xi-system\xi-system-windows.exe" > "D:\xi-system\xi-windows.log" 2>&1"
