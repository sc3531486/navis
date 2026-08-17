//! Context Manager 上下文管理模块（扩展点）
//!
//! 从 src-tauri/src/ai/context/ 迁入，负责组装发送给 LLM 的完整上下文，
//! 包括系统提示词、角色设定、项目配置、历史消息、RAG 检索结果、
//! 可用工具列表，并管理 Token 窗口裁剪和上下文压缩。
//!
//! # 迁移来源
//!
//! src-tauri/src/ai/context/ 下所有文件：
//!   mod.rs, assembler.rs, model_adapter.rs, token_counter.rs, trimmer.rs
//!   assembler/ (compression_boundary, compression_render, compression_template, runtime, summary)
//!
//! # re-export 桥
//!
//! Phase 0 阶段，所有类型从原始模块重导出。

pub use crate::ai::context::assembler;
pub use crate::ai::context::model_adapter;
pub use crate::ai::context::token_counter;
pub use crate::ai::context::trimmer;

// 重导出核心类型（保持原 context/mod.rs 的公开 API）
pub use crate::ai::context::{
    AssembledContext, CompressOptions, CompressTrigger, ContextConfig, ContextFormat, ContextManager,
    FormattedContext, MessageRole, ModelContextProfile, TokenizerType,
};
