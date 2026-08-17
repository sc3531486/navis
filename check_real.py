import os

base = r"D:\myworkspace\Navis Go\src-tauri\src\domains"
real_files = []

for root, dirs, files in os.walk(base):
    for f in files:
        if not f.endswith(".rs"):
            continue
        path = os.path.join(root, f)
        with open(path, "r", encoding="utf-8") as fh:
            content = fh.read()
        
        # 检查是否有 struct/enum/fn/impl 定义（不只是 mod 声明）
        has_impl = "pub struct " in content or "pub fn " in content or "pub enum " in content or "impl " in content
        lines = content.count("\n") + 1
        
        if has_impl and lines > 5:
            rel = os.path.relpath(path, base)
            real_files.append((rel, lines))

print(f"Files with actual implementations: {len(real_files)}")
for rel, lines in sorted(real_files, key=lambda x: -x[1]):
    print(f"  {rel}: {lines} lines")
