import os, re

base = r"D:\myworkspace\Navis Go\src-tauri\src"
needed_types = {}

for root, dirs, files in os.walk(base):
    if "domains" in root:
        continue
    for f in files:
        if not f.endswith(".rs"):
            continue
        path = os.path.join(root, f)
        with open(path, "r", encoding="utf-8") as fh:
            for i, line in enumerate(fh, 1):
                # 查找 use crate::domains::* 导入
                m = re.search(r'use crate::domains::(\w+::\w+::\w+)', line)
                if m:
                    full_path = m.group(1)
                    parts = full_path.split("::")
                    if len(parts) >= 3:
                        key = f"{parts[0]}::{parts[1]}::{parts[2]}"
                        if key not in needed_types:
                            needed_types[key] = []
                        needed_types[key].append(f"{os.path.relpath(path, base)}:{i}")

print(f"Needed type paths: {len(needed_types)}")
for key in sorted(needed_types.keys()):
    count = len(needed_types[key])
    print(f"  {key} ({count} refs)")
