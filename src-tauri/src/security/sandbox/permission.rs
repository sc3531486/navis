//! 操作权限分级与审批模式
//!
//! 基于设计文档 §3 实现，定义操作分级、操作请求、校验结果、审批模式。
//!
//! # 数据模型
//! - `PermissionLevel` - 操作分级 (Level 0-3)
//! - `ApprovalMode` - 审批模式 (Suggest / AutoEdit / FullAuto)
//! - `OperationType` - 操作类型
//! - `OperationRequest` - 操作请求
//! - `CheckResult` - 校验结果

use serde::{Deserialize, Serialize};

// ============================================================================
// 操作分级
// ============================================================================

/// 操作分级
///
/// 对标设计文档中的四级权限分级模型：
/// - Level 0: 无限制（只读、内存操作）
/// - Level 1: 轻量校验（文件读取、目录遍历）
/// - Level 2: 严格校验（文件写入、命令执行）
/// - Level 3: 用户确认（删除、系统命令、网络请求）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PermissionLevel {
    /// Level 0: 无限制（只读、内存操作）
    Unrestricted,
    /// Level 1: 轻量校验（文件读取、目录遍历）
    LightCheck,
    /// Level 2: 严格校验（文件写入、命令执行）
    StrictCheck,
    /// Level 3: 用户确认（删除、系统命令、网络请求）
    UserConfirm,
}

impl PermissionLevel {
    /// 获取数字等级
    pub fn as_u8(&self) -> u8 {
        match self {
            PermissionLevel::Unrestricted => 0,
            PermissionLevel::LightCheck => 1,
            PermissionLevel::StrictCheck => 2,
            PermissionLevel::UserConfirm => 3,
        }
    }

    /// 从数字等级创建
    pub fn from_u8(level: u8) -> Self {
        match level {
            0 => PermissionLevel::Unrestricted,
            1 => PermissionLevel::LightCheck,
            2 => PermissionLevel::StrictCheck,
            _ => PermissionLevel::UserConfirm,
        }
    }

    /// 判断当前级别是否高于指定级别
    pub fn is_higher_than(&self, other: &PermissionLevel) -> bool {
        self.as_u8() > other.as_u8()
    }
}

impl std::fmt::Display for PermissionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionLevel::Unrestricted => write!(f, "unrestricted"),
            PermissionLevel::LightCheck => write!(f, "light_check"),
            PermissionLevel::StrictCheck => write!(f, "strict_check"),
            PermissionLevel::UserConfirm => write!(f, "user_confirm"),
        }
    }
}

// ============================================================================
// 操作类型
// ============================================================================

/// 操作类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperationType {
    /// 文件读取
    FileRead,
    /// 文件写入
    FileWrite,
    /// 文件删除
    FileDelete,
    /// 目录创建
    DirCreate,
    /// 目录删除
    DirDelete,
    /// 命令执行
    CommandExecute,
    /// 网络请求
    NetworkRequest,
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationType::FileRead => write!(f, "file_read"),
            OperationType::FileWrite => write!(f, "file_write"),
            OperationType::FileDelete => write!(f, "file_delete"),
            OperationType::DirCreate => write!(f, "dir_create"),
            OperationType::DirDelete => write!(f, "dir_delete"),
            OperationType::CommandExecute => write!(f, "command_execute"),
            OperationType::NetworkRequest => write!(f, "network_request"),
        }
    }
}

// ============================================================================
// 操作请求
// ============================================================================

/// 操作请求
///
/// 封装一次需要经过 Sandbox 校验的操作信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRequest {
    /// 操作类型
    pub operation: OperationType,
    /// 目标（文件路径 / 命令 / URL）
    pub target: String,
    /// 操作者（"user" / "agent" / "extension:xxx"）
    pub actor: String,
    /// 会话 ID
    pub session_id: Option<String>,
    /// worktree 路径
    pub worktree_path: Option<String>,
}

impl OperationRequest {
    /// 创建操作请求
    pub fn new(
        operation: OperationType,
        target: impl Into<String>,
        actor: impl Into<String>,
    ) -> Self {
        Self {
            operation,
            target: target.into(),
            actor: actor.into(),
            session_id: None,
            worktree_path: None,
        }
    }

    /// 设置会话 ID
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// 设置worktree 路径
    pub fn with_worktree(mut self, worktree: impl Into<String>) -> Self {
        self.worktree_path = Some(worktree.into());
        self
    }

    /// 是否为 Agent 操作
    pub fn is_agent(&self) -> bool {
        self.actor == "agent" || self.actor.starts_with("extension:")
    }

    /// 是否为扩展操作
    pub fn is_extension(&self) -> bool {
        self.actor.starts_with("extension:")
    }

