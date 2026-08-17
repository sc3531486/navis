import re

path = r"D:\myworkspace\Navis Go\src-tauri\src\ui\extension_bridge.rs"
with open(path, "r", encoding="utf-8") as f:
    content = f.read()

# 修复 bridge_network_fetch - 添加 mcp 参数
content = content.replace(
    "async fn bridge_network_fetch(\n      extension_store: &State<'_, Arc<ExtensionStore>>,\n\n\n      args: &Value,",
    "async fn bridge_network_fetch(\n      extension_store: &State<'_, Arc<ExtensionStore>>,\n      mcp: &MCP,\n      args: &Value,"
)

# 修复 bridge_operation_execute - 添加 mcp 参数
content = content.replace(
    "async fn bridge_operation_execute(\n      extension_id: &str,\n      extension_store: &State<'_, Arc<ExtensionStore>>,\n      operation_store: &State<'_, Arc<OperationRegistry>>,\n      args: &Value,",
    "async fn bridge_operation_execute(\n      extension_id: &str,\n      extension_store: &State<'_, Arc<ExtensionStore>>,\n      operation_store: &State<'_, Arc<OperationRegistry>>,\n      mcp: &MCP,\n      args: &Value,"
)

with open(path, "w", encoding="utf-8", newline="\n") as f:
    f.write(content)
print("Fixed function signatures")
