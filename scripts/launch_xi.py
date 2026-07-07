#!/usr/bin/env python3
"""Launch xi-system in background. Call from cron or manual."""
import subprocess
import sys
import os
import time

home = "/mnt/d/xi-system"
binary = os.path.join(home, "target/release/xi-system")
log = "/tmp/xi_v3_stdout.log"
err = "/tmp/xi_v3_stderr.log"

# Kill existing
subprocess.run(["pkill", "-f", binary], capture_output=True)
time.sleep(1)

# Start fresh
with open(log, "w") as out, open(err, "w") as err_f:
    proc = subprocess.Popen(
        [binary],
        stdout=out,
        stderr=err_f,
        cwd=home,
        preexec_fn=os.setsid,
    )
    print(f"Started xi-system (PID {proc.pid})")
    print(f"Log: {log}")
    print(f"Err: {err}")

# Wait for startup
time.sleep(3)
with open(log) as f:
    startup = f.read()
    if "曦已就绪" in startup or "就绪" in startup:
        print("✅ Startup OK")
    else:
        print("⚠️  May still be starting:")
        print(startup[-500:])
