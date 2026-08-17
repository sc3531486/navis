//! 会话断点检查点模块
//!
//! 用于会话恢复（--continue / --resume），保存 Task 执行状态和执行上下文。
//!
//! # 数据模型
//! - `SessionCheckpoint`：检查点记录
//! - `CheckpointType`：检查点类型（自动/手动/Task 中断）
//! - `AgentState`：Agent 执行状态

