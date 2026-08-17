path = r"D:\myworkspace\Navis Go\src-tauri\src\extension\types.rs"
with open(path, "r", encoding="utf-8") as f:
    content = f.read()

# 添加 import
content = "use crate::extension::operation_runtime::McpOperationPort;\n\n" + content

with open(path, "w", encoding="utf-8", newline="\n") as f:
    f.write(content)
print("Added McpOperationPort import")
