path = r"D:\myworkspace\Navis Go\src-tauri\src\extension\lifecycle\mod.rs"
with open(path, "r", encoding="utf-8") as f:
    lines = f.readlines()

new_lines = []
i = 0
while i < len(lines):
    line = lines[i]
    if "Some(crate::domains::ai_platform::lsp::LanguageSource::Extension" in line:
        new_lines.append("//             LanguageSource::Extension {\n")
        new_lines.append('//                 owner: "extension-lsp-language".to_string()\n')
        new_lines.append("//             }\n")
        i += 3
        continue
    new_lines.append(line)
    i += 1

with open(path, "w", encoding="utf-8", newline="\n") as f:
    f.writelines(new_lines)
print("Fixed")
