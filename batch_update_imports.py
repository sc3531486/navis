import os, re

base = r"D:\myworkspace\Navis Go\src-tauri\src"

# 替换规则：(旧路径, 新路径)
# 注意：不替换 domains/ 目录下的文件（它们是 re-export 源）
# 注意：不替换 project/memory/mod.rs（re-export stub）
replacements = [
    ("use crate::ai::agent", "use crate::domains::agent_core::agent"),
    ("use crate::ai::context", "use crate::domains::agent_core::context"),
    ("use crate::ai::gateway", "use crate::domains::ai_platform::gateway"),
    ("use crate::tool::agent", "use crate::domains::agent_core::tool_runtime"),
    ("use crate::tool::mcp", "use crate::domains::ai_platform::mcp"),
    ("use crate::tool::lsp", "use crate::domains::ai_platform::lsp"),
    ("use crate::tool::terminal", "use crate::domains::terminal::terminal"),
    ("use crate::tool::file", "use crate::domains::editor::file"),
    ("use crate::tool::git", "use crate::domains::editor::git"),
    ("use crate::tool::clipboard", "use crate::domains::editor::clipboard"),
    ("use crate::tool::backend", "use crate::domains::editor::backend"),
    ("use crate::tool::memory", "use crate::domains::memory::tool_memory"),
    ("use crate::project::session", "use crate::domains::session::session"),
    ("use crate::project::catalog", "use crate::domains::project::catalog"),
    ("use crate::project::knowledge", "use crate::domains::project::knowledge"),
    ("use crate::project::memory", "use crate::domains::memory::project_memory"),
    ("use crate::application", "use crate::domains::agent_core::application"),
]

# 排除目录
skip_dirs = {"domains", "extension", "kernel", "foundation", "security"}

total_changed = 0
files_changed = 0

for root, dirs, files in os.walk(base):
    # 跳过排除目录
    rel = os.path.relpath(root, base)
    top_dir = rel.split(os.sep)[0] if os.sep in rel else rel
    if top_dir in skip_dirs:
        continue
    
    for fname in files:
        if not fname.endswith(".rs"):
            continue
        fpath = os.path.join(root, fname)
        
        # 跳过 re-export stub 文件
        if "project\\memory\\mod.rs" in fpath.replace("/", "\\"):
            continue
        
        with open(fpath, "r", encoding="utf-8") as f:
            content = f.read()
        
        original = content
        for old, new in replacements:
            content = content.replace(old, new)
        
        if content != original:
            with open(fpath, "w", encoding="utf-8", newline="\n") as f:
                f.write(content)
            # 统计替换次数
            changes = sum(original.count(old) - content.count(old) for old, new in replacements)
            # 更准确的统计
            for old, new in replacements:
                changes = original.count(old)
                total_changed += changes
            files_changed += 1
            relpath = os.path.relpath(fpath, base)
            print(f"Updated: {relpath}")

print(f"\nTotal: {files_changed} files, ~{total_changed} replacements")
