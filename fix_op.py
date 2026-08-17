path = r"D:\myworkspace\Navis Go\src-tauri\src\extension\operation_runtime.rs"
with open(path, "r", encoding="utf-8") as f:
    content = f.read()

# 恢复 mcp 参数
content = content.replace(
    "    //     mcp: State<'_, Arc<[REMOVED: MCP reference]",
    "    mcp: State<'_, Arc<MCP>>,"
)

with open(path, "w", encoding="utf-8", newline="\n") as f:
    f.write(content)
print("Fixed operation_runtime.rs")
