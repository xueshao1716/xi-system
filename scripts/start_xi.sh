#!/bin/bash
# xi-startup.sh — kill old, start new
LOG="/tmp/xi_v3_stdout.log"
ERR="/tmp/xi_v3_stderr.log"

pkill -f "target/release/xi-system" 2>/dev/null
sleep 1
rm -f "$LOG" "$ERR"

cd /mnt/d/xi-system || exit 1
nohup ./target/release/xi-system > "$LOG" 2>"$ERR" &
echo "Started PID $!"
sleep 3
head -15 "$LOG"
