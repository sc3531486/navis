//! navis-agent-core 扩展后端入口。
//! 业务实现只从当前扩展目录装配，不反向依赖 Navis 宿主业务模块。

pub mod agent;
pub mod application_runtime;
pub mod context;
pub mod tool_runtime;
