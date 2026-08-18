//! Sandbox Constraint 适配层
//!
//! 将 Sandbox 的 check() 逻辑封装为 kernel::PolicyEngine 的 Constraint 实现，
//! 使 Sandbox 的安全检查可以作为 PolicyEngine 管道中的标准约束节点运行。
//!
//! # 约束映射
//!
//! | Sandbox CheckResult              | PolicyDecision            |
//! |----------------------------------|---------------------------|
//! | allowed=true, require_confirm=false | Allow                    |
//! | allowed=true, require_confirm=true  | Ask (需要用户批准)       |
//! | allowed=false                      | Deny                     |
//!
//! # action 前缀约定
//!
//! | 前缀           | 对应 Sandbox 操作类型          |
//! |----------------|-------------------------------|
//! | `tool.file.*`  | FileRead / FileWrite / FileDelete / DirCreate / DirDelete |
//! | `tool.command`  | CommandExecute               |
//! | `tool.network`  | NetworkRequest               |

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use serde_json::json;

use crate::kernel::{Constraint, PolicyDecision, PolicyEngine, PolicyInput};

use super::command_rules::CommandShell;
use super::gate::Sandbox;
use super::permission::{CheckResult, OperationRequest, OperationType};

// ============================================================================
// 辅助函数：CheckResult → PolicyDecision
// ============================================================================

fn check_result_to_decision(result: CheckResult) -> PolicyDecision {
    if !result.allowed {
        PolicyDecision::Deny {
            reason: result
                .reason
                .unwrap_or_else(|| "操作被沙箱策略拒绝".to_string()),
        }
    } else if result.require_confirm {
        PolicyDecision::Ask {
            prompt: result
                .confirm_message
                .unwrap_or_else(|| "操作需要用户确认".to_string()),
            grant_spec: json!({ "level": result.level.to_string() }),
        }
    } else {
        PolicyDecision::Allow {
            reason: format!("沙箱策略允许 (level={})", result.level),
        }
    }
}

fn decision_to_check_result(decision: PolicyDecision) -> CheckResult {
    match decision {
        PolicyDecision::Allow { reason: _ } => {
            CheckResult::allowed(super::permission::PermissionLevel::LightCheck)
        }
        PolicyDecision::Ask { prompt, .. } => {
            CheckResult::needs_confirm(super::permission::PermissionLevel::UserConfirm, prompt)
        }
        PolicyDecision::Deny { reason } => {
            CheckResult::denied(super::permission::PermissionLevel::UserConfirm, reason)
        }
    }
}

fn action_for_operation(operation: &OperationType) -> &'static str {
    match operation {
        OperationType::FileRead => "tool.file.read",
        OperationType::FileWrite => "tool.file.write",
        OperationType::FileDelete => "tool.file.delete",
        OperationType::DirCreate => "tool.dir.create",
        OperationType::DirDelete => "tool.dir.delete",
        OperationType::CommandExecute => "tool.command",
        OperationType::NetworkRequest => "tool.network",
    }
}

fn policy_input_from_request(request: &OperationRequest) -> PolicyInput {
    PolicyInput {
        subject: request.actor.clone(),
        action: action_for_operation(&request.operation).to_string(),
        target: request.target.clone(),
        scope: request
            .session_id
            .clone()
            .unwrap_or_else(|| "global".to_string()),
        metadata: json!({
            "worktree_root": request.worktree_path,
            "operation": request.operation.to_string(),
        }),
    }
}

/// 对路径做不依赖文件系统的词法规范化。
///
/// 规范化失败表示路径已经回退到了绝对路径的根目录之外，调用方必须拒绝请求。
fn normalize_path_lexically(path: &Path) -> Option<PathBuf> {
    let mut prefix = None::<OsString>;
    let mut has_root = false;
    let mut parts = Vec::<OsString>::new();

    for component in path.components() {
        match component {
            Component::Prefix(value) => prefix = Some(value.as_os_str().to_os_string()),
            Component::RootDir => has_root = true,
            Component::CurDir => {}
            Component::Normal(value) => parts.push(value.to_os_string()),
            Component::ParentDir => {
                if parts.pop().is_none() {
                    if has_root {
                        return None;
                    }
                    // 相对路径尚未绑定到 worktree 时保留前导 ..，由边界检查统一拒绝。
                    parts.push(OsString::from(".."));
                }
            }
        }
    }

    let mut normalized = PathBuf::new();
    if let Some(prefix) = prefix {
        normalized.push(prefix);
    }
    if has_root {
        normalized.push(std::path::MAIN_SEPARATOR.to_string());
    }
    for part in parts {
        normalized.push(part);
    }

    if normalized.as_os_str().is_empty() {
        normalized.push(".");
    }

    Some(normalized)
}

