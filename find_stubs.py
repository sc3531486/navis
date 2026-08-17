import os

base = r"D:\myworkspace\Navis Go\src-tauri\src\domains"
stubs = []
real = []

for root, dirs, files in os.walk(base):
    for f in files:
        if not f.endswith(".rs"):
            continue
        path = os.path.join(root, f)
        size = os.path.getsize(path)
        rel = os.path.relpath(path, base)
        if size < 200:
            stubs.append((rel, size))
        else:
            real.append((rel, size))

print(f"Real files: {len(real)}")
print(f"Stub files: {len(stubs)}")
print(f"\nStubs (to remove):")
for s, sz in sorted(stubs):
    print(f"  {s} ({sz} bytes)")
