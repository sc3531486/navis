import os, re, shutil

base = r"D:\myworkspace\Navis Go\src-tauri\src\domains"
moved = 0

# 扫描所有 mod.rs，找出声明了但文件位置错误的子模块
for root, dirs, files in os.walk(base):
    for f in files:
        if f != "mod.rs":
            continue
        mod_path = os.path.join(root, f)
        with open(mod_path, "r", encoding="utf-8") as fh:
            content = fh.read()
        
        mods = re.findall(r'(?:pub(?:\(crate\))?\s+)?mod\s+(\w+)\s*;', content)
        for mod_name in mods:
            # 检查：文件是否在顶层（错误位置）
            wrong_pos = os.path.join(root, mod_name + ".rs")
            correct_dir = os.path.join(root, mod_name)
            correct_file = os.path.join(correct_dir, "mod.rs")
            
            if os.path.exists(wrong_pos) and not os.path.exists(correct_dir):
                # 需要移动：wrong_pos -> correct_file
                os.makedirs(correct_dir, exist_ok=True)
                shutil.move(wrong_pos, correct_file)
                moved += 1
                rel = os.path.relpath(wrong_pos, base)
                print(f"MOVED: {rel} -> {mod_name}/mod.rs")

print(f"\nMoved {moved} files")
