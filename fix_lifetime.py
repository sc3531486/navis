import re

base = r"D:\myworkspace\Navis Go\src-tauri\src"

# 修复 agent_tool_loop.rs
path1 = os.path.join(base, "ui", "runtime", "agent_tool_loop.rs")
with open(path1, "r", encoding="utf-8") as f:
    content = f.read()

# 移除 AgentToolProgressCallback<'_> 中的生命周期
content = content.replace("AgentToolProgressCallback<'_>", "AgentToolProgressCallback")

with open(path1, "w", encoding="utf-8", newline="\n") as f:
    f.write(content)
print("Fixed agent_tool_loop.rs")

# 修复 runtime/mod.rs
path2 = os.path.join(base, "domains", "agent_core", "tool_runtime", "runtime", "mod.rs")
with open(path2, "r", encoding="utf-8") as f:
    content = f.read()

content = content.replace("AgentToolProgressCallback<'_>", "AgentToolProgressCallback")

with open(path2, "w", encoding="utf-8", newline="\n") as f:
    f.write(content)
print("Fixed runtime/mod.rs")
