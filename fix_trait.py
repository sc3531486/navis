# 在 types.rs 中为 MCP 添加 McpOperationPort 实现
path = r"D:\myworkspace\Navis Go\src-tauri\src\extension\types.rs"
with open(path, "r", encoding="utf-8") as f:
    content = f.read()

# 在 MCP impl 块之后添加 McpOperationPort 实现
impl_block = """impl McpOperationPort for MCP {
    fn sandbox(&self) -> std::sync::Arc<crate::security::sandbox::Sandbox> {
        std::sync::Arc::new(crate::security::sandbox::Sandbox::new())
    }
}"""

# 在 MCP impl 块之后插入
content = content.replace(
    "    pub fn remove_tool_override(&self, _: &str, _: &str, _: &str) -> Result<(), String> { Ok(()) }\n}",
    "    pub fn remove_tool_override(&self, _: &str, _: &str, _: &str) -> Result<(), String> { Ok(()) }\n}\n\nimpl McpOperationPort for MCP {\n    fn sandbox(&self) -> std::sync::Arc<crate::security::sandbox::Sandbox> {\n        std::sync::Arc::new(crate::security::sandbox::Sandbox::new())\n    }\n}"
)

with open(path, "w", encoding="utf-8", newline="\n") as f:
    f.write(content)
print("Added McpOperationPort impl for MCP")
