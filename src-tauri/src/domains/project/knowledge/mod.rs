//! Knowledge 模块 - 项目知识管理
//!
//! 基于设计文档 §20 实现，提供项目知识管理能力：
//! - 项目配置注入（加载 navis.md）
//! - 文件引用（@file 读取指定文件）
//! - 项目结构摘要（文件树 + 关键文件列表）
//! - 关键词搜索（grep 命令行搜索）
//! - 知识源管理
//!
//! # 依赖
//! - `crate::kernel` - EventBus, EventEnvelope
//!
//! # 被依赖
//! - `crate::domains::agent_core::context` - Context Manager
//! - `crate::domains::agent_core::agent` - Agent 模块
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

pub mod project_knowledge;
