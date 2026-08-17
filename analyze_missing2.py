import os, re

base = r"D:\myworkspace\Navis Go\src-tauri\src\domains"
missing = []

for root, dirs, files in os.walk(base):
    for f in files:
        if f != "mod.rs":
            continue
        mod_path = os.path.join(root, f)
        with open(mod_path, "r", encoding="utf-8") as fh:
            content = fh.read()
        
        # 提取所有 mod 声明 (pub mod, pub(crate) mod, mod)
        mods = re.findall(r'(?:pub(?:\(crate\))?\s+)?mod\s+(\w+)\s*;', content)
        for mod_name in mods:
            expected_file = os.path.join(root, mod_name + ".rs")
            expected_dir = os.path.join(root, mod_name, "mod.rs")
            if not os.path.exists(expected_file) and not os.path.exists(expected_dir):
                rel = os.path.relpath(root, base)
                missing.append(f"{rel}/{mod_name}")

print(f"Missing modules: {len(missing)}")
for m in sorted(missing):
    print(f"  {m}")
