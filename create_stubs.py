import os

base = r"D:\myworkspace\Navis Go\src-tauri\src\domains"

stubs = {
    r"agent_core\agent\self_evolution": """//! Agent 自我进化模块
//! TODO: 待实现 — 经验捕获、模式提取、反思分析、策略优化
""",
    r"agent_core\tool_runtime\pipeline": """//! 工具运行管线
//! TODO: 待实现 — 工具执行管线、审批、审计
""",
    r"ai_platform\gateway\protocol": """//! Gateway 协议适配器
//! TODO: 待实现 — Chat Completions / Responses 协议适配
pub mod adapter_trait;
pub mod capability;
pub mod chat_completions;
pub mod custom;
pub mod registry;
pub mod responses;
pub mod transformer;
""",
    r"ai_platform\gateway\provider": """//! Gateway Provider 目录
//! TODO: 待实现 — Provider 配置与模型目录
pub mod profile;
""",
    r"ai_platform\mcp\builtin": """//! 内置 MCP Server
//! TODO: 待实现 — filesystem, terminal, git, lsp, clipboard, memory 内置工具
pub mod clipboard;
pub mod filesystem;
pub mod git;
pub mod lsp;
pub mod memory;
pub mod mod_rs;
pub mod resource;
pub mod terminal;
pub mod web;
""",
    r"ai_platform\mcp\tools": """//! MCP 工具定义
//! TODO: 待实现 — MCPTool trait 和工具注册
""",
    r"ai_platform\mcp\transport": """//! MCP 传输层
//! TODO: 待实现 — stdio, SSE, WebSocket 传输适配
pub mod adapter_trait;
pub mod stdio;
""",
}

created = 0
for rel, content in stubs.items():
    path = os.path.join(base, rel)
    os.makedirs(path, exist_ok=True)
    mod_path = os.path.join(path, "mod.rs")
    with open(mod_path, "w", encoding="utf-8", newline="\n") as f:
        f.write(content)
    created += 1
    print(f"Created: {rel}/mod.rs")

# Also check for sub-modules that are declared in these stubs
# and create them too
sub_stubs = {
    r"ai_platform\gateway\protocol\adapter_trait": "//! 协议适配器 trait\n",
    r"ai_platform\gateway\protocol\capability": "//! 协议能力集\n",
    r"ai_platform\gateway\protocol\chat_completions": "//! Chat Completions 协议\n",
    r"ai_platform\gateway\protocol\custom": "//! 自定义协议\n",
    r"ai_platform\gateway\protocol\registry": "//! 协议注册表\n",
    r"ai_platform\gateway\protocol\responses": "//! Responses 协议\n",
    r"ai_platform\gateway\protocol\transformer": "//! 协议转换器\n",
    r"ai_platform\gateway\provider\profile": "//! Provider 配置\n",
    r"ai_platform\mcp\builtin\clipboard": "//! 剪贴板 MCP 工具\n",
    r"ai_platform\mcp\builtin\filesystem": "//! 文件系统 MCP 工具\n",
    r"ai_platform\mcp\builtin\git": "//! Git MCP 工具\n",
    r"ai_platform\mcp\builtin\lsp": "//! LSP MCP 工具\n",
    r"ai_platform\mcp\builtin\memory": "//! 记忆 MCP 工具\n",
    r"ai_platform\mcp\builtin\mod_rs": "//! 内置 MCP 模块\n",
    r"ai_platform\mcp\builtin\resource": "//! 资源 MCP 工具\n",
    r"ai_platform\mcp\builtin\terminal": "//! 终端 MCP 工具\n",
    r"ai_platform\mcp\builtin\web": "//! Web MCP 工具\n",
    r"ai_platform\mcp\tools\mod_rs": "//! MCP 工具 trait\npub trait MCPTool { fn name(&self) -> &str; }\n",
    r"ai_platform\mcp\transport\adapter_trait": "//! 传输适配器 trait\n",
    r"ai_platform\mcp\transport\stdio": "//! stdio 传输\n",
}

for rel, content in sub_stubs.items():
    path = os.path.join(base, rel + ".rs")
    if not os.path.exists(path):
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            f.write(content)
        created += 1

print(f"\nTotal created: {created} files")
