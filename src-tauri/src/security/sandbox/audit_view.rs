//! Sandbox 近期审计视图缓存
//!
//! 这里保留 Sandbox 领域用于 UI 查询和 DTO 适配的近期审计视图。
//! 结构化审计事实源统一由 `kernel::AuditRecorder` / `AuditSink` 写入，
//! 本模块不负责持久化、聚合或作为权威事实源。
//!
//! # 设计思路
//! - 缓存近期校验操作（允许/拒绝/确认）
//! - 支持按时间/操作类型/worktree等维度过滤近期视图
//! - 使用环形缓冲区限制内存占用

use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::permission::{OperationType, PermissionLevel};

// ============================================================================
// 近期审计视图条目
// ============================================================================

/// Sandbox 审计动作（用于近期视图和 DTO）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxAuditViewAction {
    /// 操作被允许
    Allowed,
    /// 操作被拒绝
    Denied,
    /// 需要用户确认
    ConfirmRequested,
    /// 用户确认通过
    Confirmed,
    /// 用户拒绝确认
    UserDenied,
}

impl std::fmt::Display for SandboxAuditViewAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxAuditViewAction::Allowed => write!(f, "allowed"),
            SandboxAuditViewAction::Denied => write!(f, "denied"),
            SandboxAuditViewAction::ConfirmRequested => write!(f, "confirm_requested"),
            SandboxAuditViewAction::Confirmed => write!(f, "confirmed"),
            SandboxAuditViewAction::UserDenied => write!(f, "user_denied"),
        }
    }
}

/// Sandbox 近期审计视图条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxAuditViewEntry {
    /// 唯一 ID
    pub id: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 操作类型
    pub operation: OperationType,
    /// 目标（文件路径 / 命令 / URL）
    pub target: String,
    /// 操作者
    pub actor: String,
    /// 权限级别
    pub level: PermissionLevel,
    /// 审计动作
    pub action: SandboxAuditViewAction,
    /// 原因（可选）
    pub reason: Option<String>,
    /// worktree 路径（可选）
    pub worktree: Option<String>,
    /// 会话 ID（可选）
    pub session_id: Option<String>,
}

// ============================================================================
// 近期审计视图过滤器
// ============================================================================

/// Sandbox 近期审计视图过滤器
#[derive(Debug, Clone, Default)]
pub struct SandboxAuditViewFilter {
    /// 按操作类型过滤
    pub operation: Option<OperationType>,
    /// 按审计动作过滤
    pub action: Option<SandboxAuditViewAction>,
    /// 按操作者过滤
    pub actor: Option<String>,
    /// 按worktree过滤
    pub worktree: Option<String>,
    /// 按会话 ID 过滤
    pub session_id: Option<String>,
    /// 开始时间
    pub since: Option<DateTime<Utc>>,
    /// 结束时间
    pub until: Option<DateTime<Utc>>,
    /// 最大返回数量
    pub limit: Option<usize>,
}

impl SandboxAuditViewFilter {
    /// 创建空过滤器
    pub fn new() -> Self {
        Self::default()
    }

    /// 按操作类型过滤
    pub fn with_operation(mut self, operation: OperationType) -> Self {
        self.operation = Some(operation);
        self
    }

    /// 按审计动作过滤
    pub fn with_action(mut self, action: SandboxAuditViewAction) -> Self {
        self.action = Some(action);
        self
    }

    /// 按操作者过滤
    pub fn with_actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(actor.into());
        self
    }

    /// 按worktree过滤
    pub fn with_worktree(mut self, worktree: impl Into<String>) -> Self {
        self.worktree = Some(worktree.into());
        self
    }

    /// 按会话 ID 过滤
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// 设置时间范围（开始）
    pub fn with_since(mut self, since: DateTime<Utc>) -> Self {
        self.since = Some(since);
        self
    }

    /// 设置时间范围（结束）
    pub fn with_until(mut self, until: DateTime<Utc>) -> Self {
        self.until = Some(until);
        self
    }

    /// 设置最大返回数量
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// 检查条目是否匹配过滤器
    pub fn matches(&self, entry: &SandboxAuditViewEntry) -> bool {
        if let Some(ref op) = self.operation {
            if entry.operation != *op {
                return false;
            }
        }
        if let Some(ref action) = self.action {
            if entry.action != *action {
                return false;
            }
        }
        if let Some(ref actor) = self.actor {
            if entry.actor != *actor {
                return false;
            }
        }
        if let Some(ref worktree) = self.worktree {
            if entry.worktree.as_deref() != Some(worktree.as_str()) {
                return false;
            }
        }
        if let Some(ref session_id) = self.session_id {
            if entry.session_id.as_deref() != Some(session_id.as_str()) {
                return false;
            }
        }
        if let Some(since) = self.since {
            if entry.timestamp < since {
                return false;
            }
        }
        if let Some(until) = self.until {
            if entry.timestamp > until {
                return false;
            }
        }
        true
    }
}

