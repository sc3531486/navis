//! Git 凭证管理模块
//!
//! 基于设计文档 §21 实现 Git 凭证管理：
//! - SSH Key 认证（通过 GIT_SSH_COMMAND 环境变量）
//! - Token 认证（通过 git credential helper 注入）
//! - 凭证获取失败时的错误处理
//!
//! # 凭证管理要点
//! - SSH Key 路径由 Auth 模块统一管理，Git 模块不直接存储
//! - Token 认证仅在内存中临时使用，不写入 `.gitconfig`
//! - 凭证获取失败时，操作返回明确错误而非暴露认证细节
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

