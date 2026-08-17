import os
base = r"D:\myworkspace\Navis Go\src-tauri\src"
for p in [
    os.path.join(base, "ui", "runtime", "agent_tool_loop.rs"),
    os.path.join(base, "domains", "agent_core", "tool_runtime", "runtime", "mod.rs"),
]:
    with open(p, "r", encoding="utf-8") as f:
        c = f.read()
    c = c.replace("AgentToolProgressCallback<'_>", "AgentToolProgressCallback")
    with open(p, "w", encoding="utf-8", newline="\n") as f:
        f.write(c)
    print(f"Fixed {os.path.basename(p)}")
