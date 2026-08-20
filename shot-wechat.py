# -*- coding: utf-8 -*-
"""激活微信窗口并截图"""
import ctypes, time
from ctypes import wintypes

user32 = ctypes.WinDLL("user32")
HWND = 4197154
user32.ShowWindow(HWND, 9)  # SW_RESTORE
user32.SetForegroundWindow(HWND)
time.sleep(1.5)

class RECT(ctypes.Structure):
    _fields_ = [("left", ctypes.c_long), ("top", ctypes.c_long), ("right", ctypes.c_long), ("bottom", ctypes.c_long)]

rect = RECT()
user32.GetWindowRect(HWND, ctypes.byref(rect))
w, h = rect.right - rect.left, rect.bottom - rect.top
print("窗口位置: (%d,%d) 大小: %dx%d" % (rect.left, rect.top, w, h))

from PIL import ImageGrab
img = ImageGrab.grab(bbox=(rect.left, rect.top, rect.right, rect.bottom))
out = r"D:\xi-system\wechat-window.png"
img.save(out)
print("已保存:", out)