// ============================================================================
// SandboxAuditView
// ============================================================================

/// Sandbox 近期审计视图缓存
///
/// 管理内存中的近期查询视图。结构化审计事实必须通过
/// `kernel::AuditRecorder` / `AuditSink` 写入；这里的内容可丢失、
/// 可清理，不应被当成持久事实源。
#[derive(Debug)]
pub struct SandboxAuditView {
    /// 近期视图条目（环形缓冲区）
    entries: VecDeque<SandboxAuditViewEntry>,
    /// 最大容量
    max_capacity: usize,
}

impl SandboxAuditView {
    /// 创建新的近期视图缓存
    pub fn new() -> Self {
        Self::with_capacity(10000)
    }

    /// 创建指定容量的近期视图缓存
    pub fn with_capacity(max_capacity: usize) -> Self {
        tracing::debug!(
            max_capacity = max_capacity,
            "Creating sandbox audit view cache"
        );
        Self {
            entries: VecDeque::with_capacity(max_capacity.min(1000)),
            max_capacity,
        }
    }

    /// 将条目加入近期视图缓存。
    ///
    /// 此方法不会写入结构化审计事实源。调用方如果需要持久审计，
    /// 必须先通过 `kernel::AuditRecorder` / `AuditSink` 写入。
    pub fn cache(&mut self, entry: SandboxAuditViewEntry) {
        tracing::debug!(
            id = %entry.id,
            operation = %entry.operation,
            target = %entry.target,
            action = %entry.action,
            "Caching sandbox audit view entry"
        );

        // 环形缓冲区：超过容量时移除最旧的条目
        if self.entries.len() >= self.max_capacity {
            self.entries.pop_front();
        }

        self.entries.push_back(entry);
    }

    /// 将允许操作加入近期视图缓存；不写入结构化审计事实源。
    pub fn cache_allowed(
        &mut self,
        operation: OperationType,
        target: &str,
        actor: &str,
        level: PermissionLevel,
        worktree: Option<&str>,
        session_id: Option<&str>,
    ) {
        self.cache(SandboxAuditViewEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            operation,
            target: target.to_string(),
            actor: actor.to_string(),
            level,
            action: SandboxAuditViewAction::Allowed,
            reason: None,
            worktree: worktree.map(|s| s.to_string()),
            session_id: session_id.map(|s| s.to_string()),
        });
    }

    /// 将拒绝操作加入近期视图缓存；不写入结构化审计事实源。
    pub fn cache_denied(
        &mut self,
        operation: OperationType,
        target: &str,
        actor: &str,
        level: PermissionLevel,
        reason: &str,
        worktree: Option<&str>,
        session_id: Option<&str>,
    ) {
        self.cache(SandboxAuditViewEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            operation,
            target: target.to_string(),
            actor: actor.to_string(),
            level,
            action: SandboxAuditViewAction::Denied,
            reason: Some(reason.to_string()),
            worktree: worktree.map(|s| s.to_string()),
            session_id: session_id.map(|s| s.to_string()),
        });
    }

    /// 将确认请求加入近期视图缓存；不写入结构化审计事实源。
    pub fn cache_confirm_requested(
        &mut self,
        operation: OperationType,
        target: &str,
        actor: &str,
        level: PermissionLevel,
        message: &str,
        worktree: Option<&str>,
        session_id: Option<&str>,
    ) {
        self.cache(SandboxAuditViewEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            operation,
            target: target.to_string(),
            actor: actor.to_string(),
            level,
            action: SandboxAuditViewAction::ConfirmRequested,
            reason: Some(message.to_string()),
            worktree: worktree.map(|s| s.to_string()),
            session_id: session_id.map(|s| s.to_string()),
        });
    }

    /// 将网络阻止结果加入近期视图缓存；不写入结构化审计事实源。
    pub fn cache_blocked_network(&mut self, domain: &str, reason: &str) {
        self.cache_denied(
            OperationType::NetworkRequest,
            domain,
            "agent",
            PermissionLevel::UserConfirm,
            reason,
            None,
            None,
        );
    }

    /// 查询近期审计视图
    ///
    /// # Arguments
    /// * `filter` - 查询过滤器
    ///
    /// # Returns
    /// 匹配的近期视图条目列表
    pub fn query(&self, filter: &SandboxAuditViewFilter) -> Vec<SandboxAuditViewEntry> {
        let results: Vec<SandboxAuditViewEntry> = self
            .entries
            .iter()
            .rev() // 从最新开始
            .filter(|entry| filter.matches(entry))
            .take(filter.limit.unwrap_or(usize::MAX))
            .cloned()
            .collect();

        tracing::debug!(
            filter_limit = ?filter.limit,
            result_count = results.len(),
            total_entries = self.entries.len(),
            "Sandbox audit view cache queried"
        );

        results
    }

    /// 获取最近的 N 条视图条目
    pub fn recent(&self, count: usize) -> Vec<SandboxAuditViewEntry> {
        self.entries.iter().rev().take(count).cloned().collect()
    }

    /// 获取近期视图条目数
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// 清除所有近期视图条目
    pub fn clear(&mut self) {
        let count = self.entries.len();
        tracing::info!(count = count, "Clearing sandbox audit view cache");
        self.entries.clear();
    }
}

