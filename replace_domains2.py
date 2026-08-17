import os, re

base = r"D:\myworkspace\Navis Go\src-tauri\src"

# 替换规则
replacements = [
    # MCP
    ("crate::domains::ai_platform::mcp::MCP", "crate::extension::types::MCP"),
    ("crate::domains::ai_platform::mcp::protocol::", "crate::extension::types::"),
    # Gateway
    ("crate::domains::ai_platform::gateway::protocol::", "crate::extension::types::"),
    ("crate::domains::ai_platform::gateway::request::", "crate::extension::types::"),
    ("crate::domains::ai_platform::gateway::", "crate::extension::types::"),
    # LSP
    ("crate::domains::ai_platform::lsp::LSPManager", "crate::extension::types::LspManager"),
    ("crate::domains::ai_platform::lsp::LanguageSource", "crate::extension::types::LanguageSource"),
    # Editor
    ("crate::domains::editor::backend::BackendProcessManager", "crate::extension::types::BackendProcessManager"),
    ("crate::domains::editor::file::path_manager::PathManager", "crate::extension::types::PathManager"),
    # Memory
    ("crate::domains::memory::project_memory::MemoryStore", "crate::extension::types::MemoryStore"),
    # Tool runtime
    ("crate::domains::agent_core::tool_runtime::NAVIS_TOOL_SEARCH", "crate::extension::types::NAVIS_TOOL_SEARCH"),
    # agent_timeline_part
    ("crate::ui::agent_timeline_part::UiAgentTimelinePart", "crate::extension::types::UiAgentTimelinePart"),
]

total = 0
files_changed = 0

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
        for old, new in replacements:
            content = content.replace(old, new)
        if content != original:
            with open(path, "w", encoding="utf-8", newline="\n") as fh:
                fh.write(content)
            changes = sum(original.count(old) for old, new in replacements)
            total += changes
            files_changed += 1
            print(f"Updated: {os.path.relpath(path, base)}")

print(f"\nTotal: {files_changed} files, {total} replacements")
