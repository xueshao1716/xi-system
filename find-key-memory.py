# -*- coding: utf-8 -*-
"""搜微信进程内存中备份密钥串附近的数据"""
import ctypes, re
from ctypes import wintypes
import pymem

KEY_STR = b"12249403060238773042wb6jvKakYNPFKnJyvgf5U7TMSyugikuwodAh7M3FQkMwR"
kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)

class MBI(ctypes.Structure):
    _fields_ = [("BaseAddress", ctypes.c_void_p), ("AllocationBase", ctypes.c_void_p),
                ("AllocationProtect", wintypes.DWORD), ("RegionSize", ctypes.c_size_t),
                ("State", wintypes.DWORD), ("Protect", wintypes.DWORD), ("Type", wintypes.DWORD)]

import subprocess
out = subprocess.run(["powershell", "-NoProfile", "-Command",
    "(Get-Process Weixin -ErrorAction SilentlyContinue | Sort-Object WorkingSet64 -Descending | Select-Object -First 1).Id"],
    capture_output=True, text=True).stdout.strip()
pid = int(out) if out.isdigit() else None
print("Weixin PID:", pid)
pm = pymem.Pymem()
pm.open_process_from_id(pid)

hits = 0
addr = 0
while addr < 0x7FFFFFFFFFFF:
    mbi = MBI()
    r = kernel32.VirtualQueryEx(pm.process_handle, ctypes.c_void_p(addr), ctypes.byref(mbi), ctypes.sizeof(mbi))
    if r == 0: break
    if mbi.State == 0x1000 and 0 < mbi.RegionSize < 0x2000000:
        p = mbi.Protect & 0xFF
        if p in (0x02, 0x04, 0x20, 0x40, 0x80):
            try:
                buf = pm.read_bytes(mbi.BaseAddress, mbi.RegionSize)
                if buf:
                    idx = 0
                    while True:
                        i = buf.find(KEY_STR, idx)
                        if i == -1: break
                        hits += 1
                        # dump 前后
                        start = max(0, i - 64)
                        end = min(len(buf), i + len(KEY_STR) + 128)
                        seg = buf[start:end]
                        print(f"\n=== 命中 #{hits} 地址 0x{mbi.BaseAddress + i:x} 偏移 {i} ===")
                        print("前 64:", seg[:64].hex())
                        print("后 128:", seg[len(KEY_STR)+64 if len(seg) > len(KEY_STR)+64 else 64:].hex())
                        # 也看 hex 字符串形式
                        idx = i + 1
            except Exception:
                pass
    addr += mbi.RegionSize

print(f"\n总命中: {hits}")
