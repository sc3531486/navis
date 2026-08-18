//! 框架层通用类型定义（通用，不绑定 agent 领域）

use crate::extension::operation_runtime::McpOperationPort;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ============================================================
// Gateway
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProtocolConfig { pub endpoint: String, pub api_key: Option<String> }
impl CustomProtocolConfig {
    pub fn from_manifest(endpoint: &str, _config: Value) -> Result<Self, String> {
        Ok(Self { endpoint: endpoint.to_string(), api_key: None })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApiProtocol { ChatCompletions, Responses, Custom(String) }
impl ApiProtocol {
    pub fn as_str(&self) -> &str { match self { Self::ChatCompletions => "chat_completions", Self::Responses => "responses", Self::Custom(s) => s } }
    pub fn from_str(s: &str) -> Self { match s { "responses" => Self::Responses, _ => Self::ChatCompletions } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String, pub name: String, pub api_key: Option<String>,
    pub provider_type: String, pub base_url: String, pub secret_ref: Option<String>,
    pub auth_profile: Option<serde_json::Value>, pub models: Vec<ModelConfig>,
    pub default_model: String,
}
impl ProviderConfig { pub fn validate(&self) -> Result<(), String> { Ok(()) } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String, pub name: String, pub provider_id: String,
    pub api_protocol: ApiProtocol, pub context_window: u32,
    pub max_output_tokens: u32, pub supports_streaming: bool,
    pub supports_multimodal: bool, pub supports_reasoning_effort: bool,
    pub supports_structured_output: bool, pub supports_tools: bool, pub supports_usage: bool,
}
impl ModelConfig {
    pub fn new(id: String, name: String) -> Self {
        Self { id, name, provider_id: String::new(), api_protocol: ApiProtocol::ChatCompletions, context_window: 0, max_output_tokens: 0, supports_streaming: true, supports_multimodal: false, supports_reasoning_effort: false, supports_structured_output: false, supports_tools: true, supports_usage: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderAuthProfile { pub scheme: String, pub header: String }
impl ProviderAuthProfile {
    pub fn from_manifest(scheme: &str, header: &str) -> Result<Self, String> {
        Ok(Self { scheme: scheme.to_string(), header: header.to_string() })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitySet {
    pub protocols: Vec<String>, pub models: Vec<String>, pub tools: bool,
    pub streaming: bool, pub multimodal: bool, pub reasoning: bool,
    pub structured_output: bool, pub usage: bool, pub model_catalog: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToolRiskLevel { #[default] None, Read, Write, Network, Command, Destructive }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityClipDiagnostic { pub version: String }

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum GatewayProviderStatus { #[default] Available, Unavailable }

pub fn platform_risk_override(_name: &str) -> Option<ToolRiskLevel> { None }

// ============================================================
// MCP
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    pub id: String,
    pub name: Option<String>,
    pub command: String,
    pub args: Vec<String>,
    pub auto_start: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub server_id: String,
    pub declared_risk: ToolRiskLevel,
    pub effective_risk: ToolRiskLevel,
    pub user_visible: bool,
}

impl ToolDefinition {
    pub fn new(name: String, description: String, input_schema: Value, server_id: String) -> Self {
        Self {
            name,
            description,
            input_schema,
            server_id,
            declared_risk: ToolRiskLevel::None,
            effective_risk: ToolRiskLevel::None,
            user_visible: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinitionOverride {
    pub name: String,
    pub enabled: bool,
    pub model_name: Option<String>,
    pub user_visible: bool,
    pub ui_hint: Option<ToolUiHint>,
    pub description: Option<String>,
    pub renderer_hint: Option<ToolRendererHint>,
    pub declared_risk: Option<ToolRiskLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolRendererHint {
    Table,
    Diff,
    Terminal,
}

impl ToolRendererHint {
    pub fn new(value: impl Into<String>) -> Self {
        match value.into().as_str() {
            "table" => Self::Table,
            "diff" => Self::Diff,
            _ => Self::Terminal,
        }
    }

    pub fn with_detail_view(self, _detail_view: String) -> Self {
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolUiHint {
    Inline,
    Block,
}

impl ToolUiHint {
    pub fn new(value: impl Into<String>) -> Self {
        match value.into().as_str() {
            "inline" => Self::Inline,
            _ => Self::Block,
        }
    }
}

/// 通用扩展工具覆盖的测试投影，仅暴露生命周期契约所需的最终声明。
#[cfg(test)]
#[derive(Debug, Clone)]
pub struct McpToolSnapshot {
    pub model_name: Option<String>,
    pub user_visible: bool,
    pub ui_hint: Option<ToolUiHint>,
    pub description: Option<String>,
    pub renderer_hint: Option<ToolRendererHint>,
    pub declared_risk: Option<ToolRiskLevel>,
}

#[derive(Default)]
struct McpState {
    servers: std::collections::HashMap<String, MCPServerConfig>,
    tools: std::collections::HashMap<(String, String), ToolDefinition>,
    overrides: std::collections::HashMap<(String, String, String), ToolDefinitionOverride>,
}

/// MCP 容器（框架级，不绑定具体产品领域）。
pub struct MCP {
    sandbox_arc: std::sync::Arc<crate::security::sandbox::Sandbox>,
    state: std::sync::Mutex<McpState>,
}

impl MCP {
    pub fn new() -> Self {
        let event_bus: std::sync::Arc<dyn crate::kernel::EventBus> = std::sync::Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, tokio::runtime::Handle::current()),
        );
        Self {
            sandbox_arc: std::sync::Arc::new(crate::security::sandbox::Sandbox::new(event_bus)),
            state: std::sync::Mutex::new(McpState::default()),
        }
    }

    /// 构造生命周期测试使用的通用工具宿主。
    #[cfg(test)]
    pub fn init_for_test() -> Result<Self, String> {
        let host = Self::new();
        host.register_tool(ToolDefinition::new(
            "host.resource.read".to_string(),
            "读取宿主资源".to_string(),
            Value::Null,
            "builtin".to_string(),
        ))?;
        Ok(host)
    }

    pub fn sandbox(&self) -> &crate::security::sandbox::Sandbox {
        &self.sandbox_arc
    }

    pub fn inner(&self) -> &Self {
        self
    }

    pub fn add_server(&self, config: MCPServerConfig) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| format!("MCP 状态锁定失败: {error}"))?;
        if state.servers.contains_key(&config.id) {
            return Err(format!("MCP server '{}' 已存在", config.id));
        }
        state.servers.insert(config.id.clone(), config);
        Ok(())
    }

    pub fn start_server(&self, id: &str) -> Result<(), String> {
        let state = self
            .state
            .lock()
            .map_err(|error| format!("MCP 状态锁定失败: {error}"))?;
        if state.servers.contains_key(id) {
            Ok(())
        } else {
            Err(format!("MCP server '{id}' 不存在"))
        }
    }

    pub fn remove_server(&self, id: &str) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| format!("MCP 状态锁定失败: {error}"))?;
        if state.servers.remove(id).is_none() {
            return Err(format!("MCP server '{id}' 不存在"));
        }
        state.tools.retain(|(server_id, _), _| server_id != id);
        state.overrides.retain(|(_, server_id, _), _| server_id != id);
        Ok(())
    }

    pub fn register_tool(&self, tool: ToolDefinition) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| format!("MCP 状态锁定失败: {error}"))?;
        let key = (tool.server_id.clone(), tool.name.clone());
        if state.tools.contains_key(&key) {
            return Err(format!("MCP tool '{}:{}' 已存在", key.0, key.1));
        }
        state.tools.insert(key, tool);
        Ok(())
    }

    pub fn unregister_server_tools(&self, server_id: &str) -> Result<usize, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| format!("MCP 状态锁定失败: {error}"))?;
        let count = state.tools.len();
        state.tools.retain(|(owner, _), _| owner != server_id);
        let removed = count - state.tools.len();
        state.overrides.retain(|(_, owner, _), _| owner != server_id);
        Ok(removed)
    }

    pub fn apply_tool_override(
        &self,
        owner: &str,
        server_id: &str,
        tool_name: &str,
        override_: ToolDefinitionOverride,
    ) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| format!("MCP 状态锁定失败: {error}"))?;
        if !state
            .tools
            .contains_key(&(server_id.to_string(), tool_name.to_string()))
        {
            return Err(format!("MCP tool '{server_id}:{tool_name}' did not match registered tool"));
        }
        state.overrides.insert(
            (owner.to_string(), server_id.to_string(), tool_name.to_string()),
            override_,
        );
        Ok(())
    }

    pub fn remove_tool_override(
        &self,
        owner: &str,
        server_id: &str,
        tool_name: &str,
    ) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| format!("MCP 状态锁定失败: {error}"))?;
        state
            .overrides
            .remove(&(owner.to_string(), server_id.to_string(), tool_name.to_string()));
        Ok(())
    }

    /// 返回已登记服务器的测试快照。
    #[cfg(test)]
    pub fn list_servers(&self) -> Vec<MCPServerConfig> {
        self.state
            .lock()
            .expect("MCP 状态锁定失败")
            .servers
            .values()
            .cloned()
            .collect()
    }

    /// 返回工具最终覆盖声明的测试快照。
    #[cfg(test)]
    pub fn get_tool(&self, tool_name: &str) -> Option<McpToolSnapshot> {
        self.state
            .lock()
            .expect("MCP 状态锁定失败")
            .overrides
            .iter()
            .find(|((_, _, name), _)| name == tool_name)
            .map(|(_, override_)| McpToolSnapshot {
                model_name: override_.model_name.clone(),
                user_visible: override_.user_visible,
                ui_hint: override_.ui_hint.clone(),
                description: override_.description.clone(),
                renderer_hint: override_.renderer_hint.clone(),
                declared_risk: override_.declared_risk.clone(),
            })
    }
}

impl McpOperationPort for MCP {
    fn sandbox(&self) -> std::sync::Arc<crate::security::sandbox::Sandbox> {
        self.sandbox_arc.clone()
    }
}

impl crate::extension::lifecycle::McpCapabilityPort for MCP {
    fn add_server(&self, config: MCPServerConfig) -> anyhow::Result<()> {
        self.add_server(config).map_err(anyhow::Error::msg)
    }

    fn start_server(&self, id: &str) -> anyhow::Result<()> {
        self.start_server(id).map_err(anyhow::Error::msg)
    }

    fn remove_server(&self, id: &str) -> anyhow::Result<()> {
        self.remove_server(id).map_err(anyhow::Error::msg)
    }

    fn register_tool(&self, tool: ToolDefinition) -> anyhow::Result<()> {
        self.register_tool(tool).map_err(anyhow::Error::msg)
    }

    fn unregister_server_tools(&self, server_id: &str) -> anyhow::Result<usize> {
        self.unregister_server_tools(server_id).map_err(anyhow::Error::msg)
    }

    fn apply_tool_override(
        &self,
        owner: &str,
        server_id: &str,
        tool_name: &str,
        override_: ToolDefinitionOverride,
    ) -> anyhow::Result<()> {
        self.apply_tool_override(owner, server_id, tool_name, override_)
            .map_err(anyhow::Error::msg)
    }

    fn remove_tool_override(
        &self,
        owner: &str,
        server_id: &str,
        tool_name: &str,
    ) -> anyhow::Result<()> {
        self.remove_tool_override(owner, server_id, tool_name)
            .map_err(anyhow::Error::msg)
    }
}

// ============================================================
// LSP
// ============================================================

pub use crate::extension::models::{LanguageSource, LSPServerConfig};

/// LSP 注册表的只读测试视图。
pub struct LspRegistry<'a> {
    entries: &'a std::sync::Mutex<
        std::collections::HashMap<String, (LSPServerConfig, LanguageSource)>,
    >,
}

impl LspRegistry<'_> {
    pub fn get_config(&self, language_id: &str) -> Option<LSPServerConfig> {
        self.entries
            .lock()
            .ok()?
            .get(language_id)
            .map(|(config, _)| config.clone())
    }

