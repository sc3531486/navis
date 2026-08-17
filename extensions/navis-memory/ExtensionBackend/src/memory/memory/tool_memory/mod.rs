//! Memory 能力模块
//!
//! 提供 MemoryCapability，实现 kernel::Capability trait，注册到 InMemoryRegistry
//! 用于内核可观测性。同时封装 memory.recall 的语义搜索逻辑，支持 FTS5 全文检索。
//!
//! # 职责边界
//! - MemoryCapability：内核能力注册元数据（id、kind、metadata）
//! - search_similar：基于 FTS5 的语义相似度搜索封装
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