    /// 获取扩展 ID（如果是扩展操作）
    pub fn extension_id(&self) -> Option<&str> {
        self.actor.strip_prefix("extension:")
    }
}

// ============================================================================
// 校验结果
// ============================================================================

/// 校验结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// 是否允许
    pub allowed: bool,
    /// 操作分级
    pub level: PermissionLevel,
    /// 拒绝原因（仅 allowed=false 时有值）
    pub reason: Option<String>,
    /// 是否需要用户确认
    pub require_confirm: bool,
    /// 确认提示信息（仅 require_confirm=true 时有值）
    pub confirm_message: Option<String>,
}

impl CheckResult {
    /// 创建允许结果
    pub fn allowed(level: PermissionLevel) -> Self {
        tracing::debug!(level = %level, "Check result: allowed");
        Self {
            allowed: true,
            level,
            reason: None,
            require_confirm: false,
            confirm_message: None,
        }
    }

    /// 创建拒绝结果
    pub fn denied(level: PermissionLevel, reason: impl Into<String>) -> Self {
        let reason_str = reason.into();
        tracing::debug!(level = %level, reason = %reason_str, "Check result: denied");
        Self {
            allowed: false,
            level,
            reason: Some(reason_str),
            require_confirm: false,
            confirm_message: None,
        }
    }

    /// 创建需要用户确认的结果
    pub fn needs_confirm(level: PermissionLevel, message: impl Into<String>) -> Self {
        let msg = message.into();
        tracing::debug!(level = %level, message = %msg, "Check result: needs confirm");
        Self {
            allowed: true,
            level,
            reason: None,
            require_confirm: true,
            confirm_message: Some(msg),
        }
    }
}

// ============================================================================
// 审批模式
// ============================================================================

/// 审批模式（三级审批）
///
/// - Suggest: Agent 只建议，不执行，需用户确认所有操作
/// - AutoEdit: Agent 可自动编辑文件，但执行命令需确认
/// - FullAuto: Agent 完全自主，文件编辑和命令执行都不需要确认
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApprovalMode {
    /// 建议模式
    Suggest,
    /// 自动编辑模式
    AutoEdit,
    /// 完全自动模式
    FullAuto,
}

impl ApprovalMode {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "suggest" => Some(ApprovalMode::Suggest),
            "auto-edit" => Some(ApprovalMode::AutoEdit),
            "full-auto" => Some(ApprovalMode::FullAuto),
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            ApprovalMode::Suggest => "suggest",
            ApprovalMode::AutoEdit => "auto-edit",
            ApprovalMode::FullAuto => "full-auto",
        }
    }

    /// 获取默认值
    pub fn default_mode() -> Self {
        ApprovalMode::Suggest
    }
}

impl std::fmt::Display for ApprovalMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Default for ApprovalMode {
    fn default() -> Self {
        Self::default_mode()
    }
}

// ============================================================================
// 操作类型到默认权限级别的映射
// ============================================================================

impl OperationType {
    /// 获取操作类型的默认权限级别
    pub fn default_level(&self) -> PermissionLevel {
        match self {
            OperationType::FileRead => PermissionLevel::LightCheck,
            OperationType::FileWrite => PermissionLevel::StrictCheck,
            OperationType::FileDelete => PermissionLevel::UserConfirm,
            OperationType::DirCreate => PermissionLevel::StrictCheck,
            OperationType::DirDelete => PermissionLevel::UserConfirm,
            OperationType::CommandExecute => PermissionLevel::UserConfirm,
            OperationType::NetworkRequest => PermissionLevel::UserConfirm,
        }
    }

