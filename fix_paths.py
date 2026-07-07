import os
import re

root = r"D:\xi-system"

# 找到所有 .rs 文件
files = []
for dirpath, dirnames, filenames in os.walk(root):
    for f in filenames:
        if f.endswith('.rs'):
            files.append(os.path.join(dirpath, f))

print(f"Processing {len(files)} files...")

for filepath in files:
    try:
        # 用 UTF-8 读取
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # 替换路径（避免多重转义）
        content = content.replace('D:\\\\\\\\xi-system', 'D:\\\\xi-system')
        content = content.replace('D:\\\\\\xi-system', 'D:\\\\xi-system')
        content = content.replace('D:\\\\xi-system', 'D:\\\\xi-system')
        
        # 用 UTF-8 无 BOM 保存
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        
        print(f"OK: {os.path.basename(filepath)}")
    except Exception as e:
        print(f"ERROR: {filepath} - {e}")

print("Done!")
