import os
import re

root = r"D:\xi-system"

files = []
for dirpath, dirnames, filenames in os.walk(root):
    if 'target' in dirpath or '.git' in dirpath:
        continue
    for f in filenames:
        if f.endswith('.rs'):
            files.append(os.path.join(dirpath, f))

print(f"Processing {len(files)} files...")

for filepath in files:
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # 替换中文注释为英文（用简单占位符）
        # 检测到乱码就替换
        content = re.sub(r'[\u4e00-\u9fa5\u3400-\u4db5\uFA0E\uFA10-\uFA19\uFA1D-\uFAF8\uFAF9-\uFFBD\uFFC2-\uFFC7\uFFE0-\uFFE5\u0020\u007F-\u009F]+[^\n]*\n', r'\n/// (comment)\n', content)
        
        # 再用标准英文注释填充（暂时）
        content = re.sub(r'/// \(.+?\)\n', r'/// TODO: comment\n', content)
        
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        
        print(f"OK: {os.path.basename(filepath)}")
    except Exception as e:
        print(f"ERROR: {filepath} - {e}")

print("Done!")