    /// 判断该操作在指定审批模式下是否需要确认
    ///
    /// 基于设计文档 §3 中的 ApprovalMode 与操作类型联动规则表格
    pub fn requires_confirm_in_mode(&self, mode: &ApprovalMode) -> bool {
        match mode {
            ApprovalMode::Suggest => {
                // Suggest: FileRead 放行，其他都需要确认
                !matches!(self, OperationType::FileRead)
            }
            ApprovalMode::AutoEdit => {
                // AutoEdit: FileRead/FileWrite/DirCreate 放行，其他需确认
                !matches!(
                    self,
                    OperationType::FileRead | OperationType::FileWrite | OperationType::DirCreate
                )
            }
            ApprovalMode::FullAuto => {
                // FullAuto: 所有操作放行（NetworkRequest 由 NetworkPolicy 单独处理）
                false
            }
        }
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_level_ordering() {
        assert!(PermissionLevel::Unrestricted.as_u8() < PermissionLevel::LightCheck.as_u8());
        assert!(PermissionLevel::LightCheck.as_u8() < PermissionLevel::StrictCheck.as_u8());
        assert!(PermissionLevel::StrictCheck.as_u8() < PermissionLevel::UserConfirm.as_u8());
    }

    #[test]
    fn test_permission_level_from_u8() {
        assert_eq!(PermissionLevel::from_u8(0), PermissionLevel::Unrestricted);
        assert_eq!(PermissionLevel::from_u8(1), PermissionLevel::LightCheck);
        assert_eq!(PermissionLevel::from_u8(2), PermissionLevel::StrictCheck);
        assert_eq!(PermissionLevel::from_u8(3), PermissionLevel::UserConfirm);
        assert_eq!(PermissionLevel::from_u8(99), PermissionLevel::UserConfirm);
    }

    #[test]
    fn test_permission_level_is_higher_than() {
        assert!(PermissionLevel::UserConfirm.is_higher_than(&PermissionLevel::Unrestricted));
        assert!(!PermissionLevel::Unrestricted.is_higher_than(&PermissionLevel::UserConfirm));
        assert!(!PermissionLevel::StrictCheck.is_higher_than(&PermissionLevel::StrictCheck));
    }

    #[test]
    fn test_permission_level_display() {
        assert_eq!(PermissionLevel::Unrestricted.to_string(), "unrestricted");
        assert_eq!(PermissionLevel::UserConfirm.to_string(), "user_confirm");
    }

    #[test]
    fn test_operation_type_display() {
        assert_eq!(OperationType::FileRead.to_string(), "file_read");
        assert_eq!(OperationType::CommandExecute.to_string(), "command_execute");
        assert_eq!(OperationType::NetworkRequest.to_string(), "network_request");
    }

    #[test]
    fn test_operation_type_default_level() {
        assert_eq!(
            OperationType::FileRead.default_level(),
            PermissionLevel::LightCheck
        );
        assert_eq!(
            OperationType::FileWrite.default_level(),
            PermissionLevel::StrictCheck
        );
        assert_eq!(
            OperationType::FileDelete.default_level(),
            PermissionLevel::UserConfirm
        );
        assert_eq!(
            OperationType::CommandExecute.default_level(),
            PermissionLevel::UserConfirm
        );
        assert_eq!(
            OperationType::NetworkRequest.default_level(),
            PermissionLevel::UserConfirm
        );
    }

    #[test]
    fn test_operation_request_builder() {
        let req = OperationRequest::new(OperationType::FileRead, "/test/file.txt", "agent")
            .with_session_id("sess_001")
            .with_worktree("/worktree");

        assert_eq!(req.operation, OperationType::FileRead);
        assert_eq!(req.target, "/test/file.txt");
        assert_eq!(req.actor, "agent");
        assert_eq!(req.session_id, Some("sess_001".to_string()));
        assert_eq!(req.worktree_path, Some("/worktree".to_string()));
    }

    #[test]
    fn test_operation_request_actor_checks() {
        let user_req = OperationRequest::new(OperationType::FileRead, "/f", "user");
        assert!(!user_req.is_agent());
        assert!(!user_req.is_extension());
        assert!(user_req.extension_id().is_none());

        let agent_req = OperationRequest::new(OperationType::FileRead, "/f", "agent");
        assert!(agent_req.is_agent());
        assert!(!agent_req.is_extension());

        let extension_req =
            OperationRequest::new(OperationType::FileRead, "/f", "extension:my-extension");
        assert!(extension_req.is_agent());
        assert!(extension_req.is_extension());
        assert_eq!(extension_req.extension_id(), Some("my-extension"));
    }

    #[test]
    fn test_check_result_allowed() {
        let result = CheckResult::allowed(PermissionLevel::LightCheck);
        assert!(result.allowed);
        assert_eq!(result.level, PermissionLevel::LightCheck);
        assert!(result.reason.is_none());
        assert!(!result.require_confirm);
        assert!(result.confirm_message.is_none());
    }

    #[test]
    fn test_check_result_denied() {
        let result = CheckResult::denied(PermissionLevel::UserConfirm, "路径被禁止");
        assert!(!result.allowed);
        assert_eq!(result.level, PermissionLevel::UserConfirm);
        assert_eq!(result.reason, Some("路径被禁止".to_string()));
        assert!(!result.require_confirm);
    }

    #[test]
    fn test_check_result_needs_confirm() {
        let result = CheckResult::needs_confirm(PermissionLevel::UserConfirm, "请确认删除");
        assert!(result.allowed);
        assert!(result.require_confirm);
        assert_eq!(result.confirm_message, Some("请确认删除".to_string()));
    }

    #[test]
    fn test_approval_mode_parse() {
        assert_eq!(
            ApprovalMode::from_str("suggest"),
            Some(ApprovalMode::Suggest)
        );
        assert_eq!(
            ApprovalMode::from_str("auto-edit"),
            Some(ApprovalMode::AutoEdit)
        );
        assert_eq!(
            ApprovalMode::from_str("full-auto"),
            Some(ApprovalMode::FullAuto)
        );
        assert!(ApprovalMode::from_str("SUGGEST").is_none());
        assert!(ApprovalMode::from_str("autoedit").is_none());
        assert!(ApprovalMode::from_str("fullauto").is_none());
        assert!(ApprovalMode::from_str("auto_edit").is_none());
        assert!(ApprovalMode::from_str("full_auto").is_none());
        assert!(ApprovalMode::from_str("unknown").is_none());
    }

    #[test]
    fn test_approval_mode_display() {
        assert_eq!(ApprovalMode::Suggest.to_string(), "suggest");
        assert_eq!(ApprovalMode::AutoEdit.to_string(), "auto-edit");
        assert_eq!(ApprovalMode::FullAuto.to_string(), "full-auto");
    }

    #[test]
    fn test_approval_mode_default() {
        assert_eq!(ApprovalMode::default(), ApprovalMode::Suggest);
    }

    // ======================================================================
    // 审批模式与操作类型联动规则测试
    // ======================================================================

    #[test]
    fn test_suggest_mode_requires_confirm() {
        // Suggest: FileRead 放行，其他都需确认
        assert!(!OperationType::FileRead.requires_confirm_in_mode(&ApprovalMode::Suggest));
        assert!(OperationType::FileWrite.requires_confirm_in_mode(&ApprovalMode::Suggest));
        assert!(OperationType::FileDelete.requires_confirm_in_mode(&ApprovalMode::Suggest));
        assert!(OperationType::DirCreate.requires_confirm_in_mode(&ApprovalMode::Suggest));
        assert!(OperationType::DirDelete.requires_confirm_in_mode(&ApprovalMode::Suggest));
        assert!(OperationType::CommandExecute.requires_confirm_in_mode(&ApprovalMode::Suggest));
        assert!(OperationType::NetworkRequest.requires_confirm_in_mode(&ApprovalMode::Suggest));
    }

    #[test]
    fn test_autoedit_mode_requires_confirm() {
        // AutoEdit: FileRead/FileWrite/DirCreate 放行，其他需确认
        assert!(!OperationType::FileRead.requires_confirm_in_mode(&ApprovalMode::AutoEdit));
        assert!(!OperationType::FileWrite.requires_confirm_in_mode(&ApprovalMode::AutoEdit));
        assert!(!OperationType::DirCreate.requires_confirm_in_mode(&ApprovalMode::AutoEdit));
        assert!(OperationType::FileDelete.requires_confirm_in_mode(&ApprovalMode::AutoEdit));
        assert!(OperationType::DirDelete.requires_confirm_in_mode(&ApprovalMode::AutoEdit));
        assert!(OperationType::CommandExecute.requires_confirm_in_mode(&ApprovalMode::AutoEdit));
        assert!(OperationType::NetworkRequest.requires_confirm_in_mode(&ApprovalMode::AutoEdit));
    }

    #[test]
    fn test_fullauto_mode_requires_confirm() {
        // FullAuto: 所有操作放行（NetworkRequest 由 NetworkPolicy 单独处理）
        assert!(!OperationType::FileRead.requires_confirm_in_mode(&ApprovalMode::FullAuto));
        assert!(!OperationType::FileWrite.requires_confirm_in_mode(&ApprovalMode::FullAuto));
        assert!(!OperationType::FileDelete.requires_confirm_in_mode(&ApprovalMode::FullAuto));
        assert!(!OperationType::DirCreate.requires_confirm_in_mode(&ApprovalMode::FullAuto));
        assert!(!OperationType::DirDelete.requires_confirm_in_mode(&ApprovalMode::FullAuto));
        assert!(!OperationType::CommandExecute.requires_confirm_in_mode(&ApprovalMode::FullAuto));
        assert!(!OperationType::NetworkRequest.requires_confirm_in_mode(&ApprovalMode::FullAuto));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let level = PermissionLevel::StrictCheck;
        let json = serde_json::to_string(&level).unwrap();
        let deserialized: PermissionLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(level, deserialized);

        let mode = ApprovalMode::AutoEdit;
        let json = serde_json::to_string(&mode).unwrap();
        let deserialized: ApprovalMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, deserialized);

        let op = OperationType::FileWrite;
        let json = serde_json::to_string(&op).unwrap();
        let deserialized: OperationType = serde_json::from_str(&json).unwrap();
        assert_eq!(op, deserialized);
    }
}
