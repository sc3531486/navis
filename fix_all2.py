import os

base = r"D:\myworkspace\Navis Go\src-tauri\src"

# 更全面的替换 - 包括所有 crate:: 路径引用（不只是 use 语句）
replacements = [
    ("crate::ai::agent", "crate::domains::agent_core::agent"),
    ("crate::ai::context", "crate::domains::agent_core::context"),
    ("crate::ai::gateway", "crate::domains::ai_platform::gateway"),
    ("crate::tool::agent", "crate::domains::agent_core::tool_runtime"),
    ("crate::tool::mcp", "crate::domains::ai_platform::mcp"),
    ("crate::tool::lsp", "crate::domains::ai_platform::lsp"),
    ("crate::tool::terminal", "crate::domains::terminal::terminal"),
    ("crate::tool::file", "crate::domains::editor::file"),
    ("crate::tool::git", "crate::domains::editor::git"),
    ("crate::tool::clipboard", "crate::domains::editor::clipboard"),
    ("crate::tool::backend", "crate::domains::editor::backend"),
    ("crate::tool::memory", "crate::domains::memory::tool_memory"),
    ("crate::project::session", "crate::domains::session::session"),
    ("crate::project::catalog", "crate::domains::project::catalog"),
    ("crate::project::knowledge", "crate::domains::project::knowledge"),
    ("crate::project::memory", "crate::domains::memory::project_memory"),
    ("crate::application::runtime", "crate::domains::agent_core::runtime"),
    ("crate::application", "crate::domains::agent_core::application"),
]

total = 0
files_changed = 0
skip_dirs = {"domains"}

for root, dirs, files in os.walk(base):
    rel = os.path.relpath(root, base)
    top = rel.split(os.sep)[0] if os.sep in rel else rel
    if top in skip_dirs:
        continue
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
