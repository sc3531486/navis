import os, re

base = r"D:\myworkspace\Navis Go\src-tauri\src"
fixed = 0

for root, dirs, files in os.walk(base):
    for f in files:
        if not f.endswith(".rs"):
            continue
        path = os.path.join(root, f)
        with open(path, "rb") as fh:
            data = fh.read()
        # Replace \r\n with \n and standalone \r with \n
        clean = data.replace(b"\r\n", b"\n").replace(b"\r", b"\n")
        if clean != data:
            with open(path, "wb") as fh:
                fh.write(clean)
            fixed += 1

print(f"Fixed CRLF in {fixed} files")
