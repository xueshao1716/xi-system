#!/usr/bin/env python3
"""Launch xi-system in background."""
import subprocess, time, os, sys

home = "/mnt/d/xi-system"
binary = os.path.join(home, "target/release/xi-system")
log = "/tmp/xi_v3_stdout.log"
err = "/tmp/xi_v3_stderr.log"

# Kill old
subprocess.run(["pkill", "-f", "target/release/xi-system"], capture_output=True, timeout=5)
time.sleep(2)

# Fresh start
for p in [log, err]:
    if os.path.exists(p):
        os.remove(p)

proc = subprocess.Popen(
    [binary],
    stdout=open(log, "w"),
    stderr=open(err, "w"),
    cwd=home,
    preexec_fn=os.setsid,
)

time.sleep(5)
with open(log) as f:
    content = f.read()

print(f"PID={proc.pid}")
print(content[-800:] if len(content) > 800 else content)
print("---")
with open(err) as f:
    err_content = f.read()
    if err_content.strip():
        print("STDERR:", err_content[-500:])
