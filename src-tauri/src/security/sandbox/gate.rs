//! 权限门禁（统一入口）
//!
//! 基于设计文档 §4 实现，作为 Sandbox 模块的统一入口。
//!
//! # 设计思路
//! - Sandbox 结构体聚合所有子模块
//! - `check()` 方法根据操作类型自动分发到对应的子校验
//! - 统一写入 kernel audit，并维护 Sandbox 近期审计视图
//! - 统一事件发布

use std::path::Path;
use std::sync::{Arc, RwLock};

use anyhow::Result;

use crate::kernel::{
    AuditDigest, AuditRecord, AuditRecorder, AuditStatus, EventEnvelope, KernelContext, KernelScope,
};
use triomphe::Arc as SharedArc;

use super::access_control::AccessControl;
use super::audit_view::{
    SandboxAuditView, SandboxAuditViewAction, SandboxAuditViewEntry, SandboxAuditViewFilter,
};
use super::command_rules::{CommandRule, CommandRules, CommandShell};
use super::permission::{
    ApprovalMode, CheckResult, OperationRequest, OperationType, PermissionLevel,
};
use super::policy::{default_command_rules, NetworkPolicy, SandboxPolicy};
use super::resource_limit::{ResourceLimit, ResourceLimitManager, ResourceType, ResourceUsage};
use super::worktree_trust::{TrustLevel, WorktreeTrustManager};

// ============================================================================
// Sandbox
// ============================================================================

/// Sandbox 安全沙箱
///
/// 统一入口，聚合路径访问控制、命令规则引擎、worktree 信任、资源限制、近期审计视图、
/// 网络策略、审批模式等所有子模块。
pub struct Sandbox {
    /// 路径访问控制
    access_control: RwLock<AccessControl>,
    /// 命令规则引擎
    command_rules: RwLock<CommandRules>,
    /// worktree 信任管理
    worktree_trust: RwLock<WorktreeTrustManager>,
    /// 资源限制管理
    resource_limits: RwLock<ResourceLimitManager>,
    /// Sandbox 近期审计视图缓存
    audit_view: RwLock<SandboxAuditView>,
    /// 内核审计记录器。SandboxAuditView 只保留近期查询缓存，持久事实写入这里。
    audit_recorder: AuditRecorder,
    /// 审批模式
    approval_mode: RwLock<ApprovalMode>,
    /// 网络策略
    network_policy: RwLock<NetworkPolicy>,
    /// 事件总线
    event_bus: Arc<dyn crate::kernel::EventBus>,
}

impl Sandbox {
    /// 创建新的 Sandbox 实例（使用默认配置，从配置文件加载黑名单）
    pub fn new(event_bus: Arc<dyn crate::kernel::EventBus>) -> Self {
        Self::with_policy(event_bus, SandboxPolicy::default())
    }

    /// 创建新的 Sandbox 实例，从指定目录加载配置
    pub fn with_config(
        event_bus: Arc<dyn crate::kernel::EventBus>,
        user_home: Option<&Path>,
        project_dir: Option<&Path>,
    ) -> Self {
        let network_policy = NetworkPolicy::load_from_config(user_home, project_dir);
        let policy = SandboxPolicy {
            command_rules: default_command_rules(),
            network_policy,
        };
        Self::with_policy(event_bus, policy)
    }

    /// 使用指定策略创建 Sandbox 实例
    ///
    /// # Arguments
    /// * `event_bus` - 事件总线
    /// * `policy` - 沙箱策略配置
    pub fn with_policy(event_bus: Arc<dyn crate::kernel::EventBus>, policy: SandboxPolicy) -> Self {
        tracing::info!("Creating new Sandbox");

        // 从策略配置构建命令规则引擎
        let command_rules = {
            let rules: Vec<CommandRule> = policy
                .command_rules
                .iter()
                .filter_map(|config| match config.to_command_rule() {
                    Ok(rule) => Some(rule),
                    Err(e) => {
                        tracing::warn!(error = %e, "Skipping invalid command rule config");
                        None
                    }
                })
                .collect();

            match CommandRules::from_rules(&rules) {
                Ok(cr) => cr,
                Err(e) => {
                    tracing::error!(error = %e, "Failed to create CommandRules, using empty rules");
                    CommandRules::new()
                }
            }
        };

        Self {
            access_control: RwLock::new(AccessControl::new()),
            command_rules: RwLock::new(command_rules),
            worktree_trust: RwLock::new(WorktreeTrustManager::new()),
            resource_limits: RwLock::new(ResourceLimitManager::new()),
            audit_view: RwLock::new(SandboxAuditView::new()),
            audit_recorder: AuditRecorder::disabled(),
            approval_mode: RwLock::new(ApprovalMode::default()),
            network_policy: RwLock::new(policy.network_policy),
            event_bus,
        }
    }

    /// 注入内核审计记录器。
    ///
    /// 应用运行时传入 StorageAuditSink；测试或无持久层场景保持 disabled。
    pub fn with_audit_recorder(mut self, audit_recorder: AuditRecorder) -> Self {
        self.audit_recorder = audit_recorder;
        self
    }

    /// 返回当前注入的内核审计记录器。
    pub fn audit_recorder(&self) -> AuditRecorder {
        self.audit_recorder.clone()
    }

