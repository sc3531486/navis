import os, re

base = r"D:\myworkspace\Navis Go\src-tauri\src\domains"

for root, dirs, files in os.walk(base):
    if "mod.rs" not in files:
        continue
    mod_path = os.path.join(root, f)
    with open(mod_path, "r", encoding="utf-8") as fh:
        content = fh.read()
    
    # 找出所有 pub mod 声明
    mods = re.findall(r'pub mod (\w+);', content)
    
    # 检查哪些模块实际存在
    existing = []
    for mod_name in mods:
        if os.path.exists(os.path.join(root, mod_name + ".rs")):
            existing.append(mod_name)
        elif os.path.exists(os.path.join(root, mod_name, "mod.rs")):
            existing.append(mod_name)
    
    # 重建 mod.rs
    rel = os.path.relpath(root, base)
    if not existing and not any(f.endswith(".rs") for f in files if f != "mod.rs"):
        # 没有子模块也没有 .rs 文件 - 跳过
        continue
    
    # 读取原始注释
    lines = content.split("\n")
    comment_lines = [l for l in lines if l.startswith("//!")]
    
    new_content = "\n".join(comment_lines) + "\n\n"
    for mod_name in existing:
        new_content += f"pub mod {mod_name};\n"
    
    with open(mod_path, "w", encoding="utf-8", newline="\n") as fh:
        fh.write(new_content)
    
    removed_count = len(mods) - len(existing)
    if removed_count > 0:
        print(f"{rel}/mod.rs: {len(mods)} -> {len(existing)} modules (removed {removed_count})")

print("Done")