/// 解析文件目标并强制限制在 worktree 内，避免相对路径通过 .. 逃逸。
fn resolve_file_target(target: &str, worktree: &str) -> Result<PathBuf, String> {
    let worktree_path = normalize_path_lexically(Path::new(worktree))
        .ok_or_else(|| "worktree 路径无效，已回退到根目录之外".to_string())?;
    let target = target.trim();
    let target_path = Path::new(if target.is_empty() { "." } else { target });

    let candidate = if target_path.is_absolute() || target_path.has_root() {
        normalize_path_lexically(target_path)
            .ok_or_else(|| "目标路径无效，已回退到根目录之外".to_string())?
    } else {
        normalize_path_lexically(&worktree_path.join(target_path))
            .ok_or_else(|| "目标路径无效，已回退到根目录之外".to_string())?
    };

    if !candidate.starts_with(&worktree_path) {
        return Err(format!("目标路径逃逸 worktree，已拒绝: {}", target));
    }

    Ok(candidate)
}

/// 从 PolicyInput.target 提取路径和 worktree，构造已限制在 worktree 内的 OperationRequest。
fn build_file_request(
    input: &PolicyInput,
    operation: OperationType,
) -> Result<OperationRequest, String> {
    let worktree = input
        .metadata
        .get("worktree_root")
        .and_then(|v| v.as_str())
        .unwrap_or("/");
    let target = resolve_file_target(&input.target, worktree)?;

    let request = OperationRequest::new(operation, target.to_string_lossy(), &input.subject)
        .with_worktree(worktree)
        .with_session_id(&input.scope);
    Ok(request)
}

/// 从 PolicyInput 提取命令字符串，构造 CommandExecute 请求
fn build_command_request(input: &PolicyInput) -> OperationRequest {
    let worktree = input
        .metadata
        .get("worktree_root")
        .and_then(|v| v.as_str())
        .unwrap_or("/");

    OperationRequest::new(OperationType::CommandExecute, &input.target, &input.subject)
        .with_worktree(worktree)
        .with_session_id(&input.scope)
}

/// 从 PolicyInput 构造 NetworkRequest 请求
fn build_network_request(input: &PolicyInput) -> OperationRequest {
    OperationRequest::new(OperationType::NetworkRequest, &input.target, &input.subject)
        .with_session_id(&input.scope)
}

/// 工具元数据提示（参数化后自包含，不依赖 tool 域类型）。
///
/// 工具调用者在 metadata.tool_metadata 中以 JSON 携带两个 opaque 布尔标志，
/// 约束层据此做破坏性升级或网络检查。sandbox 只读取标志本身，不解析
/// tool 域的完整 ToolMetadata 结构——业务约束类型由扩展注册。
#[derive(Default)]
struct SandboxToolHints {
    /// 是否破坏性操作（删除/覆盖/批量）。
    is_destructive: bool,
    /// 是否需要网络（命令工具绕过网络约束时使用）。
    requires_network: bool,
}

/// 从 PolicyInput.metadata 中提取工具元数据标志（如果有）。
fn extract_tool_hints(input: &PolicyInput) -> Option<SandboxToolHints> {
    let value = input.metadata.get("tool_metadata")?;
    #[derive(serde::Deserialize)]
    struct RawHints {
        #[serde(default)]
        is_destructive: bool,
        #[serde(default)]
        requires_network: bool,
    }
    serde_json::from_value::<RawHints>(value.clone())
        .ok()
        .map(|raw| SandboxToolHints {
            is_destructive: raw.is_destructive,
            requires_network: raw.requires_network,
        })
}

