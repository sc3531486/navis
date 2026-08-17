import os, re

base = r"D:\myworkspace\Navis Go\src-tauri\src"
fixed = 0

for root, dirs, files in os.walk(base):
    for f in files:
        if not f.endswith(".rs"):
            continue
        path = os.path.join(root, f)
        with open(path, "r", encoding="utf-8") as fh:
            content = fh.read()
        
        lines = content.split("\n")
        doc_lines = []
        code_lines = []
        in_doc = False
        
        for line in lines:
            stripped = line.strip()
            if stripped.startswith("//!") or stripped.startswith("/*!"):
                doc_lines.append(line)
                in_doc = True
            elif stripped.startswith("///") or stripped.startswith("/**"):
                doc_lines.append(line)
                in_doc = True
            elif in_doc and (stripped == "" or stripped.startswith("//")):
                doc_lines.append(line)
            else:
                in_doc = False
                code_lines.append(line)
        
        if doc_lines and code_lines:
            # Check if doc comments are after code (which is invalid)
            first_code_idx = 0
            for i, line in enumerate(code_lines):
                if line.strip() and not line.strip().startswith("//"):
                    first_code_idx = i
                    break
            
            # Check if there are doc comments after the first code line
            has_doc_after_code = False
            for i, line in enumerate(code_lines):
                if line.strip().startswith("//!") or line.strip().startswith("///"):
                    has_doc_after_code = True
                    break
            
            if has_doc_after_code:
                # Rebuild: doc comments at top, then code
                new_content = "\n".join(doc_lines) + "\n" + "\n".join(code_lines)
                with open(path, "w", encoding="utf-8") as fh:
                    fh.write(new_content)
                fixed += 1
                print(f"Fixed: {os.path.relpath(path, base)}")

print(f"\nFixed {fixed} files")