    // ======================================================================
    // 统一校验入口
    // ======================================================================

    /// 统一校验入口
    ///
    /// 根据操作类型自动分发到对应的子校验逻辑。
    ///
    /// # Arguments
    /// * `request` - 操作请求
    ///
    /// # Returns
    /// 校验结果
    pub fn check(&self, request: &OperationRequest) -> Result<CheckResult> {
        tracing::debug!(
            operation = %request.operation,
            target = %request.target,
            actor = %request.actor,
            "Sandbox check"
        );

        let result = match &request.operation {
            OperationType::FileRead
            | OperationType::FileWrite
            | OperationType::FileDelete
            | OperationType::DirCreate
            | OperationType::DirDelete => self.check_path_request(request),
            OperationType::CommandExecute => self.check_command_request(request),
            OperationType::NetworkRequest => self.check_network(&request.target),
        };

        // 审计记录
        self.audit_check_result(request, &result);

        // 发布事件
        self.emit_check_event(request, &result);

        Ok(result)
    }

    // ======================================================================
    // 路径校验
    // ======================================================================

    /// 路径校验
    ///
    /// # Arguments
    /// * `path` - 待校验的路径
    /// * `operation` - 操作类型
    /// * `worktree` - worktree根路径
    pub fn check_path(
        &self,
        path: &Path,
        operation: &OperationType,
        worktree: &Path,
    ) -> CheckResult {
        tracing::debug!(
            path = %path.display(),
            operation = %operation,
            worktree = %worktree.display(),
            "Sandbox path check"
        );

        // 先检查worktree 信任
        if let Some(trust) = self.worktree_trust.read().unwrap().get_trust(worktree) {
            if trust == TrustLevel::Untrusted {
                // 不信任的worktree只允许只读操作
                if !matches!(operation, OperationType::FileRead) {
                    return CheckResult::denied(
                        PermissionLevel::UserConfirm,
                        "worktree不信任，仅允许只读操作",
                    );
                }
            }
        }

        // 审批模式检查
        let mode = *self.approval_mode.read().unwrap();
        let outside_worktree = path.is_absolute() && !path.starts_with(worktree);
        if outside_worktree {
            return match mode {
                ApprovalMode::Suggest => CheckResult::needs_confirm(
                    PermissionLevel::UserConfirm,
                    format!(
                        "操作 {} 访问worktree外路径，需要确认: {}",
                        operation,
                        path.display()
                    ),
                ),
                ApprovalMode::AutoEdit => {
                    if matches!(operation, OperationType::FileRead) {
                        CheckResult::allowed(PermissionLevel::LightCheck)
                    } else {
                        CheckResult::needs_confirm(
                            PermissionLevel::UserConfirm,
                            format!(
                                "操作 {} 修改worktree外路径，需要确认: {}",
                                operation,
                                path.display()
                            ),
                        )
                    }
                }
                ApprovalMode::FullAuto => CheckResult::allowed(PermissionLevel::Unrestricted),
            };
        }

        if operation.requires_confirm_in_mode(&mode) {
            return CheckResult::needs_confirm(
                PermissionLevel::UserConfirm,
                format!("操作 {} 在 {} 模式下需要确认", operation, mode),
            );
        }

        // 路径访问控制校验
        self.access_control
            .read()
            .unwrap()
            .check(path, operation, worktree)
    }

    /// 校验操作请求中的路径
    fn check_path_request(&self, request: &OperationRequest) -> CheckResult {
        let worktree = request.worktree_path.as_deref().unwrap_or("/");
        let worktree_path = Path::new(worktree);
        let target = request.target.trim();
        let target = if target.is_empty() { "." } else { target };
        let target_path = Path::new(target);
        let resolved_target = if target_path.is_absolute() {
            crate::extension::types::PathManager::normalize(target_path)
        } else {
            crate::extension::types::PathManager::resolve(worktree_path, target_path)
        };

        self.check_path(&resolved_target, &request.operation, worktree_path)
    }

    // ======================================================================
    // 命令校验
    // ======================================================================

    /// 命令校验
    ///
    /// # Arguments
    /// * `command` - 待校验的命令
    /// * `worktree` - worktree 路径（用于信任检查）
    pub fn check_command(&self, command: &str, worktree: &Path) -> CheckResult {
        self.check_command_with_shell(command, worktree, None)
    }

    /// 按实际执行 shell 校验命令。
    pub fn check_command_for_shell(
        &self,
        command: &str,
        worktree: &Path,
        shell: CommandShell,
    ) -> CheckResult {
        self.check_command_with_shell(command, worktree, Some(shell))
    }

    fn check_command_with_shell(
        &self,
        command: &str,
        worktree: &Path,
        shell: Option<CommandShell>,
    ) -> CheckResult {
        tracing::debug!(command = %command, worktree = %worktree.display(), "Sandbox command check");

        // worktree 信任检查
        if let Some(trust) = self.worktree_trust.read().unwrap().get_trust(worktree) {
            if trust == TrustLevel::Untrusted {
                return CheckResult::denied(
                    PermissionLevel::UserConfirm,
                    "worktree不信任，禁止执行命令",
                );
            }
        }

        // 命令规则引擎校验
        let rules = self.command_rules.read().unwrap();
        match shell {
            Some(shell) => rules.check_for_shell(command, shell),
            None => rules.check(command),
        }
    }

