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
        
        # 恢复 MCP 引用
        content = content.replace("// use [REMOVED: MCP reference]", "use crate::extension::types::MCP;")
        
        # 恢复 domains 引用 - 替换为 extension::types
        content = re.sub(r'// use crate::domains::(\w+)::(\w+)::(\w+); use \[REMOVED: domains reference\]', 
                         r'use crate::extension::types::\3;', content)
        
        # 恢复其他 domains 引用
        content = re.sub(r'// use crate::domains::(\w+)::(\w+)::\{([^}]+)\}; use \[REMOVED: domains reference\]',
                         r'use crate::extension::types::{\3};', content)
        
        if content != original:
            with open(path, "w", encoding="utf-8", newline="\n") as fh:
                fh.write(content)
            fixed += 1
            print(f"Fixed: {os.path.relpath(path, base)}")

print(f"\nFixed {fixed} files")
