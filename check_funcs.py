path = r"D:\myworkspace\Navis Go\src-tauri\src\ui\extension_bridge.rs"
with open(path, "r", encoding="utf-8") as f:
    lines = f.readlines()

# 找到所有 fn bridge_ 函数
in_func = False
func_name = ""
func_start = 0
for i, line in enumerate(lines):
    if "fn bridge_" in line and "async" not in line and "pub" not in line:
        in_func = True
        func_name = line.strip().split("(")[0].replace("fn ", "")
        func_start = i
    elif in_func and ("fn bridge_" in line or line.strip().startswith("pub fn") or line.strip().startswith("pub async fn")):
        in_func = False

# 检查 bridge_storage_get 签名
for i, line in enumerate(lines):
    if "fn bridge_storage_get" in line:
        print(f"bridge_storage_get at line {i+1}:")
        for j in range(i, min(i+10, len(lines))):
            print(f"  {j+1}: {lines[j].rstrip()}")
        break
