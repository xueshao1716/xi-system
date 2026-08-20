# Xi system watchdog - keep xi-system-windows alive (ASCII only)
# Loop every 20s: if process missing, restart it.
# Start as background process or via Task Scheduler.

$ErrorActionPreference = "SilentlyContinue"
$log = "D:\xi-system\xi-watchdog.log"
$exe = "D:\xi-system\xi-system-windows.exe"
$workDir = "D:\xi-system"
$outLog = "D:\xi-system\xi-windows.log"

function Write-Log($msg) {
  $line = "[{0}] {1}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss"), $msg
  Add-Content -Path $log -Value $line -Encoding ASCII
  Write-Output $line
}

Write-Log "watchdog started"

while ($true) {
  $p = Get-Process -Name "xi-system-windows" -ErrorAction SilentlyContinue
  if (-not $p) {
    Write-Log "DEAD - restarting"
    try {
      $env:XI_HOME = "D:\xi-system"
      # remove stale redirect file if present (previous process may have released it)
      Start-Process -FilePath $exe -WorkingDirectory $workDir -WindowStyle Hidden  # 2026-08-20 修复：PS5.1 中 Redirect+Hidden 不兼容（Hidden 失效会弹窗），去掉重定向优先隐藏
      Write-Log "STARTED xi-system-windows"
    } catch {
      Write-Log ("FAIL: " + $_.Exception.Message)
    }
  }
  Start-Sleep -Seconds 20
}