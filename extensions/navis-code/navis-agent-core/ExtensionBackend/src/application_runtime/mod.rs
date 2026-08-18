//! Agent 应用运行时扩展端口。
//! 该模块只定义扩展内部的最小控制对象，宿主通过能力端口与其通信。

#[derive(Debug, Default)]
pub struct AgentControlPorts;

#[derive(Debug, Default)]
pub struct TodoPort;
