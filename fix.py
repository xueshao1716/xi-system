import os

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
        with open(filepath, 'r', encoding='utf-8', errors='replace') as f:
            content = f.read()
        
        # 只用 Python repr 显示的多重转义
        # D:\\\\\\\\xi-system -> D:\\\xi-system
        # 实际替换：双反斜杠 + 斜杠的组合
        
        # 标准路径
        content = content.replace('D:\\xi-system', 'D:\\xi-system')
        
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        
        print(f"OK: {os.path.basename(filepath)}")
    except Exception as e:
        print(f"ERROR: {filepath} - {e}")

print("Done!")
