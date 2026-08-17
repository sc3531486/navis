path = r"D:\myworkspace\Navis Go\src-tauri\src\ui\extension_bridge.rs"
with open(path, "r", encoding="utf-8") as f:
    content = f.read()

# 修复 bridge_extension_route_call - 添加 mcp 参数
old = "fn bridge_extension_route_call(\n    extension_store: &State<'_, Arc<ExtensionStore>>,\n\n\n    args: &Value,"
new = "fn bridge_extension_route_call(\n    extension_store: &State<'_, Arc<ExtensionStore>>,\n    mcp: &MCP,\n    args: &Value,"
content = content.replace(old, new)

# 修复 bridge_storage_get - 添加 mcp 参数
old2 = "fn bridge_storage_get(\n    extension_store: &State<'_, Arc<ExtensionStore>>,\n    mcp: &MCP,"
# 这个函数已经有 mcp 参数，不需要修改

# 修复 bridge_storage_set - 添加 mcp 参数
old3 = "fn bridge_storage_set(\n    extension_store: &State<'_, Arc<ExtensionStore>>,\n    mcp: &MCP,"
# 这个函数已经有 mcp 参数，不需要修改

# 修复 bridge_storage_delete - 添加 mcp 参数
old4 = "fn bridge_storage_delete(\n    extension_store: &State<'_, Arc<ExtensionStore>>,\n    mcp: &MCP,"
# 这个函数已经有 mcp 参数，不需要修改

# 修复 bridge_storage_clear - 添加 mcp 参数
old5 = "fn bridge_storage_clear(\n    extension_store: &State<'_, Arc<ExtensionStore>>,\n    mcp: &MCP,"
# 这个函数已经有 mcp 参数，不需要修改

with open(path, "w", encoding="utf-8", newline="\n") as f:
    f.write(content)
print("Fixed bridge_extension_route_call")
