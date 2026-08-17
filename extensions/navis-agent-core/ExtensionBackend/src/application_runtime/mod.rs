//! 应用运行时 Agent 控制 port（扩展点）
//!
//! 从 src-tauri/src/application/runtime/ 迁入，定义 Agent 控制工具和
//! 运行时实现之间的最小合同。
//!
//! # 迁移来源
//!
//! src-tauri/src/application/runtime/
//!   mod.rs              → port 声明 + re-export
//!   agent_control.rs    → AgentControlPorts, SidechainPort, TodoPort
//!
//! # re-export 桥
//!
//! Phase 0 阶段，所有类型从原始模块重导出。

pub use crate::application::runtime::agent_control;

// 重导出核心 port 类型
pub use crate::application::runtime::{
    AgentControlPorts, AgentToolEventFact, AgentToolEventPort, SidechainPort, SidechainReadRequest,
    SidechainStartRequest, SidechainStarted, SidechainStatus, SidechainTaskSnapshot, TodoPort,
    TodoUpdate, TodoUpdateRequest,
};
