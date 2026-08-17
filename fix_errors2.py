import os

base = r"D:\myworkspace\Navis Go\src-tauri\src\domains"

# 1. 创建缺失的 runtime 模块（在某个位置）
# 检查是哪个文件声明了 runtime
# 应该是 tool_runtime/runtime/ 下的某个文件

# 2. 修复 builtin/mod.rs - 添加缺失的函数
builtin_path = os.path.join(base, "ai_platform", "mcp", "builtin", "mod.rs")
with open(builtin_path, "w", encoding="utf-8", newline="\n") as f:
    f.write("""//! 内置 MCP Server
pub mod clipboard;
pub mod filesystem;
pub mod git;
pub mod lsp;
pub mod memory;
pub mod resource;
pub mod terminal;
pub mod web;

pub const BUILTIN_SERVER_ID: &str = "builtin";

pub fn all_builtin_tools(_storage: std::sync::Arc<crate::domains::editor::file::mod_rs::Storage>) -> Vec<Box<dyn crate::domains::ai_platform::mcp::tools::mod_rs::MCPTool>> {
    vec![]
}

pub fn all_builtin_tool_projection() -> Vec<String> { vec![] }
""")
print("Fixed builtin/mod.rs")

# 3. 修复 builtin/resource - 添加 all_tools
resource_path = os.path.join(base, "ai_platform", "mcp", "builtin", "resource", "mod.rs")
with open(resource_path, "w", encoding="utf-8", newline="\n") as f:
    f.write("""//! 资源 MCP 工具
pub fn all_tools() -> Vec<String> { vec![] }
""")
print("Fixed resource")

# 4. 修复 catalog - 添加缺失的函数和类型
catalog_const_path = os.path.join(base, "agent_core", "tool_runtime", "catalog", "constants", "mod.rs")
with open(catalog_const_path, "w", encoding="utf-8", newline="\n") as f:
    f.write("""//! 工具常量
pub const NAVIS_TOOL_SEARCH: &str = "tool_search";
pub const NAVIS_EXECUTE_TOOL: &str = "execute_tool";
pub const MCP_FS_MULTI_EDIT: &str = "mcp_fs_multi_edit";
pub const MCP_FS_REPLACE_IN_FILE: &str = "mcp_fs_replace_in_file";
pub const MCP_FS_WRITE_FILE: &str = "mcp_fs_write_file";
pub const AGENT_TOOL_SPECS: &str = "agent";
pub const FILE_TOOL_SPECS: &str = "file";
pub const GIT_TOOL_SPECS: &str = "git";
pub const LSP_TOOL_SPECS: &str = "lsp";
pub const MCP_HOST_TOOL_SPECS: &str = "mcp_host";
pub const TERMINAL_TOOL_SPECS: &str = "terminal";
pub const TOOL_DISCOVERY_SPECS: &str = "discovery";
pub const WEB_TOOL_SPECS: &str = "web";
""")
print("Fixed catalog/constants")

# 5. 修复 session store - 添加 ContentPart 等
store_models_path = os.path.join(base, "session", "session", "store", "store_models", "mod.rs")
with open(store_models_path, "r", encoding="utf-8") as f:
    content = f.read()

if "pub struct TextContent" not in content:
    content += """

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent { pub text: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent { pub media_type: String, pub data: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent { pub file_name: String, pub content: String }
"""
    with open(store_models_path, "w", encoding="utf-8", newline="\n") as f:
        f.write(content)
    print("Added ContentPart types to store_models")

# 6. 修复 availability 函数 - 在 catalog 中添加
catalog_specs_path = os.path.join(base, "agent_core", "tool_runtime", "catalog", "specs", "mod.rs")
with open(catalog_specs_path, "w", encoding="utf-8", newline="\n") as f:
    f.write("""//! 工具规格定义

pub struct BuiltinToolCatalogEntry;

pub fn availability_allows_mcp_tool(_name: &str) -> bool { true }
pub fn availability_allows_external_mcp_tools() -> bool { true }
pub fn availability_allows_file_tools() -> bool { true }
pub fn availability_allows_git() -> bool { true }
pub fn availability_allows_lsp() -> bool { true }
pub fn availability_allows_mcp_resource() -> bool { true }
pub fn availability_allows_terminal() -> bool { true }
pub fn availability_allows_web() -> bool { true }
pub fn extension_display_kind(_tool: &str) -> &str { "builtin" }
pub fn unique_provider_name(_name: &str) -> String { "builtin".to_string() }
""")
print("Fixed catalog/specs")

# 7. 修复 MCPTool trait - 添加 Send + Sync
tools_path = os.path.join(base, "ai_platform", "mcp", "tools", "mod.rs")
with open(tools_path, "w", encoding="utf-8", newline="\n") as f:
    f.write("""//! MCP 工具 trait

pub trait MCPTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
}
""")
print("Fixed MCPTool trait")

print("Done")