// ============================================================================
// PathAccessConstraint
// ============================================================================

/// 路径访问约束
///
/// 将 Sandbox 的路径白名单/黑名单检查封装为 Constraint。
/// 匹配 action 前缀 `tool.file.*`。
pub struct PathAccessConstraint {
    sandbox: Arc<Sandbox>,
}

impl PathAccessConstraint {
    pub fn new(sandbox: Arc<Sandbox>) -> Self {
        Self { sandbox }
    }
}

impl Constraint for PathAccessConstraint {
    fn id(&self) -> &str {
        "sandbox.path"
    }

    fn priority(&self) -> i32 {
        10
    }

    fn evaluate(&self, input: &PolicyInput) -> Option<PolicyDecision> {
        // 只拦截文件相关操作
        let operation = match input.action.as_str() {
            "tool.file.read" => OperationType::FileRead,
            "tool.file.write" => OperationType::FileWrite,
            "tool.file.delete" => OperationType::FileDelete,
            "tool.dir.create" => OperationType::DirCreate,
            "tool.dir.delete" => OperationType::DirDelete,
            _ => return None, // 不适用
        };

        let request = match build_file_request(input, operation) {
            Ok(request) => request,
            Err(reason) => return Some(PolicyDecision::Deny { reason }),
        };

        match self.sandbox.check(&request) {
            Ok(result) => {
                let mut decision = check_result_to_decision(result);
                // 破坏性工具升级：即使沙箱允许，也升级为 Ask 需要用户确认
                if let Some(meta) = extract_tool_hints(input) {
                    if meta.is_destructive {
                        if let PolicyDecision::Allow { reason } = decision {
                            decision = PolicyDecision::Ask {
                                prompt: format!("工具将执行破坏性文件操作: {}", reason),
                                grant_spec: json!({ "level": "user_confirm", "reason": "destructive_tool" }),
                            };
                        }
                    }
                }
                Some(decision)
            }
            Err(e) => {
                tracing::error!(error = %e, "sandbox path check failed");
                Some(PolicyDecision::Deny {
                    reason: format!("沙箱路径检查异常: {}", e),
                })
            }
        }
    }
}

// ============================================================================
// CommandConstraint
// ============================================================================

/// 命令执行约束
///
/// 将 Sandbox 的命令黑白名单检查封装为 Constraint。
/// 匹配 action 前缀 `tool.command`。
///
/// 额外行为：如果工具元数据标记 `requires_network: true`，
/// 额外调用 Sandbox 网络检查，避免命令工具绕过网络约束。
pub struct CommandConstraint {
    sandbox: Arc<Sandbox>,
}

impl CommandConstraint {
    pub fn new(sandbox: Arc<Sandbox>) -> Self {
        Self { sandbox }
    }
}

impl Constraint for CommandConstraint {
    fn id(&self) -> &str {
        "sandbox.command"
    }

    fn priority(&self) -> i32 {
        10
    }

    fn evaluate(&self, input: &PolicyInput) -> Option<PolicyDecision> {
        if input.action != "tool.command" {
            return None;
        }

        let request = build_command_request(input);

        let command_decision = match self.sandbox.check(&request) {
            Ok(result) => check_result_to_decision(result),
            Err(e) => {
                tracing::error!(error = %e, "sandbox command check failed");
                return Some(PolicyDecision::Deny {
                    reason: format!("沙箱命令检查异常: {}", e),
                });
            }
        };

        // 如果命令检查结果是 Deny，直接拒绝
        if matches!(command_decision, PolicyDecision::Deny { .. }) {
            return Some(command_decision);
        }

        // 工具元数据要求网络：额外检查网络约束，避免命令工具绕过域名黑名单
        if let Some(meta) = extract_tool_hints(input) {
            if meta.requires_network {
                let network_request = build_network_request(input);
                let network_decision = match self.sandbox.check(&network_request) {
                    Ok(result) => check_result_to_decision(result),
                    Err(e) => {
                        tracing::warn!(error = %e, "sandbox network check for command tool failed");
                        PolicyDecision::Allow {
                            reason: format!("网络检查跳过 (异常: {})", e),
                        }
                    }
                };
                // 取两个决策中更严格的一个
                return Some(more_restrictive(command_decision, network_decision));
            }
        }

        Some(command_decision)
    }
}