    /// 校验操作请求中的命令
    fn check_command_request(&self, request: &OperationRequest) -> CheckResult {
        let worktree = request
            .worktree_path
            .as_deref()
            .map(Path::new)
            .unwrap_or_else(|| Path::new("/"));

        // 审批模式检查
        let mode = *self.approval_mode.read().unwrap();
        if request.operation.requires_confirm_in_mode(&mode) {
            return CheckResult::needs_confirm(
                PermissionLevel::UserConfirm,
                format!("命令执行在 {} 模式下需要确认: {}", mode, request.target),
            );
        }

        self.check_command(&request.target, worktree)
    }

    // ======================================================================
    // 网络校验
    // ======================================================================

    /// 网络校验
    ///
    /// 仅管控 Agent 工具调用产生的网络请求，不管控 Gateway 模型连接。
    /// 默认放行，仅拦截黑名单域名。
    ///
    /// # Arguments
    /// * `url` - 待校验的 URL
    pub fn check_network(&self, url: &str) -> CheckResult {
        let mode = *self.approval_mode.read().unwrap();
        let policy = self.network_policy.read().unwrap();
        let domain = extract_domain(url);

        tracing::debug!(url = %url, domain = %domain, mode = %mode, "Sandbox network check");

        // Suggest 模式：所有网络请求都需要用户确认
        if mode == ApprovalMode::Suggest {
            return CheckResult::needs_confirm(
                PermissionLevel::UserConfirm,
                format!("Agent 请求访问网络: {}", url),
            );
        }

        // AutoEdit / FullAuto 模式：默认放行，仅拦截黑名单
        if policy.is_blocked(&domain) {
            let request = OperationRequest::new(OperationType::NetworkRequest, url, "agent");
            let result = CheckResult::denied(
                PermissionLevel::UserConfirm,
                format!("域名 {} 在黑名单中（已知恶意域名）", domain),
            );
            self.audit_check_result(&request, &result);
            return result;
        }

        // 默认放行
        CheckResult::allowed(PermissionLevel::LightCheck)
    }

    // ======================================================================
    // worktree 信任
    // ======================================================================

    /// 获取worktree 信任级别
    pub fn get_trust(&self, worktree: &Path) -> Option<TrustLevel> {
        self.worktree_trust.read().unwrap().get_trust(worktree)
    }

    /// 设置worktree 信任级别
    pub fn set_trust(&self, worktree: &Path, trust: TrustLevel) -> Result<()> {
        tracing::info!(
            worktree = %worktree.display(),
            trust = %trust,
            "Setting worktree trust"
        );

        self.worktree_trust
            .write()
            .unwrap()
            .set_trust(worktree, trust);

        // 发布事件
        if let Err(error) = self.event_bus.emit(EventEnvelope::new(
            "sandbox.trust.changed",
            KernelContext::new("sandbox", KernelScope::global()),
            Some(SharedArc::new(serde_json::json!({
                "worktree": worktree.to_string_lossy(),
                "trust": trust.as_str(),
            }))),
        )) {
            tracing::warn!(
                event = "sandbox.trust.changed",
                error = %error,
                "Failed to emit sandbox event"
            );
        }

        Ok(())
    }

    /// 清除会话级信任
    pub fn clear_session_trust(&self) {
        self.worktree_trust.write().unwrap().clear_session_trust();
    }

    // ======================================================================
    // 审批模式
    // ======================================================================

    /// 获取当前审批模式
    pub fn get_approval_mode(&self) -> ApprovalMode {
        *self.approval_mode.read().unwrap()
    }

    /// 设置审批模式
    pub fn set_approval_mode(&self, mode: ApprovalMode) -> Result<()> {
        let previous = *self.approval_mode.read().unwrap();
        tracing::info!(previous = %previous, new = %mode, "Setting approval mode");

        *self.approval_mode.write().unwrap() = mode;

        // 发布事件
        if let Err(error) = self.event_bus.emit(EventEnvelope::new(
            "sandbox.approvalMode.changed",
            KernelContext::new("sandbox", KernelScope::global()),
            Some(SharedArc::new(serde_json::json!({
                "mode": mode.as_str(),
                "previousMode": previous.as_str(),
            }))),
        )) {
            tracing::warn!(
                event = "sandbox.approvalMode.changed",
                error = %error,
                "Failed to emit sandbox event"
            );
        }

        Ok(())
    }

    // ======================================================================
    // 网络策略
    // ======================================================================

    /// 获取网络策略
    pub fn get_network_policy(&self) -> NetworkPolicy {
        self.network_policy.read().unwrap().clone()
    }

    /// 设置网络策略
    pub fn set_network_policy(&self, policy: NetworkPolicy) -> Result<()> {
        tracing::info!("Setting network policy");

        *self.network_policy.write().unwrap() = policy;

        // 发布事件
        if let Err(error) = self.event_bus.emit(EventEnvelope::new(
            "sandbox.networkPolicy.changed",
            KernelContext::new("sandbox", KernelScope::global()),
            Some(SharedArc::new(serde_json::json!({
                "blockedDomains": self.network_policy.read().unwrap().blocked_domains.len(),
            }))),
        )) {
            tracing::warn!(
                event = "sandbox.networkPolicy.changed",
                error = %error,
                "Failed to emit sandbox event"
            );
        }

        Ok(())
    }

