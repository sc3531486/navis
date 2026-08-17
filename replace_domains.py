import os, re

base = r"D:\myworkspace\Navis Go\src-tauri\src"

# 替换规则
replacements = [
    # MCP 类型 → extension::types
    ("crate::domains::ai_platform::mcp::MCP", "crate::extension::types::McpEngine"),
    # Gateway 类型 → extension::types
    ("crate::domains::ai_platform::gateway::protocol::CustomProtocolConfig", "crate::extension::types::CustomProtocolConfig"),
    ("crate::domains::ai_platform::gateway::request::{ApiProtocol, ProviderConfig}", "crate::extension::types::{ApiProtocol, ProviderConfig}"),
    ("crate::domains::ai_platform::mcp::protocol::{MCPServerConfig, ToolDefinitionOverride}", "crate::extension::types::{MCPServerConfig, ToolDefinitionOverride}"),
    ("crate::domains::ai_platform::gateway::protocol::{CapabilitySet, CustomProtocolConfig}", "crate::extension::types::{CapabilitySet, CustomProtocolConfig}"),
    ("crate::domains::ai_platform::gateway::request::{ApiProtocol, ModelConfig, ProviderConfig}", "crate::extension::types::{ApiProtocol, ModelConfig, ProviderConfig}"),
    ("crate::domains::ai_platform::mcp::protocol::{", "crate::extension::types::{"),
    # LSP 类型 → extension::types
    ("crate::domains::ai_platform::lsp::LanguageSource::Extension", "crate::extension::types::LanguageSource::Extension"),
    ("crate::domains::ai_platform::lsp::LanguageSource::Builtin", "crate::extension::types::LanguageSource::Builtin"),
    ("crate::domains::ai_platform::lsp::LSPManager::new", "crate::extension::types::LspManager::new"),
    # Editor 类型 → extension::types
    ("crate::domains::editor::backend::BackendProcessManager", "crate::extension::types::BackendProcessManager"),
    ("crate::domains::editor::file::path_manager::PathManager::normalize", "crate::extension::types::PathManager::normalize"),
    ("crate::domains::editor::file::path_manager::PathManager::resolve", "crate::extension::types::PathManager::resolve"),
    # Memory 类型 → extension::types
    ("crate::domains::memory::project_memory::MemoryStore", "crate::extension::types::MemoryStore"),
    # Tool runtime → extension::types
    ("crate::domains::agent_core::tool_runtime::NAVIS_TOOL_SEARCH", "crate::extension::types::NAVIS_TOOL_SEARCH"),
    # agent_timeline_part → extension::types
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