/// 返回两个 PolicyDecision 中更严格的那个。
///
/// 优先级：Deny > Ask > Allow
fn more_restrictive(a: PolicyDecision, b: PolicyDecision) -> PolicyDecision {
    match (&a, &b) {
        (PolicyDecision::Deny { .. }, _) | (_, PolicyDecision::Deny { .. }) => {
            // 任意一个 Deny 就返回 Deny，优先取有具体原因的
            match (&a, &b) {
                (PolicyDecision::Deny { reason }, _) | (_, PolicyDecision::Deny { reason }) => {
                    PolicyDecision::Deny {
                        reason: reason.clone(),
                    }
                }
                _ => unreachable!(),
            }
        }
        (PolicyDecision::Ask { .. }, _) => a,
        (_, PolicyDecision::Ask { .. }) => b,
        _ => a,
    }
}

struct ShellCommandConstraint {
    sandbox: Arc<Sandbox>,
    shell: CommandShell,
}

impl ShellCommandConstraint {
    fn new(sandbox: Arc<Sandbox>, shell: CommandShell) -> Self {
        Self { sandbox, shell }
    }
}

impl Constraint for ShellCommandConstraint {
    fn id(&self) -> &str {
        "sandbox.command.shell"
    }

    fn priority(&self) -> i32 {
        10
    }

    fn evaluate(&self, input: &PolicyInput) -> Option<PolicyDecision> {
        if input.action != "tool.command" {
            return None;
        }

        let worktree = input
            .metadata
            .get("worktree_root")
            .and_then(|v| v.as_str())
            .unwrap_or("/");

        let result = self.sandbox.check_command_for_shell(
            &input.target,
            std::path::Path::new(worktree),
            self.shell,
        );
        Some(check_result_to_decision(result))
    }
}

// ============================================================================
// NetworkConstraint
// ============================================================================

/// 网络访问约束
///
/// 将 Sandbox 的网络域名黑名单检查封装为 Constraint。
/// 匹配 action 前缀 `tool.network`。
pub struct NetworkConstraint {
    sandbox: Arc<Sandbox>,
}

impl NetworkConstraint {
    pub fn new(sandbox: Arc<Sandbox>) -> Self {
        Self { sandbox }
    }
}

impl Constraint for NetworkConstraint {
    fn id(&self) -> &str {
        "sandbox.network"
    }

    fn priority(&self) -> i32 {
        10
    }

    fn evaluate(&self, input: &PolicyInput) -> Option<PolicyDecision> {
        if input.action != "tool.network" {
            return None;
        }

        let request = build_network_request(input);

        match self.sandbox.check(&request) {
            Ok(result) => Some(check_result_to_decision(result)),
            Err(e) => {
                tracing::error!(error = %e, "sandbox network check failed");
                Some(PolicyDecision::Deny {
                    reason: format!("沙箱网络检查异常: {}", e),
                })
            }
        }
    }
}

// ============================================================================
// 注册辅助函数
// ============================================================================

/// 将 Sandbox 的所有约束注册到 PolicyEngine
pub fn register_sandbox_constraints(
    policy: &crate::kernel::PolicyEngine,
    sandbox: Arc<Sandbox>,
) -> crate::kernel::KernelResult<()> {
    policy.add(PathAccessConstraint::new(Arc::clone(&sandbox)))?;
    policy.add(CommandConstraint::new(Arc::clone(&sandbox)))?;
    policy.add(NetworkConstraint::new(sandbox))?;
    tracing::info!("registered sandbox constraints into PolicyEngine");
    Ok(())
}