    pub fn get_source(&self, language_id: &str) -> Option<LanguageSource> {
        self.entries
            .lock()
            .ok()?
            .get(language_id)
            .map(|(_, source)| source.clone())
    }
}

/// 通用语言服务注册宿主，仅维护扩展生命周期所需的声明索引。
pub struct LspManager {
    entries: std::sync::Mutex<std::collections::HashMap<String, (LSPServerConfig, LanguageSource)>>,
}

impl LspManager {
    pub fn new(_event_bus: std::sync::Arc<dyn crate::kernel::EventBus>) -> Result<Self, String> {
        let mut entries = std::collections::HashMap::new();
        entries.insert(
            "rust".to_string(),
            (
                LSPServerConfig {
                    language_id: "rust".to_string(),
                    language_names: vec!["Rust".to_string()],
                    file_extensions: vec![".rs".to_string()],
                    server_command: "rust-analyzer".to_string(),
                    server_args: Vec::new(),
                    initialization_options: None,
                    capabilities_required: Vec::new(),
                },
                LanguageSource::Builtin,
            ),
        );
        Ok(Self {
            entries: std::sync::Mutex::new(entries),
        })
    }

    pub fn registry(&self) -> LspRegistry<'_> {
        LspRegistry {
            entries: &self.entries,
        }
    }
}

