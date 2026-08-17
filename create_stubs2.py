import os

base = r"D:\myworkspace\Navis Go\src-tauri\src\domains"

stubs = [
    # assembler sub-modules
    (r"agent_core\context\assembler\compression_boundary", "//! 压缩边界检测\n"),
    (r"agent_core\context\assembler\compression_render", "//! 压缩结果渲染\n"),
    (r"agent_core\context\assembler\compression_template", "//! 压缩模板\n"),
    (r"agent_core\context\assembler\runtime", "//! 上下文组装运行时\n"),
    (r"agent_core\context\assembler\summary", "//! 上下文摘要生成\n"),
    # catalog sub-modules
    (r"agent_core\tool_runtime\catalog\constants", "//! 工具常量定义\npub const NAVIS_TOOL_SEARCH: &str = \"tool_search\";\npub const NAVIS_EXECUTE_TOOL: &str = \"execute_tool\";\n"),
    (r"agent_core\tool_runtime\catalog\mode_filter", "//! 模式过滤器\n"),
    (r"agent_core\tool_runtime\catalog\naming", "//! 工具命名规范\n"),
    (r"agent_core\tool_runtime\catalog\schemas", "//! 工具 Schema 定义\n"),
    (r"agent_core\tool_runtime\catalog\specs", "//! 工具规格定义\npub const AGENT_TOOL_SPECS: &str = \"agent\";\npub const FILE_TOOL_SPECS: &str = \"file\";\npub const GIT_TOOL_SPECS: &str = \"git\";\npub const LSP_TOOL_SPECS: &str = \"lsp\";\npub const MCP_HOST_TOOL_SPECS: &str = \"mcp_host\";\npub const TERMINAL_TOOL_SPECS: &str = \"terminal\";\npub const TOOL_DISCOVERY_SPECS: &str = \"discovery\";\npub const WEB_TOOL_SPECS: &str = \"web\";\n"),
    # runtime sub-modules
    (r"agent_core\tool_runtime\runtime\events", "//! 工具执行事件\n"),
    (r"agent_core\tool_runtime\runtime\messages", "//! 工具消息格式\n"),
    (r"agent_core\tool_runtime\runtime\resolver", "//! 工具解析器\n"),
    (r"agent_core\tool_runtime\runtime\session_context", "//! 会话上下文\n"),
    (r"agent_core\tool_runtime\runtime\tool_search", "//! 工具搜索\n"),
    # special sub-modules
    (r"agent_core\tool_runtime\special\host", "//! 特殊工具宿主\n"),
    (r"agent_core\tool_runtime\special\response", "//! 特殊工具响应\n"),
    (r"agent_core\tool_runtime\special\sidechain", "//! Sidechain 工具\n"),
    (r"agent_core\tool_runtime\special\todo", "//! Todo 工具\n"),
    # session store sub-modules
    (r"session\session\store\store_models", "//! 会话存储数据模型\nuse serde::{Deserialize, Serialize};\nuse serde_json::Value;\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct Session {\n    pub id: String,\n    pub worktree_root: Option<String>,\n    pub name: Option<String>,\n    pub status: SessionStatus,\n    pub model: Option<String>,\n    pub provider_id: Option<String>,\n    pub model_id: Option<String>,\n    pub system_prompt: Option<String>,\n    pub total_tokens: i64,\n    pub created_at: String,\n    pub updated_at: String,\n    pub archived: bool,\n    pub archived_at: Option<String>,\n    pub metadata: Option<Value>,\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]\npub enum SessionStatus {\n    Active,\n    Archived,\n    Deleted,\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct Message {\n    pub id: String,\n    pub session_id: String,\n    pub role: MessageRole,\n    pub content: MessageContent,\n    pub token_count: Option<i64>,\n    pub tool_calls: Option<Vec<ToolCall>>,\n    pub tool_call_id: Option<String>,\n    pub created_at: String,\n    pub metadata: Option<Value>,\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]\npub enum MessageRole {\n    User,\n    Assistant,\n    System,\n    Tool,\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub enum MessageContent {\n    Text(String),\n    Parts(Vec<ContentPart>),\n}\n\nimpl std::fmt::Display for MessageContent {\n    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {\n        match self {\n            MessageContent::Text(s) => write!(f, \"{}\", s),\n            MessageContent::Parts(parts) => {\n                for p in parts {\n                    match p {\n                        ContentPart::Text(t) => write!(f, \"{}\", t.text)?,\n                        ContentPart::Image(img) => write!(f, \"[image: {}]\", img.media_type)?,\n                        ContentPart::File(file) => write!(f, \"[file: {}]\", file.file_name)?,\n                    }\n                }\n                Ok(())\n            }\n        }\n    }\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub enum ContentPart {\n    Text(TextPart),\n    Image(ImagePart),\n    File(FilePart),\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct TextPart { pub text: String }\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct ImagePart { pub media_type: String, pub data: String }\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct FilePart { pub file_name: String, pub content: String }\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct ToolCall {\n    pub id: String,\n    pub name: String,\n    pub arguments: Value,\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct AgentTimelinePart {\n    pub id: String,\n    pub session_id: String,\n    pub turn_id: String,\n    pub message_id: String,\n    pub sequence: i64,\n    pub kind: String,\n    pub status: Option<crate::domains::session::session::TimelineStatus>,\n    pub call_id: Option<String>,\n    pub data: Value,\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct CompactedRange {\n    pub session_id: String,\n    pub start_sequence: i64,\n    pub end_sequence: i64,\n    pub summary: String,\n    pub created_at: String,\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct SessionChange {\n    pub id: String,\n    pub session_id: String,\n    pub change_type: String,\n    pub data: Value,\n    pub created_at: String,\n    pub reverted: bool,\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct SessionEvent {\n    pub id: String,\n    pub session_id: String,\n    pub event_type: String,\n    pub data: Value,\n    pub sequence: i64,\n    pub created_at: String,\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct ForkSessionCopyResult {\n    pub session_id: String,\n    pub message_count: usize,\n}\n"),
    (r"session\session\store\store_timeline_status", "//! 时间线状态\nuse serde::{Deserialize, Serialize};\n\n#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\n#[serde(rename_all = \"snake_case\")]\npub enum TimelineStatus {\n    Pending,\n    Running,\n    Retrying,\n    WaitingPermission,\n    Completed,\n    Error,\n    Denied,\n    Aborted,\n    Interrupted,\n    Reused,\n    Compacted,\n    Unknown(String),\n}\n\nimpl TimelineStatus {\n    pub fn as_str(&self) -> &str {\n        match self {\n            Self::Pending => \"pending\",\n            Self::Running => \"running\",\n            Self::Retrying => \"retrying\",\n            Self::WaitingPermission => \"waiting_permission\",\n            Self::Completed => \"completed\",\n            Self::Error => \"error\",\n            Self::Denied => \"denied\",\n            Self::Aborted => \"aborted\",\n            Self::Interrupted => \"interrupted\",\n            Self::Reused => \"reused\",\n            Self::Compacted => \"compacted\",\n            Self::Unknown(s) => s,\n        }\n    }\n}\n"),
]

created = 0
for rel, content in stubs:
    path = os.path.join(base, rel)
    os.makedirs(path, exist_ok=True)
    mod_path = os.path.join(path, "mod.rs")
    with open(mod_path, "w", encoding="utf-8", newline="\n") as f:
        f.write(content)
    created += 1

print(f"Created {created} stub files")
