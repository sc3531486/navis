path = r"D:\myworkspace\Navis Go\src-tauri\src\extension\operation_runtime.rs"
with open(path, "r", encoding="utf-8") as f:
    lines = f.readlines()

# 找到 ui_operation_execute 函数签名并添加 mcp 参数
for i, line in enumerate(lines):
    if "pub async fn ui_operation_execute" in line:
        # 找到 request: OperationExecuteRequest 行
        for j in range(i, min(i+10, len(lines))):
            if "request: OperationExecuteRequest" in lines[j]:
                lines.insert(j, "    mcp: State<'_, Arc<MCP>>,\n")
                break
        break

with open(path, "w", encoding="utf-8", newline="\n") as f:
    f.writelines(lines)
print("Fixed ui_operation_execute")