impl crate::extension::lifecycle::LspCapabilityPort for LspManager {
    fn register_language(
        &self,
        config: LSPServerConfig,
        source: LanguageSource,
    ) -> anyhow::Result<()> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|error| anyhow::anyhow!("LSP 注册表锁定失败: {error}"))?;
        if let Some((_, current_source)) = entries.get(&config.language_id) {
            if current_source.is_builtin() {
                return Err(anyhow::anyhow!(
                    "builtin language '{}' cannot be overridden",
                    config.language_id
                ));
            }
            return Err(anyhow::anyhow!(
                "language '{}' is already registered",
                config.language_id
            ));
        }
        entries.insert(config.language_id.clone(), (config, source));
        Ok(())
    }

    fn unregister_language(&self, language_id: &str, owner: &str) -> anyhow::Result<()> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|error| anyhow::anyhow!("LSP 注册表锁定失败: {error}"))?;
        if matches!(entries.get(language_id), Some((_, LanguageSource::Extension { owner: current })) if current == owner)
        {
            entries.remove(language_id);
        }
        Ok(())
    }
}

// ============================================================
// Editor
// ============================================================

/// 通用后端服务生命周期测试桩，按扩展和服务 ID 维护运行登记。
pub struct BackendProcessManager {
    _sandbox: std::sync::Arc<crate::security::sandbox::Sandbox>,
    running: std::sync::Mutex<std::collections::BTreeSet<(String, String)>>,
}

