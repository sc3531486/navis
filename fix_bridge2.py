path = r"D:\myworkspace\Navis Go\src-tauri\src\ui\extension_bridge.rs"
with open(path, "r", encoding="utf-8") as f:
    lines = f.readlines()

# 找到 ui_extension_bridge_invoke 函数签名并添加 mcp 参数
for i, line in enumerate(lines):
    if "pub async fn ui_extension_bridge_invoke" in line:
        # 找到下一行 lifecycle 参数
        for j in range(i, min(i+10, len(lines))):
            if "lifecycle" in lines[j] and "Arc<ExtensionLifecycle>" in lines[j]:
                # 在 lifecycle 之后添加 mcp 参数
                lines.insert(j+1, "    mcp: State<'_, Arc<MCP>>,\n")
                break
        break

with open(path, "w", encoding="utf-8", newline="\n") as f:
    f.writelines(lines)
print("Fixed ui_extension_bridge_invoke")
