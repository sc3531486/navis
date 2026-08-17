import os, shutil

base = r"D:\myworkspace\Navis Go\src-tauri\src\domains"
removed = 0

for root, dirs, files in os.walk(base, topdown=False):
    for f in files:
        if f != "mod.rs":
            continue
        path = os.path.join(root, f)
        size = os.path.getsize(path)
        if size < 200:
            # 这是一个桩 mod.rs - 删除它所在的目录
            dir_path = root
            # 不删除顶层 domains/ 下的直接子目录 mod.rs
            rel = os.path.relpath(dir_path, base)
            parts = rel.split(os.sep)
            if len(parts) > 1:
                shutil.rmtree(dir_path)
                removed += 1
                print(f"Removed: {rel}")

print(f"\nRemoved {removed} stub directories")