impl BackendProcessManager {
    pub fn new(sandbox: std::sync::Arc<crate::security::sandbox::Sandbox>) -> Self {
        Self {
            _sandbox: sandbox,
            running: std::sync::Mutex::new(std::collections::BTreeSet::new()),
        }
    }

    #[cfg(test)]
    pub fn is_running(&self, extension_id: &str, service_id: &str) -> bool {
        self.running
            .lock()
            .map(|running| running.contains(&(extension_id.to_string(), service_id.to_string())))
            .unwrap_or(false)
    }

    #[cfg(test)]
    pub fn list(&self, extension_id: Option<&str>) -> Vec<(String, String)> {
        self.running
            .lock()
            .map(|running| {
                running
                    .iter()
                    .filter(|(owner, _)| extension_id.is_none_or(|id| id == owner))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    #[cfg(test)]
    pub fn kill_all_for_extension(&self, extension_id: &str) {
        <Self as crate::extension::lifecycle::cordis::BackendProcessPort>::kill_all_for_extension(
            self,
            extension_id,
        );
    }
}

impl crate::extension::lifecycle::cordis::BackendProcessPort for BackendProcessManager {
    fn spawn_for_lifecycle(
        &self,
        _store: &crate::extension::store::ExtensionStore,
        extension_id: &str,
        service: &crate::extension::models::BackendServiceRegistration,
    ) -> Result<String, String> {
        if service.id.trim().is_empty() {
            return Err("后端服务 ID 不能为空".to_string());
        }
        self.running
            .lock()
            .map_err(|error| format!("后端服务状态锁定失败: {error}"))?
            .insert((extension_id.to_string(), service.id.clone()));
        Ok(format!("{extension_id}/{}", service.id))
    }

    fn kill_all_for_extension(&self, extension_id: &str) {
        if let Ok(mut running) = self.running.lock() {
            running.retain(|(owner, _)| owner != extension_id);
        }
    }
}

pub struct PathManager;
impl PathManager {
    pub fn normalize(p: &std::path::Path) -> String { p.display().to_string() }
    pub fn resolve(base: &std::path::Path, path: &std::path::Path) -> std::path::PathBuf { base.join(path) }
}

// ============================================================
// Session
// ============================================================

pub struct SessionStore;

pub struct MemoryStore;
impl MemoryStore {
    pub fn new() -> Self { Self }
    pub fn save(&self, _m: &()) -> Result<(), String> { Ok(()) }
    pub fn search(&self, _q: &str) -> Result<Vec<()>, String> { Ok(vec![]) }
}

// ============================================================
// Tool
// ============================================================

pub const NAVIS_TOOL_SEARCH: &str = "tool_search";

// ============================================================
// UI
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiAgentTimelinePart {
    pub id: String, pub session_id: String, pub turn_id: String,
    pub message_id: String, pub sequence: i64, pub kind: String, pub data: Value,
}
