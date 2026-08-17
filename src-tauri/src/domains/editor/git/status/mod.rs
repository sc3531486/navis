//! Git 状态检测模块
//!
//! 基于设计文档 §21 实现 Git 状态检测：
//! - `git status --porcelain=v1 -b` 解析
//! - 变更文件列表提取
//! - 分支信息解析
//! - is_clean 判断
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

