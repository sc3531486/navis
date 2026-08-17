import os

base = r"D:\myworkspace\Navis Go\src-tauri\src\domains\agent_core"

# 创建 application/mod.rs - re-export runtime
app_dir = os.path.join(base, "application")
os.makedirs(app_dir, exist_ok=True)
with open(os.path.join(app_dir, "mod.rs"), "w", encoding="utf-8", newline="\n") as f:
    f.write("//! 应用层合同与用例边界\n\npub mod runtime;\n")

# 更新 agent_core/mod.rs - 确保声明正确
mod_content = """//! navis-agent-core 扩展后端 — Agent 引擎 / 上下文 / 工具运行时

pub mod agent;
pub mod application;
pub mod context;
pub mod runtime;
pub mod tool_runtime;
"""
with open(os.path.join(base, "mod.rs"), "w", encoding="utf-8", newline="\n") as f:
    f.write(mod_content)

print("Fixed application module")
