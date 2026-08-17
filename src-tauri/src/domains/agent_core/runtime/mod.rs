//! 应用运行时 port。
//!
//! `agent_control` 定义 Agent 控制工具和运行时实现之间的最小合同。具体
//! 的 Task/Todo 存储、Session 服务和 UI 投影都应在外层适配，不进入这些
//! 合同。

pub mod agent_control;