    // ======================================================================
    // 路径访问控制
    // ======================================================================

    /// 添加白名单路径
    pub fn allow_path(&self, pattern: &str, description: &str) {
        self.access_control
            .write()
            .unwrap()
            .allow(pattern, description);
    }

    /// 添加黑名单路径
    pub fn deny_path(&self, pattern: &str, description: &str) {
        self.access_control
            .write()
            .unwrap()
            .deny(pattern, description);
    }

    // ======================================================================
    // 命令规则管理
    // ======================================================================

    /// 获取所有命令规则
    pub fn get_command_rules(&self) -> Vec<CommandRule> {
        self.command_rules.read().unwrap().rules()
    }

    /// 设置命令规则（替换所有现有规则）
    pub fn set_command_rules(&self, rules: Vec<CommandRule>) -> Result<()> {
        match CommandRules::from_rules(&rules) {
            Ok(new_rules) => {
                *self.command_rules.write().unwrap() = new_rules;

                // 发布事件
                if let Err(error) = self.event_bus.emit(EventEnvelope::new(
                    "sandbox.policy.changed",
                    KernelContext::new("sandbox", KernelScope::global()),
                    Some(SharedArc::new(serde_json::json!({
                        "rules": rules.iter().map(|r| &r.description).collect::<Vec<_>>(),
                    }))),
                )) {
                    tracing::warn!(
                        event = "sandbox.policy.changed",
                        error = %error,
                        "Failed to emit sandbox event"
                    );
                }

                tracing::info!(count = rules.len(), "Command rules updated");
                Ok(())
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to set command rules");
                Err(anyhow::anyhow!("{}", e))
            }
        }
    }

    // ======================================================================
    // 资源限制
    // ======================================================================

    /// 设置资源限制
    pub fn set_resource_limit(&self, resource: ResourceType, limit: ResourceLimit) -> Result<()> {
        self.resource_limits
            .write()
            .unwrap()
            .set_limit(resource, limit);
        Ok(())
    }

    /// 获取资源用量
    pub fn get_resource_usage(&self, resource: ResourceType) -> ResourceUsage {
        self.resource_limits.read().unwrap().get_usage(&resource)
    }

    /// 列出所有资源限制和用量
    pub fn list_resource_limits(&self) -> Vec<(ResourceType, ResourceLimit, ResourceUsage)> {
        self.resource_limits.read().unwrap().list_all()
    }

    /// 更新资源用量
    pub fn update_resource_usage(&self, resource: ResourceType, current: f64) {
        let result = self
            .resource_limits
            .write()
            .unwrap()
            .update_usage(resource, current);

        match result {
            super::resource_limit::ResourceCheckResult::Warning => {
                if let Err(error) = self.event_bus.emit(EventEnvelope::new(
                    "sandbox.resource.warning",
                    KernelContext::new("sandbox", KernelScope::global()),
                    Some(SharedArc::new(serde_json::json!({
                        "resource": resource.as_str(),
                        "usage": current,
                    }))),
                )) {
                    tracing::warn!(
                        event = "sandbox.resource.warning",
                        error = %error,
                        "Failed to emit sandbox event"
                    );
                }
            }
            super::resource_limit::ResourceCheckResult::Exceeded => {
                if let Err(error) = self.event_bus.emit(EventEnvelope::new(
                    "sandbox.resource.exceeded",
                    KernelContext::new("sandbox", KernelScope::global()),
                    Some(SharedArc::new(serde_json::json!({
                        "resource": resource.as_str(),
                        "usage": current,
                    }))),
                )) {
                    tracing::warn!(
                        event = "sandbox.resource.exceeded",
                        error = %error,
                        "Failed to emit sandbox event"
                    );
                }
            }
            _ => {}
        }
    }

    // ======================================================================
    // 近期审计视图
    // ======================================================================

    /// 查询近期审计视图
    pub fn get_recent_audit_view(
        &self,
        filter: SandboxAuditViewFilter,
    ) -> Vec<SandboxAuditViewEntry> {
        self.audit_view.read().unwrap().query(&filter)
    }

    /// 获取近期审计视图条目数
    pub fn audit_view_count(&self) -> usize {
        self.audit_view.read().unwrap().count()
    }

    // ======================================================================
    // 内部辅助方法
    // ======================================================================

    /// 写入 kernel audit，并同步到 Sandbox 近期审计视图缓存。
    fn audit_check_result(&self, request: &OperationRequest, result: &CheckResult) {
        let mut audit_view = self.audit_view.write().unwrap();

        let action = if result.require_confirm {
            SandboxAuditViewAction::ConfirmRequested
        } else if result.allowed {
            SandboxAuditViewAction::Allowed
        } else {
            SandboxAuditViewAction::Denied
        };

        let entry = SandboxAuditViewEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            operation: request.operation.clone(),
            target: request.target.clone(),
            actor: request.actor.clone(),
            level: result.level,
            action,
            reason: result.reason.clone(),
            worktree: request.worktree_path.clone(),
            session_id: request.session_id.clone(),
        };