/// Ensure Sandbox constraints are present on a shared PolicyEngine.
///
/// Shared engines may be reused across many tool calls. This helper is
/// intentionally idempotent so hot paths can depend on one Kernel PolicyEngine
/// without tripping duplicate constraint registration.
pub fn ensure_sandbox_constraints(
    policy: &crate::kernel::PolicyEngine,
    sandbox: Arc<Sandbox>,
) -> crate::kernel::KernelResult<()> {
    if !policy.contains("sandbox.path") {
        policy.replace(PathAccessConstraint::new(Arc::clone(&sandbox)))?;
    }
    if !policy.contains("sandbox.command") {
        policy.replace(CommandConstraint::new(Arc::clone(&sandbox)))?;
    }
    if !policy.contains("sandbox.network") {
        policy.replace(NetworkConstraint::new(sandbox))?;
    }
    Ok(())
}

/// Evaluate an existing Sandbox operation DTO through an externally-provided PolicyEngine.
///
/// This variant accepts a pre-configured `PolicyEngine` so that callers who already
/// hold a shared engine (e.g. injected at application startup) can evaluate sandbox
/// operations without creating a throwaway engine on every call.
pub fn evaluate_operation_request_with_engine(
    engine: &PolicyEngine,
    sandbox: Arc<Sandbox>,
    request: &OperationRequest,
) -> crate::kernel::KernelResult<CheckResult> {
    ensure_sandbox_constraints(engine, sandbox)?;
    Ok(decision_to_check_result(
        engine.evaluate(&policy_input_from_request(request)),
    ))
}

/// Evaluate a shell-aware terminal command through an externally-provided PolicyEngine.
///
/// This variant accepts a pre-configured `PolicyEngine` so that callers who already
/// hold a shared engine can evaluate shell command requests without creating a
/// throwaway engine on every call.
pub fn evaluate_command_request_for_shell_with_engine(
    engine: &PolicyEngine,
    sandbox: Arc<Sandbox>,
    request: &OperationRequest,
    shell: CommandShell,
) -> crate::kernel::KernelResult<CheckResult> {
    engine.replace(ShellCommandConstraint::new(sandbox, shell))?;
    Ok(decision_to_check_result(
        engine.evaluate(&policy_input_from_request(request)),
    ))
}

/// Evaluate an existing Sandbox operation through an externally-provided PolicyEngine.
///
/// Accepts a pre-configured `PolicyEngine` reference so callers who already hold a
/// shared engine can evaluate sandbox operations without creating a throwaway engine.
pub fn evaluate_operation_request(
    engine: &PolicyEngine,
    sandbox: Arc<Sandbox>,
    request: &OperationRequest,
) -> crate::kernel::KernelResult<CheckResult> {
    evaluate_operation_request_with_engine(engine, sandbox, request)
}

