import os, glob

base = r"D:\myworkspace\Navis Go\src-tauri\src"

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
    ("use crate::application::runtime", "use crate::domains::agent_core::runtime"),
    ("use crate::application", "use crate::domains::agent_core::application"),
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
