//! Gateway Pipeline 配置与 middleware 声明。
//!
//! 本模块收集 Gateway 特有的 middleware 声明（请求/响应/错误阶段），
//! 按 phase 分组后生成 `kernel::Pipeline` 实例供执行。
//! 执行权完全委托给 `crate::kernel::Pipeline`，Gateway 本身不维护
//! 并行的 pipeline 抽象，此模块的数据结构仅充当配置快照。
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