impl Default for SandboxAuditView {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_entry(
        action: SandboxAuditViewAction,
        actor: &str,
        target: &str,
    ) -> SandboxAuditViewEntry {
        SandboxAuditViewEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            operation: OperationType::FileRead,
            target: target.to_string(),
            actor: actor.to_string(),
            level: PermissionLevel::LightCheck,
            action,
            reason: None,
            worktree: None,
            session_id: None,
        }
    }

    #[test]
    fn test_sandbox_audit_view_new() {
        let audit = SandboxAuditView::new();
        assert_eq!(audit.count(), 0);
    }

    #[test]
    fn test_sandbox_audit_view_cache() {
        let mut audit = SandboxAuditView::new();
        let entry = create_entry(SandboxAuditViewAction::Allowed, "agent", "/test/file.txt");
        audit.cache(entry);

        assert_eq!(audit.count(), 1);
    }

    #[test]
    fn test_sandbox_audit_view_cache_allowed() {
        let mut audit = SandboxAuditView::new();

        audit.cache_allowed(
            OperationType::FileRead,
            "/test/file.txt",
            "agent",
            PermissionLevel::LightCheck,
            Some("/worktree"),
            Some("sess_001"),
        );

        assert_eq!(audit.count(), 1);
        let entries = audit.recent(1);
        assert_eq!(entries[0].action, SandboxAuditViewAction::Allowed);
        assert_eq!(entries[0].worktree, Some("/worktree".to_string()));
    }

    #[test]
    fn test_sandbox_audit_view_cache_denied() {
        let mut audit = SandboxAuditView::new();

        audit.cache_denied(
            OperationType::FileDelete,
            "/etc/passwd",
            "agent",
            PermissionLevel::UserConfirm,
            "系统文件禁止访问",
            None,
            None,
        );

        assert_eq!(audit.count(), 1);
        let entries = audit.recent(1);
        assert_eq!(entries[0].action, SandboxAuditViewAction::Denied);
        assert_eq!(entries[0].reason, Some("系统文件禁止访问".to_string()));
    }

    #[test]
    fn test_sandbox_audit_view_cache_confirm_requested() {
        let mut audit = SandboxAuditView::new();

        audit.cache_confirm_requested(
            OperationType::CommandExecute,
            "sudo apt install",
            "agent",
            PermissionLevel::UserConfirm,
            "sudo 命令需要确认",
            Some("/worktree"),
            Some("sess_001"),
        );

        assert_eq!(audit.count(), 1);
        let entries = audit.recent(1);
        assert_eq!(entries[0].action, SandboxAuditViewAction::ConfirmRequested);
    }

    #[test]
    fn test_sandbox_audit_view_cache_blocked_network() {
        let mut audit = SandboxAuditView::new();

        audit.cache_blocked_network("evil.com", "域名在黑名单中");

        assert_eq!(audit.count(), 1);
        let entries = audit.recent(1);
        assert_eq!(entries[0].action, SandboxAuditViewAction::Denied);
        assert_eq!(entries[0].operation, OperationType::NetworkRequest);
    }

    // ======================================================================
    // 查询测试
    // ======================================================================

    #[test]
    fn test_audit_view_query_by_action() {
        let mut audit = SandboxAuditView::new();

        audit.cache(create_entry(SandboxAuditViewAction::Allowed, "agent", "/a"));
        audit.cache(create_entry(SandboxAuditViewAction::Denied, "agent", "/b"));
        audit.cache(create_entry(SandboxAuditViewAction::Allowed, "user", "/c"));

        let filter = SandboxAuditViewFilter::new().with_action(SandboxAuditViewAction::Allowed);
        let results = audit.query(&filter);
        assert_eq!(results.len(), 2);

        let filter = SandboxAuditViewFilter::new().with_action(SandboxAuditViewAction::Denied);
        let results = audit.query(&filter);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_audit_view_query_by_actor() {
        let mut audit = SandboxAuditView::new();

        audit.cache(create_entry(SandboxAuditViewAction::Allowed, "agent", "/a"));
        audit.cache(create_entry(SandboxAuditViewAction::Allowed, "user", "/b"));
        audit.cache(create_entry(SandboxAuditViewAction::Denied, "agent", "/c"));

        let filter = SandboxAuditViewFilter::new().with_actor("agent");
        let results = audit.query(&filter);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_audit_view_query_by_worktree() {
        let mut audit = SandboxAuditView::new();

        let mut entry1 = create_entry(SandboxAuditViewAction::Allowed, "agent", "/a");
        entry1.worktree = Some("/worktree1".to_string());
        audit.cache(entry1);

        let mut entry2 = create_entry(SandboxAuditViewAction::Allowed, "agent", "/b");
        entry2.worktree = Some("/worktree2".to_string());
        audit.cache(entry2);

        let filter = SandboxAuditViewFilter::new().with_worktree("/worktree1");
        let results = audit.query(&filter);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_audit_view_query_by_session_id() {
        let mut audit = SandboxAuditView::new();

        let mut entry1 = create_entry(SandboxAuditViewAction::Allowed, "agent", "/a");
        entry1.session_id = Some("sess_001".to_string());
        audit.cache(entry1);

        let mut entry2 = create_entry(SandboxAuditViewAction::Allowed, "agent", "/b");
        entry2.session_id = Some("sess_002".to_string());
        audit.cache(entry2);

        let filter = SandboxAuditViewFilter::new().with_session_id("sess_001");
        let results = audit.query(&filter);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_audit_view_query_by_operation() {
        let mut audit = SandboxAuditView::new();

        let mut entry1 = create_entry(SandboxAuditViewAction::Allowed, "agent", "/a");
        entry1.operation = OperationType::FileRead;
        audit.cache(entry1);

        let mut entry2 = create_entry(SandboxAuditViewAction::Allowed, "agent", "/b");
        entry2.operation = OperationType::CommandExecute;
        audit.cache(entry2);

        let filter = SandboxAuditViewFilter::new().with_operation(OperationType::FileRead);
        let results = audit.query(&filter);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_audit_view_query_with_limit() {
        let mut audit = SandboxAuditView::new();

        for i in 0..10 {
            audit.cache(create_entry(
                SandboxAuditViewAction::Allowed,
                "agent",
                &format!("/f{}", i),
            ));
        }

        let filter = SandboxAuditViewFilter::new().with_limit(3);
        let results = audit.query(&filter);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_audit_view_query_combined_filters() {
        let mut audit = SandboxAuditView::new();

        audit.cache(create_entry(SandboxAuditViewAction::Allowed, "agent", "/a"));
        audit.cache(create_entry(SandboxAuditViewAction::Denied, "agent", "/b"));
        audit.cache(create_entry(SandboxAuditViewAction::Allowed, "user", "/c"));

        let filter = SandboxAuditViewFilter::new()
            .with_actor("agent")
            .with_action(SandboxAuditViewAction::Allowed)
            .with_limit(1);

        let results = audit.query(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].actor, "agent");
        assert_eq!(results[0].action, SandboxAuditViewAction::Allowed);
    }

    // ======================================================================
    // 环形缓冲区测试
    // ======================================================================

    #[test]
    fn test_audit_view_circular_buffer() {
        let mut audit = SandboxAuditView::with_capacity(3);

        for i in 0..5 {
            audit.cache(create_entry(
                SandboxAuditViewAction::Allowed,
                "agent",
                &format!("/f{}", i),
            ));
        }

        // 只保留最后 3 条
        assert_eq!(audit.count(), 3);

        let recent = audit.recent(3);
        assert_eq!(recent[0].target, "/f4");
        assert_eq!(recent[1].target, "/f3");
        assert_eq!(recent[2].target, "/f2");
    }

    // ======================================================================
    // 辅助操作测试
    // ======================================================================

    #[test]
    fn test_audit_view_recent() {
        let mut audit = SandboxAuditView::new();

        for i in 0..5 {
            audit.cache(create_entry(
                SandboxAuditViewAction::Allowed,
                "agent",
                &format!("/f{}", i),
            ));
        }

        let recent = audit.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].target, "/f4");
        assert_eq!(recent[1].target, "/f3");
    }

    #[test]
    fn test_audit_view_clear() {
        let mut audit = SandboxAuditView::new();

        audit.cache(create_entry(SandboxAuditViewAction::Allowed, "agent", "/a"));
        audit.cache(create_entry(SandboxAuditViewAction::Denied, "agent", "/b"));
        assert_eq!(audit.count(), 2);

        audit.clear();
        assert_eq!(audit.count(), 0);
    }

    #[test]
    fn test_audit_view_default() {
        let audit = SandboxAuditView::default();
        assert_eq!(audit.count(), 0);
    }

    // ======================================================================
    // 类型测试
    // ======================================================================

    #[test]
    fn test_audit_view_action_display() {
        assert_eq!(SandboxAuditViewAction::Allowed.to_string(), "allowed");
        assert_eq!(SandboxAuditViewAction::Denied.to_string(), "denied");
        assert_eq!(
            SandboxAuditViewAction::ConfirmRequested.to_string(),
            "confirm_requested"
        );
        assert_eq!(SandboxAuditViewAction::Confirmed.to_string(), "confirmed");
        assert_eq!(
            SandboxAuditViewAction::UserDenied.to_string(),
            "user_denied"
        );
    }

    #[test]
    fn test_audit_view_action_serialization() {
        let action = SandboxAuditViewAction::Denied;
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: SandboxAuditViewAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, deserialized);
    }

    #[test]
    fn test_audit_view_entry_serialization() {
        let entry = SandboxAuditViewEntry {
            id: "test-id".to_string(),
            timestamp: Utc::now(),
            operation: OperationType::FileRead,
            target: "/test/file.txt".to_string(),
            actor: "agent".to_string(),
            level: PermissionLevel::LightCheck,
            action: SandboxAuditViewAction::Allowed,
            reason: None,
            worktree: Some("/worktree".to_string()),
            session_id: Some("sess_001".to_string()),
        };

        let json = serde_json::to_string_pretty(&entry).unwrap();
        let deserialized: SandboxAuditViewEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "test-id");
        assert_eq!(deserialized.action, SandboxAuditViewAction::Allowed);
        assert_eq!(deserialized.worktree, Some("/worktree".to_string()));
    }

    #[test]
    fn test_audit_view_filter_no_filters_matches_all() {
        let filter = SandboxAuditViewFilter::new();
        let entry = create_entry(SandboxAuditViewAction::Allowed, "agent", "/test");
        assert!(filter.matches(&entry));
    }

    #[test]
    fn test_audit_view_filter_mismatch() {
        let filter = SandboxAuditViewFilter::new()
            .with_action(SandboxAuditViewAction::Denied)
            .with_actor("user");

        let entry = create_entry(SandboxAuditViewAction::Allowed, "agent", "/test");
        assert!(!filter.matches(&entry));
    }
}
