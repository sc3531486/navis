//! Sandbox 安全沙箱模块
//!
//! 基于设计文档 §06 实现，提供访问控制、命令黑白名单、worktree 信任、
//! 资源限制、审计能力等安全管控能力。
//!
//! # 子模块
//! - `gate` - 权限门禁（统一入口，Sandbox 结构体定义在此）
//! - `access_control` - 路径访问控制（白名单/黑名单）
//! - `command_rules` - 命令规则引擎（黑白名单、正则匹配）
//! - `worktree_trust` - worktree 信任管理
//! - `resource_limit` - 资源限制（CPU/内存/时间）
//! - `permission` - 操作权限分级与审批模式
//! - `audit_view` - Sandbox 近期审计视图缓存（结构化事实源在 kernel audit）
//! - `policy` - 策略配置（网络策略、命令规则配置）
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

pub mod access_control;
pub mod audit_view;
pub mod command_rules;
pub mod constraint;
pub mod gate;
pub mod permission;
pub mod policy;
pub mod resource_limit;
pub mod worktree_trust;

// 重导出核心类型
pub use access_control::AccessControl;
pub use audit_view::SandboxAuditView;
pub use audit_view::{SandboxAuditViewAction, SandboxAuditViewEntry, SandboxAuditViewFilter};
pub use command_rules::CommandRules;
pub use command_rules::{CommandRule, CommandShell, RuleAction};
pub use gate::Sandbox;
pub use permission::{ApprovalMode, CheckResult, OperationRequest, OperationType, PermissionLevel};
pub use policy::{default_command_rules, domain_matches, NetworkPolicy, SandboxPolicy};
pub use resource_limit::ResourceLimitManager;
pub use resource_limit::{ResourceLimit, ResourceType, ResourceUsage};
pub use worktree_trust::TrustLevel;
pub use worktree_trust::WorktreeTrustManager;
