import os

path = r"D:\myworkspace\Navis Go\src-tauri\src\extension\types.rs"
with open(path, "r", encoding="utf-8") as f:
    content = f.read()

# Fix PathManager::resolve to accept &Path
content = content.replace(
    "pub fn resolve(base: &str, path: &std::path::Path) -> String",
    "pub fn resolve(base: &str, path: &std::path::Path) -> String"
)

# Fix ModelConfig context_window: Option<u32>
# The code assigns declaration.context_window (u32) to model.context_window (Option<u32>)
# This should work - u32 auto-wraps to Some(u32) via From trait... no it doesnt
# The code is: model.context_window = declaration.context_window;
# declaration.context_window is u32, model.context_window is Option<u32>
# We need to wrap it: model.context_window = Some(declaration.context_window);

# Fix CapabilitySet.supports_tools: bool -> Vec<String> or vice versa
# The code does: tools: capabilities.supports_tools
# capabilities.supports_tools is bool but tools expects Vec<String>
# Change tools to bool
content = content.replace("pub tools: bool,", "pub supports_tools: bool,")

# Fix ToolDefinitionOverride.user_visible: Option<bool>  
# The code does: user_visible: override_.user_visible
# override_.user_visible is Option<bool> but user_visible expects bool
# Keep as Option<bool> and unwrap_or(true)

with open(path, "w", encoding="utf-8", newline="\n") as f:
    f.write(content)
print("Fixed types.rs")

