import os

base = r"D:\myworkspace\Navis Go\src-tauri\src\domains"

mods = {
    "agent_core/mod.rs": """//! navis-agent-core 扩展后端 — Agent 引擎 / 上下文 / 工具运行时

pub mod agent;
pub mod context;
pub mod tool_runtime;
pub mod runtime;
""",
    "ai_platform/mod.rs": """//! navis-ai-platform 扩展后端 — Gateway / MCP / LSP

pub mod gateway;
pub mod mcp;
pub mod lsp;
""",
    "terminal/mod.rs": """//! navis-terminal 扩展后端 — 终端 PTY 管理

pub mod terminal;
""",
    "editor/mod.rs": """//! navis-editor 扩展后端 — 文件 / Git / 剪贴板 / 后端进程

pub mod file;
pub mod git;
pub mod clipboard;
pub mod backend;
""",
    "session/mod.rs": """//! navis-session 扩展后端 — 会话 / 消息管理

pub mod session;
""",
    "project/mod.rs": """//! navis-project 扩展后端 — 项目目录 / 知识库

pub mod catalog;
pub mod knowledge;
""",
    "memory/mod.rs": """//! navis-memory 扩展后端 — Agent 记忆 / 项目记忆

pub mod tool_memory;
pub mod project_memory;
""",
}

for rel, content in mods.items():
    path = os.path.join(base, rel)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(content)
    print(f"Updated {rel}")

print("Done")
