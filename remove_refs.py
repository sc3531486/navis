import os, re

base = r"D:\myworkspace\Navis Go\src-tauri\src"
fixed = 0

for root, dirs, files in os.walk(base):
    if "domains" in root or "extensions" in root:
        continue
    for f in files:
        if not f.endswith(".rs"):
            continue
        path = os.path.join(root, f)
        with open(path, "r", encoding="utf-8") as fh:
            content = fh.read()
        original = content
        # 注释掉所有 crate::domains:: 引用
        content = re.sub(r'(.*)crate::domains::[^\n]+', r'// \1[REMOVED: domains reference]', content)
        # 注释掉所有 crate::extension::types::MCP 引用
        content = re.sub(r'(.*)crate::extension::types::MCP[^\n]*', r'// \1[REMOVED: MCP reference]', content)
        if content != original:
            with open(path, "w", encoding="utf-8", newline="\n") as fh:
                fh.write(content)
            fixed += 1
            print(f"Fixed: {os.path.relpath(path, base)}")

print(f"\nFixed {fixed} files")
