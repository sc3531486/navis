import os

base = r"D:\myworkspace\Navis Go\src-tauri\src\domains"
total_lines = 0
total_size = 0
files_by_size = []

for root, dirs, files in os.walk(base):
    for f in files:
        if not f.endswith(".rs"):
            continue
        path = os.path.join(root, f)
        size = os.path.getsize(path)
        with open(path, "r", encoding="utf-8") as fh:
            lines = len(fh.readlines())
        total_lines += lines
        total_size += size
        rel = os.path.relpath(path, base)
        files_by_size.append((rel, size, lines))

files_by_size.sort(key=lambda x: -x[1])
print(f"Total: {len(files_by_size)} files, {total_lines} lines, {total_size} bytes")
print(f"\nTop 20 largest files:")
for rel, size, lines in files_by_size[:20]:
    print(f"  {rel}: {lines} lines, {size} bytes")
print(f"\nBottom 10 smallest files:")
for rel, size, lines in files_by_size[-10:]:
    print(f"  {rel}: {lines} lines, {size} bytes")
