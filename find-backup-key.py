# -*- coding: utf-8 -*-
"""搜微信内存中备份相关标记（WXGBACKUP/目录名/字段8），提取附近密钥"""
import ctypes, subprocess
from ctypes import wintypes
import pymem

kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
class MBI(ctypes.Structure):
    _fields_ = [("BaseAddress", ctypes.c_void_p), ("AllocationBase", ctypes.c_void_p),
                ("AllocationProtect", wintypes.DWORD), ("RegionSize", ctypes.c_size_t),
                ("State", wintypes.DWORD), ("Protect", wintypes.DWORD), ("Type", wintypes.DWORD)]

# 字段8 和备份目录名
F8 = b"12249403060238773042wb6jvKakYNPFKnJyvgf5U7TMSyugikuwodAh7M3FQkMw"
DIR = b"7ecd61f6419e09e9d6dd3f3bb0731400"
WXBACKUP = b"WXGBACKUP"
needles = [F8, DIR, WXBACKUP, b"ChatPackage", b"chatpackage", b"ilink.im.sdk", b"RMFH"]

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
                if not buf: addr += mbi.RegionSize; continue
                for nd in needles:
                    idx = 0
                    while True:
                        i = buf.find(nd, idx)
                        if i == -1: break
                        hits += 1
                        print(f"\n=== 命中 #{hits}: {nd.decode('utf-8',errors='replace')[:20]} @ 0x{mbi.BaseAddress+i:x} ===")
                        start = max(0, i - 64)
                        end = min(len(buf), i + len(nd) + 160)
                        seg = buf[start:end]
                        # 打印 ASCII 可读部分
                        printable = ''.join(chr(b) if 32 <= b < 127 else '.' for b in seg)
                        print("前后 ASCII:", printable)
                        # 附近找 16/24/32/48 字节高熵（可能密钥）
                        for k in range(max(0, i-96), min(len(buf), i+len(nd)+96)-48):
                            chunk = buf[k:k+48]
                            # 高熵检查（>200 随机）
                            if sum(1 for b in chunk if 32 <= b < 127) < 10:
                                # 打印 hex 形式的 16/32 字节片段
                                pass
                        idx = i + 1
            except Exception:
                pass
    addr += mbi.RegionSize
print(f"\n总命中: {hits}")
