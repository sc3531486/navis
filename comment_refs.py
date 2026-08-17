import os, re

base = r"D:\myworkspace\Navis Go\src-tauri\src"
fixed = 0

for root, dirs, files in os.walk(base):
    for f in files:
        if not f.endswith(".rs"):
            continue
        path = os.path.join(root, f)
        with open(path, "r", encoding="utf-8") as fh:
            lines = fh.readlines()
        
        new_lines = []
        changed = False
        for line in lines:
            if re.search(r'crate::(ai|tool|project|application|domains)::', line):
                new_lines.append("// " + line)
                changed = True
            else:
                new_lines.append(line)
        
        if changed:
            with open(path, "w", encoding="utf-8", newline="\n") as fh:
                fh.writelines(new_lines)
            fixed += 1
            print(f"Fixed: {os.path.relpath(path, base)}")

print(f"\nFixed {fixed} files")
