//! Git 差异查看模块
//!
//! 基于设计文档 §21 实现 Git 差异查看：
//! - `git diff` / `git diff --staged` 输出格式化
//! - 单文件 diff 查看
//! - 统一 diff 格式解析
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

