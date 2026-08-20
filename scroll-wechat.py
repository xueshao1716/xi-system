# -*- coding: utf-8 -*-
"""滚动微信聊天列表并逐屏截图"""
import ctypes, time
from ctypes import wintypes
from PIL import ImageGrab

user32 = ctypes.WinDLL("user32")
HWND = 4197154
user32.ShowWindow(HWND, 9)
user32.SetForegroundWindow(HWND)
time.sleep(1)

class RECT(ctypes.Structure):
    _fields_ = [("left", ctypes.c_long), ("top", ctypes.c_long), ("right", ctypes.c_long), ("bottom", ctypes.c_long)]
rect = RECT()
user32.GetWindowRect(HWND, ctypes.byref(rect))

# 列表区域（左侧，消息列表）
list_x = rect.left + 150
list_y = rect.top + 300
user32.SetCursorPos(list_x, list_y)
time.sleep(0.5)

# 先滚动到顶部
for _ in range(10):
    ctypes.windll.user32.mouse_event(0x0800, 0, 0, -120, 0)  # WHEEL up
    time.sleep(0.1)
time.sleep(0.5)

# 截图第 0 屏
img = ImageGrab.grab(bbox=(rect.left, rect.top, rect.right, rect.bottom))
img.save(r"D:\xi-system\list-00.png")

# 逐屏滚动截图
for i in range(1, 15):
    ctypes.windll.user32.mouse_event(0x0800, 0, 0, -300, 0)  # WHEEL down (向下滚)
    time.sleep(0.6)
    img = ImageGrab.grab(bbox=(rect.left, rect.top, rect.right, rect.bottom))
    img.save(r"D:\xi-system\list-%02d.png" % i)

print("已截 15 屏")
