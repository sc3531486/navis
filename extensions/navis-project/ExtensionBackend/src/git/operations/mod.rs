//! Git 操作封装模块
//!
//! 基于设计文档 §21 实现 Git 操作封装：
//! - commit（提交）
//! - stage / unstage / stageAll（暂存管理）
//! - push / pull（远程操作，通过 Terminal MCP 工具执行）
//! - merge（合并）
//!
//! # 凭证注入
//! push / pull 等涉及远端操作的命令，执行前自动获取凭证。
//! 凭证通过环境变量注入到 git 命令进程中。
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

