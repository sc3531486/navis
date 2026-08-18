use crate::extension::types::{
    ApiProtocol, CapabilityClipDiagnostic, CapabilitySet, GatewayProviderStatus,
};
use crate::extension::models::{ExtensionPermissions, MenuRisk};
use crate::foundation::status::StatusPresentation;
use crate::kernel::SchemaVersion;
use crate::extension::types::UiAgentTimelinePart;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiLanguageOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiLanguageState {
    pub language: String,
    pub builtin_languages: Vec<UiLanguageOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiMenuRegistration {
    pub id: String,
    pub label: String,
    pub target: String,
    pub command: String,
    pub group: Option<String>,
    pub when: Option<String>,
    pub icon: Option<String>,
    pub shortcut: Option<String>,
    pub risk: Option<MenuRisk>,
    pub extension_id: Option<String>,
    pub action: Option<UiMenuBuiltinAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiCommandRegistration {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub category: String,
    pub icon: Option<String>,
    pub extension_id: String,
    pub extension_name: String,
    pub action: UiMenuBuiltinAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiExtensionKeybinding {
    pub id: String,
    pub keybinding: String,
    pub scope: String,
    pub command: String,
    pub description: String,
    pub category: String,
    pub extension_id: String,
    pub extension_name: String,
    pub action: UiMenuBuiltinAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UiExtensionViewDescriptor {
    pub view_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub zone: String,
    /// Deprecated compatibility field for older frontend code; mirrors `zone`.
    pub placement: String,
    pub renderer: String,
    pub entry: Option<String>,
    pub resource_path: Option<String>,
    pub config: Option<Value>,
    pub allow_close: bool,
    pub default_visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiHostViewTarget {
    #[serde(flatten)]
    pub view: UiExtensionViewDescriptor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "PascalCase", rename_all_fields = "camelCase")]
pub enum UiMenuBuiltinAction {
    OpenView { view: UiHostViewTarget },
    ToggleView { view: UiHostViewTarget },
    OpenDialog { view: UiHostViewTarget, size: Option<String>, position: Option<String>, modal: Option<bool> },
    RunScript { script_id: String, args: Option<Value> },
    SendMessage { target: String, payload: Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiSlashCommandRegistration {
    pub trigger: String,
    pub name: String,
    pub description: String,
    pub trigger_type: String,
    pub source: String,
    pub source_label: String,
    pub extension_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiExtensionContributionCounts {
    pub work_modes: usize,
    pub views: usize,
    pub menus: usize,
    pub commands: usize,
    pub keybindings: usize,
    pub triggers: usize,
    pub mcp_servers: usize,
    pub providers: usize,
    pub zones: usize,
    pub scripts: usize,
    pub toolbar_items: usize,
    pub statusbar_items: usize,
    pub inline_extensions: usize,
    pub configuration: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiZone {
    pub id: String,
    pub name: String,
    /// zone 类型：`"builtin"` 或 `"extension"`。
    pub kind: String,
    pub extension_id: Option<String>,
    pub anchor_parent: Option<String>,
    pub anchor_position: Option<String>,
    /// 当前宿主是否已承接（可渲染）。
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiExtensionScript {
    pub extension_id: String,
    pub script_id: String,
    pub entry: String,
    pub resource_path: Option<String>,
    pub run_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiExtensionLocale {
    pub extension_id: String,
    pub lang: String,
    pub entry: String,
    pub resource_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiExtensionDiscoveryResult {
    pub extension_id: String,
    pub extension_name: String,
    pub provides: Vec<String>,
    pub views: Vec<String>,
    pub commands: Vec<String>,
    pub scripts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiExtensionConfiguration {
    pub extension_id: String,
    pub schema: Value,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiExtensionConfigurationUpdate {
    pub extension_id: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiExtensionPointRegistration {
    pub extension_id: String,
    pub kind: String,
    pub id: String,
    pub label: Option<String>,
    pub command: Option<String>,
    pub target: Option<String>,
    pub group: Option<String>,
    pub when: Option<String>,
    pub data: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiExtensionState {
    pub id: String,
    pub status: String,
    pub status_presentation: StatusPresentation,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub install_path: String,
    pub installed_at: String,
    pub enabled_at: Option<String>,
    pub error: Option<String>,
    pub permissions: ExtensionPermissions,
    pub contribution_counts: UiExtensionContributionCounts,
    pub provides: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiExtensionView {
    pub extension_id: String,
    pub extension_name: String,
    pub extension_description: String,
    #[serde(flatten)]
    pub view: UiExtensionViewDescriptor,
    pub contribution_counts: UiExtensionContributionCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiWorkModeModelPreferences {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub extended_thinking: Option<bool>,
    pub language_quality_emphasis: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiWorkModeRegistration {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub role: Option<String>,
    pub available_tools: Option<Vec<String>>,
    pub skills: Option<Vec<String>>,
    pub commands: Option<Vec<String>>,
    pub context_policy: Option<String>,
    pub behavior_rules: Option<Vec<String>>,
    pub entry_view: Option<String>,
    pub default_views: Option<Vec<String>>,
    pub default_model: Option<String>,
    pub model_preferences: Option<UiWorkModeModelPreferences>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiRegisteredWorkMode {
    pub extension_id: String,
    pub extension_name: String,
    pub mode_id: String,
    pub runtime_id: String,
    pub mode: UiWorkModeRegistration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiGatewayProvider {
    pub id: String,
    pub label: String,
    pub description: String,
    pub default_base_url: String,
    pub default_protocol: ApiProtocol,
    pub protocols: Vec<ApiProtocol>,
    pub requires_secret: bool,
    pub capabilities: CapabilitySet,
    pub capability_version: SchemaVersion,
    pub diagnostics: Vec<CapabilityClipDiagnostic>,
    pub configured: bool,
    pub model_count: usize,
    pub available_model_count: usize,
    pub status: GatewayProviderStatus,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiGatewayModel {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub supports_multimodal: bool,
    pub supports_reasoning_effort: bool,
    pub supports_structured_output: bool,
    pub supports_usage: bool,
    pub default_reasoning_effort: String,
    pub api_protocol: ApiProtocol,
    pub cost_per_1k_input: f64,
    pub cost_per_1k_output: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiGatewayProtocolCatalog {
    pub id: ApiProtocol,
    pub runtime_id: String,
    pub label: String,
    pub description: String,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub supports_multimodal: bool,
    pub supports_reasoning_effort: bool,
    pub supports_structured_output: bool,
    pub supports_usage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiGatewayProviderCatalog {
    pub id: String,
    pub label: String,
    pub description: String,
    pub default_base_url: String,
    pub default_protocol: ApiProtocol,
    pub protocols: Vec<ApiProtocol>,
    pub requires_secret: bool,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub supports_multimodal: bool,
    pub supports_reasoning_effort: bool,
    pub supports_structured_output: bool,
    pub supports_usage: bool,
    pub capabilities: CapabilitySet,
    pub capability_version: SchemaVersion,
    pub diagnostics: Vec<CapabilityClipDiagnostic>,
    pub configured: bool,
    pub model_count: usize,
    pub available_model_count: usize,
    pub status: GatewayProviderStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiGatewayCatalog {
    pub protocols: Vec<UiGatewayProtocolCatalog>,
    pub providers: Vec<UiGatewayProviderCatalog>,
    pub models: Vec<UiGatewayModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiGatewayModelConfig {
    pub id: String,
    pub name: String,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub supports_multimodal: bool,
    pub supports_reasoning_effort: bool,
    pub supports_structured_output: bool,
    pub supports_usage: bool,
    pub default_reasoning_effort: String,
    pub api_protocol: ApiProtocol,
    pub cost_per_1k_input: f64,
    pub cost_per_1k_output: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UiGatewayProviderConfig {
    pub id: String,
    pub provider_type: String,
    pub name: String,
    pub base_url: String,
    pub secret_ref: Option<String>,
    pub models: Vec<UiGatewayModelConfig>,
    pub default_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiGatewayConfig {
    pub providers: Vec<UiGatewayProviderConfig>,
    pub default_provider: Option<String>,
    pub offline_fallback_model: Option<String>,
    pub request_timeout_secs: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiGatewayDiscoveredModel {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiExternalEditorConfig {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiEditorSettings {
    pub font_size: u32,
    pub tab_size: u32,
    pub word_wrap: String,
    pub minimap: bool,
    pub format_on_save: bool,
    pub external_editors: Vec<UiExternalEditorConfig>,
    pub default_external_editor_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiRecentWorktree {
    pub id: String,
    pub name: String,
    pub path: String,
    pub opened_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiWorktree {
    pub id: String,
    pub name: String,
    pub path: String,
    pub opened_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiWorktreeFileNode {
    pub name: String,
    pub relative_path: String,
    pub absolute_path: String,
    pub is_directory: bool,
    pub children: Vec<UiWorktreeFileNode>,
    pub extension: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiSessionWorktreeSnapshot {
    pub session_id: String,
    pub worktree: Option<UiWorktree>,
    pub worktree_files: Vec<String>,
    pub file_tree: Vec<UiWorktreeFileNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiWorktreeFileDocument {
    pub session_id: String,
    pub relative_path: String,
    pub absolute_path: String,
    pub file_name: String,
    pub extension: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiComposerTask {
    pub id: String,
    #[serde(default)]
    pub kind: Option<String>,
    pub text: String,
    #[serde(default)]
    pub source_text: Option<String>,
    #[serde(default)]
    pub display_text: Option<String>,
    #[serde(default)]
    pub attachments: Vec<UiChatMessageAttachment>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiPendingPlanReview {
    pub id: String,
    pub request_text: String,
    #[serde(default)]
    pub plan_content: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiComposerRunState {
    pub session_id: String,
    pub plan_mode_enabled: bool,
    #[serde(default)]
    pub plan_execution_started: bool,
    #[serde(default)]
    pub multi_agent_enabled: bool,
    #[serde(default)]
    pub pending_plan_review: Option<UiPendingPlanReview>,
    pub goal_tracking_enabled: bool,
    pub goal_paused: bool,
    pub active_goal_text: Option<String>,
    pub active_goal_started_at: Option<String>,
    #[serde(default)]
    pub running_task: Option<UiComposerTask>,
    #[serde(default)]
    pub queued_tasks: Vec<UiComposerTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposerTaskPayload {
    pub session_id: String,
    pub task: UiComposerTask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposerTaskIdPayload {
    pub session_id: String,
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposerTaskSubmitResult {
    pub state: UiComposerRunState,
    pub disposition: String,
    pub task: UiComposerTask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposerTaskFinishResult {
    pub state: UiComposerRunState,
    pub next_task: Option<UiComposerTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposerTaskClearResult {
    pub state: UiComposerRunState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalRunnerPayload {
    pub session_id: String,
    pub goal: String,
    #[serde(default)]
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalRunnerControlPayload {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiSidebarSession {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub pinned: bool,
    pub unread: bool,
    pub has_running_task: bool,
    pub has_completed_task: bool,
    pub model: Option<String>,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub mode: Option<String>,
    pub worktree_root: Option<String>,
    pub permission_policy: Option<String>,
    pub transcript_view: String,
    pub reasoning_effort: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiSessionWorktree {
    pub name: String,
    pub sessions: Vec<UiSidebarSession>,
    pub collapsed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiSessionTree {
    pub worktrees: Vec<UiSessionWorktree>,
    pub active_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiChatMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub attachments: Vec<UiChatMessageAttachment>,
    pub token_count: Option<i64>,
    pub created_at: String,
    pub agent_timeline_parts: Vec<UiAgentTimelinePart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiChatMessageAttachment {
    pub kind: String,
    pub name: String,
    pub mime_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub data_base64: Option<String>,
    pub text_content: Option<String>,
    pub is_truncated: Option<bool>,
    pub model_readable: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiSessionMessages {
    pub messages: Vec<UiChatMessage>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiTask {
    pub id: String,
    pub session_id: String,
    pub parent_task_id: Option<String>,
    pub sidechain_session_id: Option<String>,
    pub kind: String,
    pub owner: Option<String>,
    pub active_form: Option<String>,
    pub blocks: Vec<String>,
    pub blocked_by: Vec<String>,
    pub status: String,
    pub status_presentation: StatusPresentation,
    pub description: String,
    pub error: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: u64,
    pub message_count: usize,
    pub tool_call_count: usize,
    pub latest_tool_name: Option<String>,
    pub token_count: i64,
    pub latest_message: Option<String>,
    pub current_activity: Option<String>,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiTodoItem {
    pub id: String,
    pub content: String,
    pub status: String,
    pub status_presentation: StatusPresentation,
    pub priority: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiToolApprovalRequest {
    pub request_id: String,
    pub session_id: String,
    pub worktree_root: Option<String>,
    pub call_id: String,
    pub permission: String,
    pub tool: String,
    pub gateway_tool: String,
    pub pattern: String,
    pub title: String,
    pub summary: Option<String>,
    pub message: String,
    pub risk_level: String,
    pub args: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiSessionContextUsage {
    pub session_id: String,
    pub model: Option<String>,
    pub used_tokens: usize,
    pub total_tokens: usize,
    pub used_percent: u8,
    pub compression_threshold_percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiSessionGitDiffFile {
    pub path: String,
    pub status: String,
    pub staged: bool,
    pub insertions: i64,
    pub deletions: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiSessionGitDiff {
    pub session_id: String,
    pub worktree_root: String,
    pub is_repo: bool,
    pub can_create_repo: bool,
    pub staged: bool,
    pub diff: String,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub file_changes: Vec<UiSessionGitDiffFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionViewPayload {
    pub extension_id: String,
    pub view_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionEnabledPayload {
    pub extension_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallExtensionPayload {
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionIdPayload {
    pub extension_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRecentWorktreesPayload {
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordRecentWorktreePayload {
    pub path: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveRecentWorktreePayload {
    pub path: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionIdPayload {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionWorktreeFilePayload {
    pub session_id: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteSessionWorktreeFilePayload {
    pub session_id: String,
    pub relative_path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenSessionExternalEditorPayload {
    pub session_id: String,
    pub editor_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguagePayload {
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSessionMessagesPayload {
    pub session_id: String,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    #[serde(default)]
    pub latest: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSessionChangesPayload {
    pub session_id: String,
    pub turn_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionGitDiffPayload {
    pub session_id: String,
    pub staged: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionGitRepoPayload {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTasksPayload {
    pub session_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskIdPayload {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearFinishedTasksPayload {
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTodosPayload {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelStreamPayload {
    pub stream_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolApprovalResponsePayload {
    pub request_id: String,
    pub decision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendSessionMessagePayload {
    pub session_id: String,
    pub content: String,
    #[serde(default)]
    pub display_content: Option<String>,
    #[serde(default)]
    pub attachments: Vec<UiChatMessageAttachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewSessionPayload {
    pub name: Option<String>,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub worktree_name: Option<String>,
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameSessionPayload {
    pub session_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionModelPayload {
    pub session_id: String,
    pub provider_id: String,
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPermissionPayload {
    pub session_id: String,
    pub permission_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTranscriptViewPayload {
    pub session_id: String,
    pub transcript_view: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionReasoningEffortPayload {
    pub session_id: String,
    pub reasoning_effort: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionWorktreeRootPayload {
    pub session_id: String,
    pub worktree_root: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionFlagPayload {
    pub session_id: String,
    pub value: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveSessionToWorktreePayload {
    pub session_id: String,
    pub worktree_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameWorktreePayload {
    pub old_name: String,
    pub new_name: String,
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeNamePayload {
    pub worktree_name: String,
    pub mode: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn view_descriptor() -> UiExtensionViewDescriptor {
        UiExtensionViewDescriptor {
            view_id: "sample.view".to_string(),
            name: "Sample View".to_string(),
            icon: Some("panel".to_string()),
            zone: "rightWorkspace".to_string(),
            placement: "rightWorkspace".to_string(),
            renderer: "host:panel".to_string(),
            entry: None,
            resource_path: None,
            config: Some(json!({"density": "compact"})),
            allow_close: true,
            default_visible: false,
        }
    }

    #[test]
    fn host_view_target_serializes_view_fields_flat() {
        let target = UiHostViewTarget {
            view: view_descriptor(),
        };

        assert_eq!(
            serde_json::to_value(target).unwrap(),
            json!({
                "viewId": "sample.view",
                "name": "Sample View",
                "icon": "panel",
                "zone": "rightWorkspace",
                "placement": "rightWorkspace",
                "renderer": "host:panel",
                "entry": null,
                "resourcePath": null,
                "config": {"density": "compact"},
                "allowClose": true,
                "defaultVisible": false,
            })
        );
    }

    #[test]
    fn extension_view_serializes_shared_view_fields_flat() {
        let view = view_descriptor();
        let extension_view = UiExtensionView {
            extension_id: "sample.extension".to_string(),
            extension_name: "Sample Extension".to_string(),
            extension_description: "An extension view".to_string(),
            view,
            contribution_counts: UiExtensionContributionCounts {
                work_modes: 0,
                views: 1,
                menus: 1,
                commands: 1,
                keybindings: 0,
                triggers: 0,
                mcp_servers: 0,
                providers: 0,
                zones: 0,
                scripts: 0,
                toolbar_items: 0,
                statusbar_items: 0,
                inline_extensions: 0,
                configuration: 0,
            },
        };

        let value = serde_json::to_value(&extension_view).unwrap();
        assert_eq!(value["extensionId"], "sample.extension");
        assert_eq!(value["viewId"], "sample.view");
        assert_eq!(value["renderer"], "host:panel");
        assert_eq!(value["contributionCounts"]["views"], 1);
        assert!(value.get("view").is_none());

        let decoded: UiExtensionView = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.view, extension_view.view);
    }
}
