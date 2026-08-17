import os, re

base = r"D:\myworkspace\Navis Go\src-tauri\src\domains"
results = []

for root, dirs, files in os.walk(base):
    for f in files:
        if not f.endswith(".rs"):
            continue
        path = os.path.join(root, f)
        with open(path, "r", encoding="utf-8") as fh:
            content = fh.read()
        
        # 检查是否有 struct/enum/fn/impl 定义
        has_struct = bool(re.search(r'pub struct \w+', content))
        has_fn = bool(re.search(r'pub fn \w+', content))
        has_impl = bool(re.search(r'impl \w+', content))
        has_pub_mod = bool(re.search(r'pub mod \w+', content))
        
        rel = os.path.relpath(path, base)
        lines = content.count('\n') + 1
        
        if has_struct or has_fn or has_impl:
            results.append(f"IMPLEMENTED: {rel} ({lines} lines)")
        elif has_pub_mod:
            results.append(f"STUB: {rel} ({lines} lines)")

print(f"Total files: {len(results)}")
for r in sorted(results):
    print(f"  {r}")
