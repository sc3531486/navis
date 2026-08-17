//! Agent 工具运行时模块（扩展点）
//!
//! 从 src-tauri/src/tool/agent/ 迁入，管理 Agent 工具目录、契约、
//! guardrail、扩展 hook 约束和 Pipeline 入口。
//!
//! # 迁移来源
//!
//! src-tauri/src/tool/agent/ 下所有文件，重命名为 tool_runtime 避免歧义：
//!   catalog.rs, contract.rs, guardrail.rs, hooks.rs, mod.rs,
//!   result_format.rs, result.rs, runtime.rs, special.rs,
//!   text_tool_call.rs, tool_call_utils.rs, write_guard.rs
//!   catalog/ (constants, mode_filter, naming, schemas, specs)
//!   pipeline/ (agent_control, audit, data, emit_events, guardrail, mcp_execution, mod.rs, observe, policy_check, runner, skill)
//!   runtime/ (events, messages, resolver, session_context, tool_search)
//!   special/ (host, response, sidechain, todo)
//!
//! # re-export 桥
//!
//! Phase 0 阶段，所有类型从原始模块重导出。

pub use crate::tool::agent::catalog;
pub use crate::tool::agent::contract;
pub use crate::tool::agent::guardrail;
pub use crate::tool::agent::hooks;
pub use crate::tool::agent::pipeline;
pub use crate::tool::agent::result;
pub use crate::tool::agent::result_format;
pub use crate::tool::agent::runtime;
pub use crate::tool::agent::special;
pub use crate::tool::agent::text_tool_call;
pub use crate::tool::agent::tool_call_utils;
pub use crate::tool::agent::write_guard;

// 重导出核心类型（保持原 tool::agent 的公开 API）
pub use crate::tool::agent::{
    NAVIS_EXECUTE_TOOL, NAVIS_TOOL_SEARCH,
};
pub use crate::tool::agent::{
    AgentExecutionContext, ToolAvailability,
};
pub use crate::tool::agent::{
    ToolHookDecision, ToolHookInput, ToolHookRunner,
};
pub use crate::tool::agent::{
    ToolCallRecord, ToolCallResult, ToolCallStatus,
};
pub use crate::tool::agent::{
    agent_tool_definitions, assistant_tool_message, assistant_tool_message_with_content,
    effective_gateway_tool_call, execute_agent_tool_call_async, is_supported_gateway_tool,
    is_supported_sidechain_gateway_tool, sidechain_agent_tool_definitions, tool_started_event,
    AgentToolEvent, AgentToolExecution, AgentToolPhase, AgentToolProgressCallback, AgentToolStatus,
};
pub use crate::tool::agent::SpecialAgentToolHost;
pub use crate::tool::agent::{parse_text_tool_call, parse_text_tool_calls};
pub use crate::tool::agent::{tool_call_arguments, tool_call_summary};