        self.record_kernel_audit(&entry);
        audit_view.cache(entry);
    }

    fn record_kernel_audit(&self, entry: &SandboxAuditViewEntry) {
        let scope = entry
            .session_id
            .as_ref()
            .map(|session_id| KernelScope::scoped("session", session_id))
            .or_else(|| {
                entry
                    .worktree
                    .as_ref()
                    .map(|worktree| KernelScope::scoped("worktree", worktree))
            })
            .unwrap_or_else(KernelScope::global);
        let status = match entry.action {
            SandboxAuditViewAction::Allowed | SandboxAuditViewAction::Confirmed => {
                AuditStatus::Success
            }
            SandboxAuditViewAction::Denied | SandboxAuditViewAction::UserDenied => {
                AuditStatus::Failed
            }
            SandboxAuditViewAction::ConfirmRequested => AuditStatus::Retried,
        };
        let context = KernelContext::new("sandbox", scope).with_owner(entry.actor.clone());
        let mut record = AuditRecord::new(
            &context,
            entry.id.clone(),
            entry.operation.to_string(),
            status,
        );
        record.policy_decision = Some(serde_json::json!({
            "action": entry.action.to_string(),
            "permissionLevel": entry.level.to_string(),
            "reason": entry.reason,
        }));
        record.input_digest = AuditDigest::Metadata {
            fields: vec![
                crate::kernel::FieldMeta {
                    name: "operation".into(),
                    value_type: "string".into(),
                    byte_size: entry.operation.to_string().len(),
                },
                crate::kernel::FieldMeta {
                    name: "target".into(),
                    value_type: "string".into(),
                    byte_size: entry.target.len(),
                },
                crate::kernel::FieldMeta {
                    name: "actor".into(),
                    value_type: "string".into(),
                    byte_size: entry.actor.len(),
                },
            ],
        };

        if let Err(error) = self.audit_recorder.record_owned(record) {
            tracing::warn!(
                audit_id = %entry.id,
                error = %error,
                "Failed to persist sandbox audit record"
            );
        }
    }

    /// 发布校验事件
    fn emit_check_event(&self, request: &OperationRequest, result: &CheckResult) {
        let actor_type = if request.is_extension() {
            "extension"
        } else if request.is_agent() {
            "agent"
        } else {
            "user"
        };

        if result.require_confirm {
            if let Err(error) = self.event_bus.emit(EventEnvelope::new(
                "sandbox.check.confirm",
                KernelContext::new("sandbox", KernelScope::global()),
                Some(SharedArc::new(serde_json::json!({
                    "operation": request.operation.to_string(),
                    "target": request.target,
                    "message": result.confirm_message.as_deref().unwrap_or(""),
                    "actor": actor_type,
                }))),
            )) {
                tracing::warn!(
                    event = "sandbox.check.confirm",
                    error = %error,
                    "Failed to emit sandbox event"
                );
            }
        } else if result.allowed {
            if let Err(error) = self.event_bus.emit(EventEnvelope::new(
                "sandbox.check.allowed",
                KernelContext::new("sandbox", KernelScope::global()),
                Some(SharedArc::new(serde_json::json!({
                    "operation": request.operation.to_string(),
                    "target": request.target,
                    "level": result.level.to_string(),
                    "actor": actor_type,
                }))),
            )) {
                tracing::warn!(
                    event = "sandbox.check.allowed",
                    error = %error,
                    "Failed to emit sandbox event"
                );
            }
        } else {
            if let Err(error) = self.event_bus.emit(EventEnvelope::new(
                "sandbox.check.denied",
                KernelContext::new("sandbox", KernelScope::global()),
                Some(SharedArc::new(serde_json::json!({
                    "operation": request.operation.to_string(),
                    "target": request.target,
                    "reason": result.reason.as_deref().unwrap_or(""),
                    "actor": actor_type,
                }))),
            )) {
                tracing::warn!(
                    event = "sandbox.check.denied",
                    error = %error,
                    "Failed to emit sandbox event"
                );
            }
        }
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 从 URL 提取域名
fn extract_domain(url: &str) -> String {
    let url = url.trim();

    // 去掉协议前缀
    let without_protocol = if let Some(pos) = url.find("://") {
        &url[pos + 3..]
    } else {
        url
    };

    // 提取域名（到第一个 / 或 : 或结尾）
    let domain = without_protocol
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("");

    domain.to_lowercase()
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn test_event_bus() -> Arc<dyn crate::kernel::EventBus> {
        static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
        let runtime = RUNTIME.get_or_init(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
        });
        Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            runtime.handle().clone(),
        ))
    }

    fn create_sandbox() -> Sandbox {
        let event_bus: Arc<dyn crate::kernel::EventBus> = test_event_bus();
        Sandbox::new(event_bus)
    }

    fn worktree() -> PathBuf {
        PathBuf::from("/home/user/worktree")
    }

    // ======================================================================
    // 辅助函数测试
    // ======================================================================

    #[test]
    fn test_extract_domain() {
        assert_eq!(extract_domain("https://crates.io/api/v1"), "crates.io");
        assert_eq!(
            extract_domain("http://api.github.com/repos"),
            "api.github.com"
        );
        assert_eq!(
            extract_domain("https://example.com:8080/path"),
            "example.com"
        );
        assert_eq!(extract_domain("crates.io"), "crates.io");
        assert_eq!(extract_domain("https://127.0.0.1:3000"), "127.0.0.1");
    }

    // ======================================================================
    // Sandbox 创建测试
    // ======================================================================

    #[test]
    fn test_sandbox_new() {
        let sandbox = create_sandbox();
        assert_eq!(sandbox.get_approval_mode(), ApprovalMode::Suggest);
        assert_eq!(sandbox.audit_view_count(), 0);
    }

    #[test]
    fn test_sandbox_with_policy() {
        let event_bus: Arc<dyn crate::kernel::EventBus> = test_event_bus();
        let policy = SandboxPolicy {
            command_rules: vec![],
            network_policy: NetworkPolicy::default(),
        };
        let sandbox = Sandbox::with_policy(event_bus, policy);

        assert!(sandbox.get_command_rules().is_empty());
        assert!(sandbox.get_network_policy().blocked_domains.is_empty());
    }

    // ======================================================================
    // 审批模式测试
    // ======================================================================

    #[test]
    fn test_set_approval_mode() {
        let sandbox = create_sandbox();

        assert_eq!(sandbox.get_approval_mode(), ApprovalMode::Suggest);

        sandbox.set_approval_mode(ApprovalMode::AutoEdit).unwrap();
        assert_eq!(sandbox.get_approval_mode(), ApprovalMode::AutoEdit);

        sandbox.set_approval_mode(ApprovalMode::FullAuto).unwrap();
        assert_eq!(sandbox.get_approval_mode(), ApprovalMode::FullAuto);
    }

    // ======================================================================
    // 路径校验测试
    // ======================================================================

    #[test]
    fn test_check_path_with_trusted_worktree() {
        let sandbox = create_sandbox();
        let ws = worktree();

        // 设置worktree为信任
        sandbox.set_trust(&ws, TrustLevel::Trusted).unwrap();

        // 信任worktree中，Suggest 模式下 FileRead 不需要确认
        let result = sandbox.check_path(
            Path::new("/home/user/worktree/src/main.rs"),
            &OperationType::FileRead,
            &ws,
        );
        assert!(result.allowed);
    }

    #[test]
    fn test_check_path_with_untrusted_worktree() {
        let sandbox = create_sandbox();
        let ws = worktree();

        // 设置worktree为不信任
        sandbox.set_trust(&ws, TrustLevel::Untrusted).unwrap();

        // 不信任worktree中，写操作被拒绝
        let result = sandbox.check_path(
            Path::new("/home/user/worktree/src/main.rs"),
            &OperationType::FileWrite,
            &ws,
        );
        assert!(!result.allowed);
        assert!(result.reason.as_ref().unwrap().contains("不信任"));

        // 只读操作可以通过
        let result = sandbox.check_path(
            Path::new("/home/user/worktree/src/main.rs"),
            &OperationType::FileRead,
            &ws,
        );
        assert!(result.allowed);
    }

    #[test]
    fn test_check_path_suggest_mode_requires_confirm() {
        let sandbox = create_sandbox();
        let ws = worktree();

        sandbox.set_trust(&ws, TrustLevel::Trusted).unwrap();
        sandbox.set_approval_mode(ApprovalMode::Suggest).unwrap();

        // Suggest 模式下，FileWrite 需要确认
        let result = sandbox.check_path(
            Path::new("/home/user/worktree/src/main.rs"),
            &OperationType::FileWrite,
            &ws,
        );
        assert!(result.allowed);
        assert!(result.require_confirm);
    }

    #[test]
    fn test_check_path_autoedit_mode() {
        let sandbox = create_sandbox();
        let ws = worktree();

        sandbox.set_trust(&ws, TrustLevel::Trusted).unwrap();
        sandbox.set_approval_mode(ApprovalMode::AutoEdit).unwrap();

        // AutoEdit 模式下，FileRead 不需要确认
        let result = sandbox.check_path(
            Path::new("/home/user/worktree/src/main.rs"),
            &OperationType::FileRead,
            &ws,
        );
        assert!(result.allowed);
        assert!(!result.require_confirm);

        // AutoEdit 模式下，FileWrite 不需要确认
        let result = sandbox.check_path(
            Path::new("/home/user/worktree/src/main.rs"),
            &OperationType::FileWrite,
            &ws,
        );
        assert!(result.allowed);
        assert!(!result.require_confirm);

        // AutoEdit 模式下，FileDelete 需要确认
        let result = sandbox.check_path(
            Path::new("/home/user/worktree/src/main.rs"),
            &OperationType::FileDelete,
            &ws,
        );
        assert!(result.require_confirm);
    }

    #[test]
    fn test_check_path_with_access_control() {
        let sandbox = create_sandbox();
        let ws = worktree();

        sandbox.set_trust(&ws, TrustLevel::Trusted).unwrap();

        // 添加路径规则
        sandbox.deny_path("/home/user/worktree/.env", "环境变量文件");
        sandbox.allow_path("/home/user/worktree/src", "源代码目录");

        // 被黑名单阻止
        let result = sandbox.check_path(
            Path::new("/home/user/worktree/.env"),
            &OperationType::FileRead,
            &ws,
        );
        assert!(!result.allowed);

        // 在白名单中
        let result = sandbox.check_path(
            Path::new("/home/user/worktree/src/main.rs"),
            &OperationType::FileRead,
            &ws,
        );
        assert!(result.allowed);
    }

    // ======================================================================
    // 命令校验测试
    // ======================================================================

    #[test]
    fn test_check_command_allow() {
        let sandbox = create_sandbox();
        let ws = worktree();

        sandbox.set_trust(&ws, TrustLevel::Trusted).unwrap();

        // 白名单命令
        let result = sandbox.check_command("git status", &ws);
        assert!(result.allowed);
        assert!(!result.require_confirm);
    }

    #[test]
    fn test_check_command_deny() {
        let sandbox = create_sandbox();
        let ws = worktree();

        sandbox.set_trust(&ws, TrustLevel::Trusted).unwrap();

        // 黑名单命令
        let result = sandbox.check_command("rm -rf /", &ws);
        assert!(!result.allowed);
    }

    #[test]
    fn test_check_command_confirm() {
        let sandbox = create_sandbox();
        let ws = worktree();

        sandbox.set_trust(&ws, TrustLevel::Trusted).unwrap();

        // 需确认命令
        let result = sandbox.check_command("git push origin main", &ws);
        assert!(result.allowed);
        assert!(result.require_confirm);
    }

    #[test]
    fn test_check_command_uses_actual_shell_semantics() {
        let sandbox = create_sandbox();
        let ws = worktree();

        sandbox.set_trust(&ws, TrustLevel::Trusted).unwrap();

        let powershell = sandbox.check_command_for_shell(
            "Get-ChildItem -Recurse",
            &ws,
            CommandShell::PowerShell,
        );
        assert!(powershell.allowed);
        assert!(!powershell.require_confirm);

        let cmd = sandbox.check_command_for_shell("Get-ChildItem -Recurse", &ws, CommandShell::Cmd);
        assert!(cmd.allowed);
        assert!(cmd.require_confirm);
    }

    #[test]
    fn test_check_command_untrusted_worktree() {
        let sandbox = create_sandbox();
        let ws = worktree();

        sandbox.set_trust(&ws, TrustLevel::Untrusted).unwrap();

        // 不信任worktree中，所有命令被拒绝
        let result = sandbox.check_command("git status", &ws);
        assert!(!result.allowed);
    }

    // ======================================================================
    // 网络校验测试
    // ======================================================================

    #[test]
    fn test_check_network_suggest_mode() {
        let sandbox = create_sandbox();

        // Suggest 模式：所有网络请求需要确认
        let result = sandbox.check_network("https://crates.io/api/v1");
        assert!(result.allowed);
        assert!(result.require_confirm);
    }

    #[test]
    fn test_check_network_default_allow_all() {
        let sandbox = create_sandbox();

        sandbox.set_approval_mode(ApprovalMode::FullAuto).unwrap();

        // 默认策略：所有域名都放行（包括公网、内网、localhost）
        let result = sandbox.check_network("https://crates.io/api/v1");
        assert!(result.allowed);
        assert!(!result.require_confirm);

        let result = sandbox.check_network("http://localhost:3000/api");
        assert!(result.allowed);

        let result = sandbox.check_network("http://192.168.1.1/api");
        assert!(result.allowed);

        let result = sandbox.check_network("http://10.0.0.1/api");
        assert!(result.allowed);
    }

    #[test]
    fn test_check_network_blocked_domain() {
        let sandbox = create_sandbox();

        sandbox.set_approval_mode(ApprovalMode::AutoEdit).unwrap();
        let policy = NetworkPolicy::with_blocked(vec!["evil.com".to_string()]);
        sandbox.set_network_policy(policy).unwrap();

        // 黑名单域名被拦截
        let result = sandbox.check_network("https://evil.com/api");
        assert!(!result.allowed);
        assert!(result.reason.as_ref().unwrap().contains("黑名单"));

        // 子域名也被拦截
        let result = sandbox.check_network("https://sub.evil.com/api");
        assert!(!result.allowed);

        // 非黑名单域名放行
        let result = sandbox.check_network("https://crates.io/api/v1");
        assert!(result.allowed);
    }

    #[test]
    fn test_check_network_localhost_always_allowed() {
        let sandbox = create_sandbox();

        sandbox.set_approval_mode(ApprovalMode::FullAuto).unwrap();

        // localhost 默认放行
        let result = sandbox.check_network("http://localhost:3000/api");
        assert!(result.allowed);

        let result = sandbox.check_network("http://127.0.0.1:11434/api");
        assert!(result.allowed);
    }

    // ======================================================================
    // 统一入口测试
    // ======================================================================

    #[test]
    fn test_check_unified_file_read() {
        let sandbox = create_sandbox();
        let ws = worktree();

        sandbox.set_trust(&ws, TrustLevel::Trusted).unwrap();

        let request = OperationRequest::new(
            OperationType::FileRead,
            "/home/user/worktree/src/main.rs",
            "agent",
        )
        .with_worktree("/home/user/worktree");

        let result = sandbox.check(&request).unwrap();
        assert!(result.allowed);
        // 近期审计视图有记录
        assert!(sandbox.audit_view_count() > 0);
    }

    #[test]
    fn test_check_unified_command() {
        let sandbox = create_sandbox();
        let ws = worktree();

        sandbox.set_trust(&ws, TrustLevel::Trusted).unwrap();

        let request = OperationRequest::new(OperationType::CommandExecute, "git status", "agent")
            .with_worktree("/home/user/worktree");

        let result = sandbox.check(&request).unwrap();
        assert!(result.allowed);
    }

    #[test]
    fn test_check_unified_network() {
        let sandbox = create_sandbox();

        let request =
            OperationRequest::new(OperationType::NetworkRequest, "https://crates.io", "agent");

        let result = sandbox.check(&request).unwrap();
        // Suggest 模式下需要确认
        assert!(result.require_confirm);
    }

    // ======================================================================
    // worktree 信任测试
    // ======================================================================

    #[test]
    fn test_trust_roundtrip() {
        let sandbox = create_sandbox();
        let ws = worktree();

        assert!(sandbox.get_trust(&ws).is_none());

        sandbox.set_trust(&ws, TrustLevel::Trusted).unwrap();
        assert_eq!(sandbox.get_trust(&ws), Some(TrustLevel::Trusted));

        sandbox.set_trust(&ws, TrustLevel::Untrusted).unwrap();
        assert_eq!(sandbox.get_trust(&ws), Some(TrustLevel::Untrusted));
    }

    // ======================================================================
    // 网络策略测试
    // ======================================================================

    #[test]
    fn test_network_policy_roundtrip() {
        let sandbox = create_sandbox();

        assert!(sandbox.get_network_policy().blocked_domains.is_empty());

        let policy = NetworkPolicy::with_blocked(vec!["evil.com".to_string()]);
        sandbox.set_network_policy(policy).unwrap();
        assert_eq!(
            sandbox.get_network_policy().blocked_domains,
            vec!["evil.com"]
        );
    }

    // ======================================================================
    // 命令规则管理测试
    // ======================================================================

    #[test]
    fn test_get_command_rules() {
        let sandbox = create_sandbox();
        let rules = sandbox.get_command_rules();
        assert_eq!(rules.len(), 9); // 默认有 9 条规则
    }

    #[test]
    fn test_set_command_rules() {
        let sandbox = create_sandbox();

        let new_rules = vec![CommandRule {
            pattern: r"^custom".to_string(),
            action: super::super::command_rules::RuleAction::Allow,
            description: "自定义规则".to_string(),
        }];

        sandbox.set_command_rules(new_rules).unwrap();

        let rules = sandbox.get_command_rules();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].description, "自定义规则");
    }

    // ======================================================================
    // 资源限制测试
    // ======================================================================

    #[test]
    fn test_resource_limit_roundtrip() {
        let sandbox = create_sandbox();

        let limits = sandbox.list_resource_limits();
        assert_eq!(limits.len(), 5); // 5 种默认资源

        // 更新 CPU 用量
        sandbox.update_resource_usage(ResourceType::Cpu, 50.0);
        let usage = sandbox.get_resource_usage(ResourceType::Cpu);
        assert!((usage.usage_percent - 62.5).abs() < f64::EPSILON); // 50/80 * 100
    }

    #[test]
    fn test_set_resource_limit() {
        let sandbox = create_sandbox();

        let limit = ResourceLimit::new(ResourceType::Cpu, 95.0);
        sandbox
            .set_resource_limit(ResourceType::Cpu, limit)
            .unwrap();

        let limits = sandbox.list_resource_limits();
        let cpu_limit = limits
            .iter()
            .find(|(rt, _, _)| *rt == ResourceType::Cpu)
            .unwrap();
        assert_eq!(cpu_limit.1.max_value, 95.0);
    }

    // ======================================================================
    // 近期审计视图测试
    // ======================================================================

    #[test]
    fn test_audit_log_after_check() {
        let sandbox = create_sandbox();
        let ws = worktree();

        sandbox.set_trust(&ws, TrustLevel::Trusted).unwrap();

        let request = OperationRequest::new(
            OperationType::FileRead,
            "/home/user/worktree/src/main.rs",
            "agent",
        )
        .with_worktree("/home/user/worktree");

        let _ = sandbox.check(&request).unwrap();

        let audit_entries = sandbox.get_recent_audit_view(SandboxAuditViewFilter::new());
        assert!(!audit_entries.is_empty());
        assert_eq!(audit_entries[0].operation, OperationType::FileRead);
    }

    // ======================================================================
    // 会话信任测试
    // ======================================================================

    #[test]
    fn test_clear_session_trust() {
        let sandbox = create_sandbox();
        let ws = worktree();

        sandbox.set_trust(&ws, TrustLevel::SessionScoped).unwrap();
        assert_eq!(sandbox.get_trust(&ws), Some(TrustLevel::SessionScoped));

        sandbox.clear_session_trust();
        assert!(sandbox.get_trust(&ws).is_none());
    }
}
