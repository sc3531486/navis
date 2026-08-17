import os, re

base = r"D:\myworkspace\Navis Go\src-tauri\src\domains"

# 替换规则：domains/ 内部文件的旧路径 → 新路径
replacements = [
    # agent_core 内部
    ("crate::ai::agent", "crate::domains::agent_core::agent"),
    ("crate::ai::context", "crate::domains::agent_core::context"),
    ("crate::tool::agent", "crate::domains::agent_core::tool_runtime"),
    ("crate::application::runtime", "crate::domains::agent_core::runtime"),
    ("crate::application", "crate::domains::agent_core::application"),
    # ai_platform 内部
    ("crate::ai::gateway", "crate::domains::ai_platform::gateway"),
    ("crate::tool::mcp", "crate::domains::ai_platform::mcp"),
    ("crate::tool::lsp", "crate::domains::ai_platform::lsp"),
    # terminal
    ("crate::tool::terminal", "crate::domains::terminal::terminal"),
    # editor
    ("crate::tool::file", "crate::domains::editor::file"),
    ("crate::tool::git", "crate::domains::editor::git"),
    ("crate::tool::clipboard", "crate::domains::editor::clipboard"),
    ("crate::tool::backend", "crate::domains::editor::backend"),
    # session
    ("crate::project::session", "crate::domains::session::session"),
    # project
    ("crate::project::catalog", "crate::domains::project::catalog"),
    ("crate::project::knowledge", "crate::domains::project::knowledge"),
    # memory
    ("crate::project::memory", "crate::domains::memory::project_memory"),
    ("crate::tool::memory", "crate::domains::memory::tool_memory"),
]

total = 0
files_changed = 0
for root, dirs, files in os.walk(base):
    for fname in files:
        if not fname.endswith(".rs"):
            continue
        fpath = os.path.join(root, fname)
        with open(fpath, "r", encoding="utf-8") as f:
            content = f.read()
        original = content
        for old, new in replacements:
            content = content.replace(old, new)
        if content != original:
            with open(fpath, "w", encoding="utf-8", newline="\n") as f:
                f.write(content)
            changes = sum(original.count(old) for old, new in replacements)
            total += changes
            files_changed += 1
            print(f"Updated: {os.path.relpath(fpath, base)}")

print(f"\nTotal: {files_changed} files, {total} replacements")
