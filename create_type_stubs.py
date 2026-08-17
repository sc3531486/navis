import os

base = r"D:\myworkspace\Navis Go\src-tauri\src\domains"

# 定义每个模块需要导出的类型
stubs = {
    # session::session - 最关键 (18 refs for SessionManager)
    "session/session/mod.rs": """//! 会话管理

pub mod store;
pub mod composer_runtime;

pub use store::{Session, SessionManager, SessionStore, Message, MessageContent, MessageRole, TimelineStatus, AgentTimelinePart, CompactedRange, SessionChange, SessionStatus};
pub use composer_runtime::ComposerRuntime;
""",
    "session/session/store/mod.rs": """//! 会话存储

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: Option<String>,
    pub worktree_root: Option<String>,
    pub status: SessionStatus,
    pub model: Option<String>,
    pub provider_id: Option<String>,
    pub metadata: Option<Value>,
    pub system_prompt: Option<String>,
    pub permission_policy: Option<String>,
    pub ui_metadata: Value,
    pub total_tokens: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus { Active, Archived, Deleted }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionManager;

impl SessionManager {
    pub fn new() -> Self { Self }
    pub fn get(&self, _id: &str) -> Result<Option<Session>, String> { Ok(None) }
    pub fn update(&self, _id: &str, _name: Option<&str>, _model: Option<&str>, _system_prompt: Option<&str>) -> Result<(), String> { Ok(()) }
    pub fn update_metadata(&self, _id: &str, _metadata: Option<Value>) -> Result<(), String> { Ok(()) }
    pub fn update_message_content(&self, _session_id: &str, _message_id: &str, _content: MessageContent, _tokens: Option<i64>, _metadata: Option<Value>) -> Result<(), String> { Ok(()) }
    pub fn rename(&self, _id: &str, _name: &str) -> Result<(), String> { Ok(()) }
    pub fn add_message(&self, _session_id: &str, _message: Message) -> Result<(), String> { Ok(()) }
    pub fn get_messages(&self, _session_id: &str, _limit: Option<i64>, _offset: Option<i64>) -> Result<Vec<Message>, String> { Ok(vec![]) }
}

#[derive(Debug, Clone)]
pub struct SessionStore;

impl SessionStore {
    pub fn new() -> Self { Self }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: MessageRole,
    pub content: MessageContent,
    pub token_count: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent { Text(String), Parts(Vec<Value>) }

impl std::fmt::Display for MessageContent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self { MessageContent::Text(s) => write!(f, "{}", s), _ => Ok(()) }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole { User, Assistant, System, Tool }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TimelineStatus { Pending, Running, Completed, Error }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTimelinePart {
    pub id: String, pub session_id: String, pub turn_id: String,
    pub message_id: String, pub sequence: i64, pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactedRange;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionChange;
""",

    # ai_platform::mcp - 10 refs for MCP
    "ai_platform/mcp/mod.rs": """//! MCP 工具协议引擎

pub mod protocol;
pub mod builtin;
pub mod tools;
pub mod transport;

pub struct MCP;

impl MCP {
    pub fn new() -> Self { Self }
    pub fn add_server(&self, _config: protocol::MCPServerConfig) -> Result<(), String> { Ok(()) }
    pub fn start_server(&self, _id: &str) -> Result<(), String> { Ok(()) }
    pub fn remove_server(&self, _id: &str) -> Result<(), String> { Ok(()) }
    pub fn register_tool(&self, _tool: protocol::ToolDefinition) -> Result<(), String> { Ok(()) }
    pub fn unregister_server_tools(&self, _server_id: &str) -> Result<usize, String> { Ok(0) }
    pub fn apply_tool_override(&self, _owner: &str, _server_id: &str, _tool_name: &str, _override: protocol::ToolDefinitionOverride) -> Result<(), String> { Ok(()) }
    pub fn remove_tool_override(&self, _owner: &str, _server_id: &str, _tool_name: &str) -> Result<(), String> { Ok(()) }
}
""",
    "ai_platform/mcp/protocol/mod.rs": """//! MCP 协议定义

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig { pub id: String, pub command: String, pub args: Vec<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition { pub name: String, pub description: String, pub input_schema: Value }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinitionOverride;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest { pub name: String, pub arguments: Value, pub session_id: Option<String> }
impl ToolCallRequest {
    pub fn new(name: &str, arguments: Value) -> Self { Self { name: name.to_string(), arguments, session_id: None } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult { pub call_id: String, pub tool_name: String, pub content: Vec<ToolContent>, pub is_error: bool, pub duration: std::time::Duration }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolContent { Text { text: String } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata { pub is_read_only: bool, pub is_destructive: bool, pub is_concurrency_safe: bool, pub risk_level: ToolMetadataRiskLevel, pub requires_network: bool, pub requires_filesystem: bool, pub renderer_hint: Option<String>, pub progress_mode: Value, pub estimated_duration: Value }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum ToolMetadataRiskLevel { #[default] Low, Medium, High }

pub fn platform_risk_override(_name: &str) -> Option<ToolMetadataRiskLevel> { None }
""",

    # ai_platform::mcp::protocol also needs ToolRendererHint, ToolRiskLevel, ToolUiHint
    # These are referenced in ui/mod.rs

    # ai_platform::gateway - Gateway type
    "ai_platform/gateway/mod.rs": """//! Gateway 模型网关

pub mod request;
pub mod response;
pub mod protocol;
pub mod provider;
pub mod multimodal;
pub mod router;
pub mod cost;
pub mod middleware;
pub mod offline;
pub mod quota;

pub use request::{ChatMessage, ChatRequest, ToolCall, ToolDefinition, GatewayConfig, ModelConfig, ProviderConfig, MessageRole, ApiProtocol};
pub use response::ChatResponse;
pub use protocol::{CapabilitySet, CustomProtocolConfig};

pub struct Gateway;
impl Gateway {
    pub fn new() -> Self { Self }
    pub fn router(&self, _request: ChatRequest) -> Result<ChatResponse, String> { unimplemented!() }
}
""",

    # ai_platform::gateway::request
    "ai_platform/gateway/request/mod.rs": """//! Gateway 请求

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole { System, User, Assistant, Tool }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage { pub role: MessageRole, pub content: String }
impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self { Self { role: MessageRole::System, content: content.into() } }
    pub fn user(content: impl Into<String>) -> Self { Self { role: MessageRole::User, content: content.into() } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest { pub model: String, pub messages: Vec<ChatMessage> }
impl ChatRequest {
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self { Self { model: model.into(), messages } }
    pub fn with_tools(self, _tools: Vec<ToolDefinition>) -> Self { self }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall { pub id: String, pub name: String, pub arguments: Value }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition { pub name: String, pub description: String, pub input_schema: Value }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiProtocol { ChatCompletions, Responses, Custom }
impl ApiProtocol {
    pub fn from_str(s: &str) -> Self { match s { "responses" => Self::Responses, _ => Self::ChatCompletions } }
}
""",

    # ai_platform::gateway::response
    "ai_platform/gateway/response/mod.rs": """//! Gateway 响应

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse { pub model: String, pub id: String, pub choices: Vec<Value>, pub usage: Option<Value>, pub finish_reason: Option<String> }
""",

    # ai_platform::lsp
    "ai_platform/lsp/mod.rs": """//! LSP 语言服务协议

pub struct LSPManager;
impl LSPManager {
    pub fn new() -> Result<Self, String> { Ok(Self) }
}

pub struct LSPServerConfig;
pub enum LanguageSource { Builtin, Extension }
pub fn set_global_manager(_m: std::sync::Arc<LSPManager>) -> Result<(), String> { Ok(()) }
""",

    # agent_core::agent - TaskManager
    "agent_core/agent/mod.rs": """//! Agent 决策引擎

pub mod turn_output;
pub mod sidechain;

pub use turn_output::assistant_visible_content;

pub struct TaskManager;
impl TaskManager {
    pub fn new() -> Self { Self }
    pub fn get_task_mut(&mut self, _id: &str) -> Option<&mut TaskRecord> { None }
}

pub struct TaskRecord;
impl TaskRecord {
    pub fn mark_failed(&mut self, _error: &str) {}
}

pub enum TaskKind { Agent, Sidechain, Background }
pub enum TaskStatus { Running, Completed, Failed }

pub struct SidechainOutcome;

pub fn notify_parent_sidechain_task(_manager: &TaskManager, _task_id: &str) {}
pub fn task_description(_task: &TaskRecord) -> &str { "" }

// Goal runner types
pub struct GoalRunnerCommand;
pub struct GoalRunnerDecision;
pub struct GoalRunnerRequest;
pub struct GoalRunnerStatePatch;
pub fn apply_goal_runner_command(_cmd: GoalRunnerCommand) {}
pub fn decide_goal_runner_next_task() -> GoalRunnerDecision { GoalRunnerDecision }
pub fn sidechain_stop_requested(_manager: &TaskManager, _task_id: &str) -> bool { false }
pub fn update_sidechain_progress(_manager: &TaskManager, _task_id: &str, _progress: f32) {}
pub fn mode_config_from_key(_key: Option<&str>) -> String { "default".to_string() }
""",

    # agent_core::tool_runtime - types used by framework
    "agent_core/tool_runtime/mod.rs": """//! 工具运行时

pub mod pipeline;
pub mod runtime;

pub use pipeline::AgentDefaultAllowConstraint;
pub use runtime::{AgentToolEvent, AgentToolPhase, AgentToolStatus};

pub const NAVIS_TOOL_SEARCH: &str = "tool_search";

pub struct AgentToolProgressCallback;
pub struct AgentExecutionContext;
impl AgentExecutionContext {
    pub fn new() -> Self { Self }
}
""",

    "agent_core/tool_runtime/pipeline/mod.rs": """//! 工具管线
pub struct AgentDefaultAllowConstraint;
pub struct ToolPipelineData;
pub fn run_standard_tool_pipeline(_data: &ToolPipelineData) -> Result<(), String> { Ok(()) }
""",

    "agent_core/tool_runtime/runtime/mod.rs": """//! 工具运行时实现

pub enum AgentToolEvent { Started, Completed, Failed }
pub enum AgentToolPhase { Pre, Main, Post }
pub enum AgentToolStatus { Pending, Running, Done }
pub struct AgentToolExecution;
""",

    # editor
    "editor/backend/mod.rs": """//! 后端扩展进程管理
pub struct BackendProcessManager;
impl BackendProcessManager {
    pub fn new() -> Self { Self }
}
""",

    "editor/clipboard/mod.rs": """//! 剪贴板
pub mod policy;
pub use policy::register_clipboard_constraints;
""",

    "editor/clipboard/policy/mod.rs": """//! 剪贴板策略
pub fn register_clipboard_constraints(_engine: &(), _mode: ()) -> Result<(), String> { Ok(()) }
""",

    "editor/file/mod.rs": """//! 文件系统
pub mod worktree_fs;
pub use worktree_fs::resolve_worktree_path;
""",

    "editor/file/worktree_fs/mod.rs": """//! Worktree 文件系统
pub fn resolve_worktree_path(_base: &str, _path: &str) -> String { String::new() }
""",

    "editor/git/mod.rs": """//! Git 操作
pub struct GitStatusParser;
pub struct GitDiff;
pub struct ChangeStatus;
""",

    # terminal
    "terminal/terminal/mod.rs": """//! 终端管理
pub struct TerminalManager;
impl TerminalManager {
    pub fn new() -> Self { Self }
}
""",

    # memory
    "memory/mod.rs": """//! 记忆模块
pub mod tool_memory;
pub mod project_memory;

pub use project_memory::MemoryStore;
""",

    "memory/project_memory/mod.rs": """//! 项目记忆
pub struct MemoryStore;
impl MemoryStore {
    pub fn new() -> Self { Self }
    pub fn save(&self, _memory: &()) -> Result<(), String> { Ok(()) }
    pub fn search(&self, _query: &str) -> Result<Vec<()>, String> { Ok(vec![]) }
}
""",

    # catalog
    "project/catalog/mod.rs": """//! 项目目录
pub struct ProjectManager;
pub struct RecentWorktree;
impl ProjectManager {
    pub fn new() -> Self { Self }
}
""",

    # composer_runtime
    "session/session/composer_runtime/mod.rs": """//! Composer 运行时
pub struct ComposerRuntime;
pub struct ComposerAttachment;
pub struct ComposerTask;
pub enum SubmitDisposition;
impl ComposerRuntime {
    pub fn new() -> Self { Self }
}
""",
}

for rel, content in stubs.items():
    path = os.path.join(base, rel)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(content)
    print(f"Created: {rel}")

print(f"\nCreated {len(stubs)} type stubs")
