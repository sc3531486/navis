//! 工具运行时实现

pub enum AgentToolEvent { Started, Completed, Failed }
pub enum AgentToolPhase { Pre, Main, Post }
pub enum AgentToolStatus { Pending, Running, Done }
pub struct AgentToolExecution;