/// Evaluate a shell-aware terminal command through an externally-provided PolicyEngine.
///
/// Accepts a pre-configured `PolicyEngine` reference so callers who already hold a
/// shared engine can evaluate shell command requests without creating a throwaway engine.
pub fn evaluate_command_request_for_shell(
    engine: &PolicyEngine,
    sandbox: Arc<Sandbox>,
    request: &OperationRequest,
    shell: CommandShell,
) -> crate::kernel::KernelResult<CheckResult> {
    evaluate_command_request_for_shell_with_engine(engine, sandbox, request, shell)
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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

    fn setup() -> (Arc<Sandbox>, PolicyEngine) {
        let event_bus: Arc<dyn crate::kernel::EventBus> = test_event_bus();
        let sandbox = Arc::new(Sandbox::new(event_bus));
        let policy = PolicyEngine::new();
        register_sandbox_constraints(&policy, Arc::clone(&sandbox)).unwrap();
        (sandbox, policy)
    }

    fn file_read_input(target: &str) -> PolicyInput {
        PolicyInput {
            subject: "agent".into(),
            action: "tool.file.read".into(),
            target: target.into(),
            scope: "session_001".into(),
            metadata: json!({ "worktree_root": "/home/user/worktree" }),
        }
    }

    fn command_input(command: &str) -> PolicyInput {
        PolicyInput {
            subject: "agent".into(),
            action: "tool.command".into(),
            target: command.into(),
            scope: "session_001".into(),
            metadata: json!({ "worktree_root": "/home/user/worktree" }),
        }
    }

    fn network_input(url: &str) -> PolicyInput {
        PolicyInput {
            subject: "agent".into(),
            action: "tool.network".into(),
            target: url.into(),
            scope: "session_001".into(),
            metadata: json!({}),
        }
    }

    fn file_read_input_with_worktree(target: &str, worktree: &std::path::Path) -> PolicyInput {
        PolicyInput {
            subject: "agent".into(),
            action: "tool.file.read".into(),
            target: target.into(),
            scope: "session_001".into(),
            metadata: json!({ "worktree_root": worktree.to_string_lossy() }),
        }
    }

    #[test]
    fn path_read_allowed_in_suggest_mode() {
        let (sandbox, policy) = setup();
        // 设置worktree为信任
        let ws = std::path::Path::new("/home/user/worktree");
        sandbox
            .set_trust(ws, super::super::worktree_trust::TrustLevel::Trusted)
            .unwrap();

        let input = file_read_input("/home/user/worktree/src/main.rs");
        let decision = policy.evaluate(&input);
        assert!(
            matches!(decision, PolicyDecision::Allow { .. }),
            "expected Allow, got {:?}",
            decision
        );
    }

    #[test]
    fn relative_file_target_resolves_against_worktree() {
        let (sandbox, policy) = setup();
        let ws = std::env::temp_dir().join("navis-policy-worktree");
        sandbox
            .set_trust(&ws, super::super::worktree_trust::TrustLevel::Trusted)
            .unwrap();

        for target in [".", "", "src/main.rs"] {
            let input = file_read_input_with_worktree(target, &ws);
            let decision = policy.evaluate(&input);
            assert!(
                matches!(decision, PolicyDecision::Allow { .. }),
                "expected Allow for target {target:?}, got {:?}",
                decision
            );
        }
    }

    #[test]
    fn relative_file_target_cannot_escape_worktree() {
        let (sandbox, policy) = setup();
        let ws = std::env::temp_dir().join("navis-policy-worktree");
        sandbox
            .set_trust(&ws, super::super::worktree_trust::TrustLevel::Trusted)
            .unwrap();

        let input = file_read_input_with_worktree("../outside.txt", &ws);
        let decision = policy.evaluate(&input);
        assert!(
            matches!(
                decision,
                PolicyDecision::Ask { .. } | PolicyDecision::Deny { .. }
            ),
            "expected Ask or Deny for worktree escape, got {:?}",
            decision
        );
    }

    #[test]
    fn path_write_needs_confirm_in_suggest_mode() {
        let (sandbox, policy) = setup();
        let ws = std::path::Path::new("/home/user/worktree");
        sandbox
            .set_trust(ws, super::super::worktree_trust::TrustLevel::Trusted)
            .unwrap();

        let input = PolicyInput {
            subject: "agent".into(),
            action: "tool.file.write".into(),
            target: "/home/user/worktree/src/main.rs".into(),
            scope: "session_001".into(),
            metadata: json!({ "worktree_root": "/home/user/worktree" }),
        };
        let decision = policy.evaluate(&input);
        assert!(
            matches!(decision, PolicyDecision::Ask { .. }),
            "expected Ask, got {:?}",
            decision
        );
    }

    #[test]
    fn command_blocked_in_untrusted_worktree() {
        let (sandbox, policy) = setup();
        sandbox
            .set_approval_mode(super::super::ApprovalMode::FullAuto)
            .unwrap();
        let ws = std::path::Path::new("/home/user/worktree");
        sandbox
            .set_trust(ws, super::super::worktree_trust::TrustLevel::Untrusted)
            .unwrap();

        let input = command_input("rm -rf /");
        let decision = policy.evaluate(&input);
        assert!(
            matches!(decision, PolicyDecision::Deny { .. }),
            "expected Deny, got {:?}",
            decision
        );
    }

    #[test]
    fn network_suggest_mode_asks() {
        let (_sandbox, policy) = setup();

        let input = network_input("https://crates.io/api/v1");
        let decision = policy.evaluate(&input);
        assert!(
            matches!(decision, PolicyDecision::Ask { .. }),
            "expected Ask, got {:?}",
            decision
        );
    }

    #[test]
    fn unrelated_action_returns_none() {
        let (_sandbox, policy) = setup();

        let input = PolicyInput {
            subject: "agent".into(),
            action: "capability.register".into(),
            target: "something".into(),
            scope: "global".into(),
            metadata: json!({}),
        };
        // Sandbox 约束不匹配，PolicyEngine 默认 deny
        let decision = policy.evaluate(&input);
        assert!(
            matches!(decision, PolicyDecision::Deny { .. }),
            "expected Deny (no constraint matched), got {:?}",
            decision
        );
    }

    #[test]
    fn register_sandbox_constraints_count() {
        let (_sandbox, policy) = setup();
        assert_eq!(policy.len(), 3);
    }

    #[test]
    fn constraint_ids() {
        let sandbox = Arc::new(Sandbox::new(test_event_bus()));

        let path_c = PathAccessConstraint {
            sandbox: Arc::clone(&sandbox),
        };
        assert_eq!(path_c.id(), "sandbox.path");
        assert_eq!(path_c.priority(), 10);

        let cmd_c = CommandConstraint {
            sandbox: Arc::clone(&sandbox),
        };
        assert_eq!(cmd_c.id(), "sandbox.command");

        let net_c = NetworkConstraint { sandbox };
        assert_eq!(net_c.id(), "sandbox.network");
    }

    /// 验证全局策略引擎可以组合沙箱约束与通用扩展运行时约束。
    #[test]
    fn global_policy_engine_composes_sandbox_and_host_constraints() {
        struct HostDefaultAllowConstraint;

        impl Constraint for HostDefaultAllowConstraint {
            fn id(&self) -> &str {
                "host.default-allow"
            }

            fn priority(&self) -> i32 {
                100
            }

            fn evaluate(&self, input: &PolicyInput) -> Option<PolicyDecision> {
                input.action.starts_with("tool.").then(|| PolicyDecision::Allow {
                    reason: "通用宿主默认放行未匹配工具动作".to_string(),
                })
            }
        }

        struct ExtensionRuntimeDenyConstraint;

        impl Constraint for ExtensionRuntimeDenyConstraint {
            fn id(&self) -> &str {
                "extension.runtime-deny"
            }

            fn priority(&self) -> i32 {
                90
            }

            fn evaluate(&self, input: &PolicyInput) -> Option<PolicyDecision> {
                (input.action == "extension.runtime.check").then(|| PolicyDecision::Deny {
                    reason: "扩展运行时策略拒绝该动作".to_string(),
                })
            }
        }

        let engine = PolicyEngine::new();
        let sandbox = Arc::new(Sandbox::new(test_event_bus()));
        let worktree = std::path::Path::new("/home/user/worktree");
        sandbox
            .set_trust(worktree, super::super::worktree_trust::TrustLevel::Trusted)
            .unwrap();
        register_sandbox_constraints(&engine, Arc::clone(&sandbox)).unwrap();
        assert_eq!(engine.len(), 3);

        engine.add(HostDefaultAllowConstraint).unwrap();
        engine.add(ExtensionRuntimeDenyConstraint).unwrap();
        assert_eq!(engine.len(), 5);

        let file_input = PolicyInput {
            subject: "extension-runtime".into(),
            action: "tool.file.read".into(),
            target: "/home/user/worktree/src/main.rs".into(),
            scope: "global".into(),
            metadata: json!({ "worktree_root": "/home/user/worktree" }),
        };
        assert!(matches!(engine.evaluate(&file_input), PolicyDecision::Allow { .. }));

        let runtime_input = PolicyInput {
            subject: "extension-runtime".into(),
            action: "extension.runtime.check".into(),
            target: "extension.example".into(),
            scope: "global".into(),
            metadata: json!({}),
        };
        assert!(matches!(
            engine.evaluate(&runtime_input),
            PolicyDecision::Deny { ref reason } if reason.contains("扩展运行时策略")
        ));

        let unmatched_tool_input = PolicyInput {
            subject: "extension-runtime".into(),
            action: "tool.custom_action".into(),
            target: "resource".into(),
            scope: "global".into(),
            metadata: json!({}),
        };
        assert!(matches!(
            engine.evaluate(&unmatched_tool_input),
            PolicyDecision::Allow { .. }
        ));
    }
}
