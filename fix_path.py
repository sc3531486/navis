path = r"D:\myworkspace\Navis Go\src-tauri\src\extension\types.rs"
with open(path, "r", encoding="utf-8") as f:
    content = f.read()

# 修复 PathManager - 返回 PathBuf
content = content.replace(
    "pub fn resolve(base: &str, path: &std::path::Path) -> String",
    "pub fn resolve(base: &str, path: &std::path::Path) -> std::path::PathBuf"
)
content = content.replace(
    'format!("{}/{}", base, path.display())',
    'std::path::PathBuf::from(base).join(path)'
)

with open(path, "w", encoding="utf-8", newline="\n") as f:
    f.write(content)
print("Fixed PathManager")
