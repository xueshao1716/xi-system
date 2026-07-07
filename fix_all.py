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

broke_files = ['src/brain.rs', 'src/dream.rs']

for filepath in files:
    try:
        with open(filepath, 'r', encoding='utf-8', errors='replace') as f:
            content = f.read()
        
        if any(name in filepath for name in broke_files):
            print(f"SKIP: {os.path.basename(filepath)}")
            continue
        
        # 多重转义修复
        content = content.replace('D:\\\\\\\\\\\\\\\\\\\\\\\\\\\\\xi-system', 'D:\\\\\\\\\\\\\\\\\\\\\\\\\xi-system')
        content = content.replace('D:\\\\\\\\\\\\\\\\\\\\\\\\\xi-system', 'D:\\\\\\\\\\\\\\\\\\\\\\\\\xi-system')
        content = content.replace('D:\\\\\\\\\\\\\\\\\\\\\\\xi-system', 'D:\\\\\\\\\\\\\\\\\\\\\\\\\xi-system')
        content = content.replace('D:\\\\\\\\\\\\\\\\\\\\xi-system', 'D:\\\\\\\\\\\\\\\\\\\\\\\\\xi-system')
        content = content.replace('D:\\\\\\\\\\\\\\\\xi-system', 'D:\\\\\\\\\\\\\\\\\\\\\\\\\xi-system')
        content = content.replace('D:\\\\\\\\\\\\\\xi-system', 'D:\\\\\\\\\\\\\\\\\\\\\\\\\xi-system')
        content = content.replace('D:\\\\\\\\\\\\xi-system', 'D:\\\\\\\\\\\\\\\\\\\\\\\\\xi-system')
        content = content.replace('D:\\\\\\\\xi-system', 'D:\\\\\\\\\\\\\\\\\\\\\\\\\xi-system')
        content = content.replace('D:\\\\\\\xi-system', 'D:\\\\\\\\\\\\\\\\\\\\\\\\\xi-system')
        content = content.replace('D:\\\\xi-system', 'D:\\\\\\\\\\\\\\\\\\\\\\\\\xi-system')
        
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        
        print(f"OK: {os.path.basename(filepath)}")
    except Exception as e:
        print(f"ERROR: {filepath} - {e}")

print("Done!")
