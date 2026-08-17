// ── 归属扩展：navis-agent-core ──
// 迁移目标：extensions/navis-agent-core/ExtensionBackend/src/

//! Agent 运行编排子模块。
//!
//! `agent_tool_loop::run_agent_tool_loop` 的编排将在 D4 迁入
//! `extension::lifecycle::cordis::SERVICE_AGENT_LOOP`（`agentLoop`）capability
//! 服务缝。默认实现 `DefaultAgentLoopPort`（当前为浅占位）定义在 seam 旁
//! （`extension::lifecycle::cordis`），因为 `ui::runtime` 对 `app` 模块私有、
//! 容器组合根无法直达；D4 迁移编排时随 `run_agent_tool_loop` 一并落到本目录。

pub(crate) mod agent_control;
pub(crate) mod agent_tool_loop;
pub(crate) mod session_change_capture;
pub(crate) mod session_message_stream;
pub(crate) mod session_turn;
pub(crate) mod sidechain_task;
pub(crate) mod tool_approval;
pub(crate) mod tool_approval_flow;
