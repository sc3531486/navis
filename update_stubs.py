import os

base = r"D:\myworkspace\Navis Go\src-tauri\src\domains"

updates = {
    # gateway/protocol - 需要导出 CapabilitySet, CustomProtocolConfig 等
    r"ai_platform\gateway\protocol\capability": """//! 协议能力集

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilityClipDiagnostic { pub version: String }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilityEvaluationInput { }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitySet { }

pub struct GatewayCapabilityEvaluatorPort;
pub struct GatewayCapabilityPolicies;
pub struct GatewayCapabilityProjection;
pub struct IntersectionCapabilityEvaluator;
pub struct ModelIdentity;
pub struct ProviderIdentity;
pub const GATEWAY_CAPABILITY_PROJECTION_VERSION: &str = "1.0";
""",
    r"ai_platform\gateway\protocol\chat_completions": """//! Chat Completions 协议适配器
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolAdapterInfo { pub id: String, pub name: String }

pub struct ProtocolAdapterRegistry;
pub struct ProviderAdapter;
pub struct StreamFrame;
pub struct StreamFrameDecoder;
""",
    r"ai_platform\gateway\protocol\custom": """//! 自定义协议
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProtocolConfig { pub endpoint: String }
""",
    # gateway/provider - 需要导出 builtin_provider_profile
    r"ai_platform\gateway\provider\profile": """//! Provider 配置
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderAuthProfile { pub provider_id: String }

pub fn builtin_provider_profile() -> ProviderAuthProfile {
    ProviderAuthProfile { provider_id: "builtin".to_string() }
}

pub const BUILTIN_PROVIDER_PROFILES: &[&str] = &["openai", "anthropic", "google"];
""",
    # mcp/transport - 需要导出 TransportAdapter 等
    r"ai_platform\mcp\transport\adapter_trait": """//! 传输适配器 trait
use serde::{Deserialize, Serialize};

pub trait TransportAdapter: Send + Sync {
    fn name(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportAdapterCapability { pub key: String }

pub fn create_default_adapter() -> Box<dyn TransportAdapter> {
    struct DefaultAdapter;
    impl TransportAdapter for DefaultAdapter {
        fn name(&self) -> &str { "default" }
    }
    Box::new(DefaultAdapter)
}

pub fn is_builtin_transport_key(_key: &str) -> bool { true }
pub fn list_builtin_transports() -> Vec<String> { vec!["stdio".to_string()] }
pub fn normalize_custom_transport_key(key: &str) -> String { key.to_string() }
pub fn transport_key(name: &str) -> String { name.to_string() }
""",
    # tool_runtime/pipeline - 需要导出 AgentDefaultAllowConstraint, run_standard_tool_pipeline
    r"agent_core\tool_runtime\pipeline": """//! 工具运行管线
//! TODO: 待实现完整管线

pub struct AgentDefaultAllowConstraint;

pub struct ToolPipelineData;

pub fn run_standard_tool_pipeline(_data: &ToolPipelineData) -> Result<(), String> {
    Ok(())
}
""",
    # self_evolution - 需要导出类型
    r"agent_core\agent\self_evolution": """//! Agent 自我进化模块
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionExperience { pub tool: String, pub success: bool }

#[derive(Debug, Clone, Default)]
pub struct ExperienceFilter;

pub struct ExperienceLogger;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExperienceOutcome { Success, Failure, Timeout }
""",
    # context/assembler sub-modules
    r"agent_core\context\assembler\compression_boundary": """//! 压缩边界检测

pub fn select_gateway_compression_boundary(_messages: &[()]) -> Option<usize> { None }
pub fn select_storage_compression_boundary(_messages: &[()]) -> Option<usize> { None }
""",
    r"agent_core\context\assembler\compression_render": """//! 压缩结果渲染
pub fn render_gateway_message_for_compression(_msg: &str) -> String { _msg.to_string() }
pub fn render_storage_message_for_compression(_msg: &str) -> String { _msg.to_string() }
""",
    r"agent_core\context\assembler\runtime": """//! 上下文组装运行时
pub async fn block_on_context_future<F, T>(_f: F) -> Result<T, String>
where F: std::future::Future<Output = Result<T, String>> + Send,
{ Err("stub".to_string()) }
""",
    r"agent_core\context\assembler\summary": """//! 上下文摘要生成
pub fn generate_summary_text(_messages: &[()]) -> String { String::new() }
""",
    # tool_runtime/runtime sub-modules
    r"agent_core\tool_runtime\runtime\events": """//! 工具执行事件
pub enum AgentToolEvent { Started, Completed, Failed }
pub struct AgentToolExecution;
pub enum AgentToolPhase { Pre, Main, Post }
pub struct AgentToolProgressCallback;
pub enum AgentToolStatus { Pending, Running, Done }
""",
    r"agent_core\tool_runtime\runtime\messages": """//! 工具消息格式
pub fn assistant_tool_message(_id: &str, _content: &str) -> String { String::new() }
pub fn assistant_tool_message_with_content(_id: &str, _content: &str) -> String { String::new() }
""",
    r"agent_core\tool_runtime\runtime\resolver": """//! 工具解析器
pub fn effective_gateway_tool_call(_name: &str, _args: &str) -> Option<String> { None }
pub fn is_supported_gateway_tool(_name: &str) -> bool { true }
pub fn is_supported_sidechain_gateway_tool(_name: &str) -> bool { true }
pub fn executable_specs() -> Vec<String> { vec![] }
pub fn resolve_effective_tool_call(_name: &str, _args: &str) -> Option<String> { None }
""",
    r"agent_core\tool_runtime\runtime\session_context": """//! 会话上下文
pub fn inject_worktree_root(_ctx: &mut (), _root: &str) {}
pub fn take_permission_grant(_ctx: &()) -> Option<String> { None }
""",
    r"agent_core\tool_runtime\runtime\tool_search": """//! 工具搜索
pub fn execute_tool_search(_query: &str) -> Vec<String> { vec![] }
""",
    # special sub-modules
    r"agent_core\tool_runtime\special\host": """//! 特殊工具宿主
pub struct SpecialAgentToolHost;
""",
    r"agent_core\tool_runtime\special\response": """//! 特殊工具响应
pub fn special_tool_execution(_name: &str) -> Result<String, String> { Ok(String::new()) }
""",
    r"agent_core\tool_runtime\special\sidechain": """//! Sidechain 工具
pub const SIDECHAIN_OUTPUT_CONTRACT: &str = "sidechain_output";
""",
    r"agent_core\tool_runtime\special\todo": """//! Todo 工具
pub fn parse_todos(_content: &str) -> Vec<String> { vec![] }
pub fn todo_items_output(_items: &[String]) -> String { String::new() }
""",
    # catalog sub-modules
    r"agent_core\tool_runtime\catalog\constants": """//! 工具常量
pub const NAVIS_TOOL_SEARCH: &str = "tool_search";
pub const NAVIS_EXECUTE_TOOL: &str = "execute_tool";
pub const MCP_FS_MULTI_EDIT: &str = "mcp_fs_multi_edit";
pub const MCP_FS_REPLACE_IN_FILE: &str = "mcp_fs_replace_in_file";
pub const MCP_FS_WRITE_FILE: &str = "mcp_fs_write_file";
""",
}

for rel, content in updates.items():
    path = os.path.join(base, rel)
    os.makedirs(path, exist_ok=True)
    mod_path = os.path.join(path, "mod.rs")
    with open(mod_path, "w", encoding="utf-8", newline="\n") as f:
        f.write(content)
    print(f"Updated {rel}/mod.rs")

print(f"Updated {len(updates)} modules")
