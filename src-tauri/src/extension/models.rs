//! Extension 数据模型定义
//!
//! 基于设计文档 §07 三、数据模型 实现，定义扩展系统的所有数据结构。
//!
//! 包含：ExtensionManifest、ExtensionPermissions、ExtensionContributes、ExtensionState、
//! 各类 Registration 结构体及相关枚举。

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::foundation::status::{StatusClassify, StatusPresentation};
use crate::kernel::{Constraint, PolicyDecision, PolicyInput, SchemaVersion};
use crate::security::auth::SecretValue;

// ============================================================================
// ExtensionManifest - 扩展清单
// ============================================================================

/// 扩展清单（对应 extension.json）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtensionManifest {
    /// 扩展唯一 ID（如 "com.example.my-extension"）
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 版本号
    pub version: String,
    /// 描述
    pub description: String,
    /// 作者
    pub author: String,
    /// 权限声明
    pub permissions: ExtensionPermissions,
    /// 贡献点（扩展点声明）
    pub contributes: ExtensionContributes,
}

// ============================================================================
// ExtensionPermissions - 权限声明
// ============================================================================

/// 扩展权限声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtensionPermissions {
    /// 路径权限（如 "read:./src/**"）
    pub filesystem: Vec<String>,
    /// 命令权限（如 "npm", "git"）
    pub terminal: Vec<String>,
    /// 网络权限（如 "https://api.example.com"）
    pub network: Vec<String>,
    /// 允许调用的 IPC 命令（如 "agent.cancelTask"）
    pub ipc: Vec<String>,
    /// 允许订阅的 EventBus pattern（如 "agent.*"）
    pub events: Vec<String>,
    /// 资源配额
    pub resources: ResourceLimits,
}

impl Default for ExtensionPermissions {
    fn default() -> Self {
        Self {
            filesystem: Vec::new(),
            terminal: Vec::new(),
            network: Vec::new(),
            ipc: Vec::new(),
            events: Vec::new(),
            resources: ResourceLimits::default(),
        }
    }
}

// ============================================================================
// ResourceLimits - 资源配额
// ============================================================================

/// 资源配额限制
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceLimits {
    /// 最大内存（MB），如 512
    pub max_memory_mb: u64,
    /// 最大 CPU 占用（%），如 50.0
    pub max_cpu_percent: f32,
    /// 执行超时（毫秒），如 30000
    pub timeout_ms: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 512,
            max_cpu_percent: 50.0,
            timeout_ms: 30_000,
        }
    }
}

// ============================================================================
// ExtensionContributes - 贡献点（扩展点声明）
// ============================================================================

/// 扩展贡献点声明
///
/// 对应 extension.json 中的 "contributes" 字段，声明扩展提供的各类扩展能力。
/// 本结构是 manifest DTO，不是运行能力事实源；启用扩展时只有具备宿主落点
/// 的贡献会登记到 MCP、Skills、UI 或 Agent hook 等宿主子系统。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ExtensionContributes {
    /// MCP Server 声明；启用时通过 MCP 宿主注册 server。
    pub mcp_servers: Option<Vec<MCPServerConfig>>,
    /// MCP 工具属性覆盖；启用时通过 MCP 工具注册表应用。
    pub mcp_tool_overrides: Option<Vec<McpToolOverride>>,
    /// MCP 工具声明；启用时通过 MCP ToolRegistry 注册为平台工具。
    pub tools: Option<Vec<ToolRegistration>>,
    /// Skill 声明；启用时进入 Skills 子系统。
    pub skills: Option<Vec<SkillDefinition>>,
    /// 角色声明；当前仅作为 manifest DTO，等待 Role 宿主落点。
    pub roles: Option<Vec<RoleDefinition>>,
    /// UI 视图声明；UI 域读取已启用扩展 manifest 渲染，不是 Kernel capability。
    pub views: Option<Vec<ViewRegistration>>,
    /// 菜单声明；UI 域读取已启用扩展 manifest 渲染。
    pub menus: Option<Vec<MenuRegistration>>,
    /// 命令声明；当前只支持 UI/内建声明式动作，不是扩展层运行命令 registry。
    pub commands: Option<Vec<CommandRegistration>>,
    /// 快捷键声明；当前作为 UI/热键宿主待接入 DTO。
    pub keybindings: Option<Vec<KeybindingRegistration>>,
    /// 配置项声明；当前作为配置宿主待接入 DTO。
    pub configuration: Option<Value>,
    /// Custom 完整工作模式声明（显示在 leftSidebar 的 Custom 页签下）。
    pub work_modes: Option<Vec<WorkModeRegistration>>,
    /// 工具栏按钮
    pub toolbar_items: Option<Vec<ToolbarItemRegistration>>,
    /// 状态栏项目
    pub statusbar_items: Option<Vec<StatusBarItemRegistration>>,
    /// 视图内嵌组件
    pub inline_extensions: Option<Vec<InlineExtensionRegistration>>,
    /// Gateway Adapter 与 Provider 声明；启用时由 Gateway capability port 以事务方式注册。
    pub gateway: Option<GatewayContributions>,
    /// Gateway 请求中间件声明；运行时必须由 Gateway/kernel Pipeline 宿主接入。
    pub middlewares: Option<Vec<MiddlewareRegistration>>,
    /// 自定义 MCP 传输适配器声明；运行时必须转换为 MCP TransportAdapter 后进入 MCP 宿主。
    pub transport_adapters: Option<Vec<TransportAdapterRegistration>>,
    /// 自定义语言 LSP Server 配置声明；运行时应进入 LSP 宿主注册入口。
    pub languages: Option<Vec<LanguageRegistration>>,
    /// 自定义编辑器主题声明。
    pub themes: Option<Vec<ThemeRegistration>>,
    /// 自定义语言模式/语法声明。
    pub editor_languages: Option<Vec<EditorLanguageRegistration>>,
    /// 自定义编辑器扩展声明。
    pub editor_extensions: Option<Vec<EditorExtensionRegistration>>,
    /// 自定义托盘菜单项声明。
    pub tray_items: Option<Vec<TrayItemRegistration>>,
    /// 自定义通知渠道声明；运行时应进入通知宿主注册入口。
    pub notification_channels: Option<Vec<NotificationChannelRegistration>>,
    /// 自定义输入快捷触发器（建议 /xxx；扩展/能力引用主路径仍是 + 菜单）
    pub triggers: Option<Vec<TriggerRegistration>>,
    /// 自定义 CSS 样式声明。
    pub styles: Option<Vec<StyleRegistration>>,
    /// 布局覆盖声明。
    pub layout_overrides: Option<Vec<LayoutOverrideRegistration>>,
    /// 事件驱动的 UI 行为声明。
    pub behaviors: Option<Vec<BehaviorRegistration>>,
    /// Agent 管道钩子声明；启用后只进入 ExtensionStore 声明索引，由宿主 runner 执行。
    pub hooks: Option<Vec<HookRegistration>>,
    /// 自定义上下文数据源声明；运行时应进入 Context 宿主。
    pub context_providers: Option<Vec<ContextProviderRegistration>>,
    /// 自定义搜索源声明。
    pub search_providers: Option<Vec<SearchProviderRegistration>>,
    /// 文件变更监听声明；运行时应进入文件监听宿主。
    pub file_watchers: Option<Vec<FileWatcherRegistration>>,
    /// Kernel EventBus 订阅声明；等待 Extension runtime handler 入口落地后注册。
    #[serde(rename = "eventSubscriptions")]
    pub event_subscriptions: Option<Vec<EventSubscriptionRegistration>>,
    /// 扩展 KV 存储声明（34 §2.5）。
    pub storage: Option<StorageDeclaration>,
    /// 扩展网络策略声明（34 §2.6），未声明时默认 None/fail-closed。
    pub network: Option<NetworkPolicy>,
    /// 扩展本地化资源（34 §10.4 / 28-i18n）。
    pub i18n: Option<Vec<I18nResource>>,
    /// 白名单授权声明（34 §3.4）：iframe/Worker 桥调用的能力白名单。
    /// 未声明任何能力时，扩展 UI 只能做纯静态渲染。
    pub capabilities: Option<CapabilityDeclaration>,
    /// 跨扩展面显式导出（34 §2.4）：仅被显式列出的 view/command 才可被其他扩展调用。
    #[serde(rename = "extensionExports")]
    pub extension_exports: Option<ExtensionExports>,
    /// 能力标签（34 §2.7），供发现机制检索。
    pub provides: Option<Vec<String>>,
    /// 逻辑轨脚本声明（34 §3.4）：Web Worker 承载。
    pub scripts: Option<Vec<ScriptRegistration>>,
    /// 扩展自定义 zone 声明（34 §2.2）：含锚定语义。
    pub zones: Option<Vec<ZoneRegistration>>,
    /// 后端扩展服务声明（35 §3.4）：独立进程运行，容器 spawn/kill，经协议通信。
    #[serde(rename = "backendServices")]
    #[serde(default)]
    pub backend_services: Option<Vec<BackendServiceRegistration>>,
    /// WASM 组件轨声明（37 §5.1）：逻辑扩展统一编译为 wasm32-wasip2 组件，
    /// entry 为 .wasm 相对路径，必须位于 ExtensionUI/ 或 ExtensionBackend/ 下。
    #[serde(rename = "components")]
    #[serde(default)]
    pub components: Option<Vec<ComponentRegistration>>,
}

// ============================================================================
// EventBus 订阅声明
// ============================================================================

/// Extension 事件订阅声明。
///
/// 这是 manifest DTO，不是可执行 handler。`handler` 只描述未来 runtime
/// 如何解析入口；在 Extension runtime contract 落地前，lifecycle 必须
/// fail-closed，不得把它直接转换成 Kernel EventBus handler。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EventSubscriptionRegistration {
    /// Extension 内唯一订阅 ID。
    pub id: String,
    /// Kernel EventBus 精确 topic。
    pub topic: String,
    /// 可选的 Kernel scope key。
    #[serde(rename = "scopeKey")]
    pub scope_key: Option<String>,
    /// Extension runtime handler 入口引用。
    pub handler: EventHandlerReference,
}

/// Extension runtime handler 入口引用。
///
/// `module` 和 `export` 仅作为受控 DTO 传递，不允许 lifecycle 自行加载
/// 任意脚本或执行任意函数。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EventHandlerReference {
    /// 经 Extension runtime 解析的模块标识。
    pub module: String,
    /// 模块导出的 handler 名称。
    pub export: String,
}

/// Extension runtime 接收的只读事件 DTO。
///
/// 该 DTO 隔离 Kernel `EventEnvelope`，避免把 Kernel context、共享指针和
/// EventBus 实现细节泄漏到 Extension runtime。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExtensionEventDto {
    pub id: String,
    pub topic: String,
    pub version: SchemaVersion,
    #[serde(rename = "scopeKey")]
    pub scope_key: String,
    pub source: String,
    pub payload: Option<Value>,
    pub created_at: DateTime<Utc>,
}
// ============================================================================
// MCP 相关
// ============================================================================

/// MCP Server 配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MCPServerConfig {
    /// Server 名称
    pub name: String,
    /// Server 配置
    #[serde(flatten)]
    pub config: Value,
}

/// MCP 工具属性覆盖
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpToolOverride {
    /// MCP Server 名称
    pub server: String,
    /// 工具名称
    pub tool: String,
    /// 模型可见 provider-safe 名称；为空时由 Tool Catalog 从 MCP canonical name 生成。
    pub model_name: Option<String>,
    /// 是否对用户可见
    pub user_visible: Option<bool>,
    /// 显示名称覆盖
    pub display_name: Option<String>,
    /// 描述覆盖
    pub description: Option<String>,
    /// 对话区 renderer 注册 ID。
    pub renderer: Option<String>,
    /// renderer 的详情视图语义。
    pub detail_view: Option<String>,
    /// Provider 自声明风险，平台最终会取声明风险和强制风险的较高值。
    pub declared_risk: Option<String>,
}

/// 扩展 MCP 工具声明
///
/// 启用时由 `ExtensionLifecycle` 通过 MCP ToolRegistry 注册为平台工具，
/// server_id 自动设为 `extension:{extension_id}`。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolRegistration {
    /// 工具唯一名称（同一扩展内不可重复）
    pub name: String,
    /// 工具描述（供模型选择参考）
    pub description: String,
    /// JSON Schema 输入参数定义
    #[serde(default)]
    pub input_schema: Value,
    /// 是否对用户可见（默认 true）
    #[serde(default = "default_true")]
    pub user_visible: bool,
    /// 是否使用内置风险评估（默认 true）
    #[serde(default = "default_true")]
    pub use_builtin_risk: bool,
    /// Provider 自声明风险等级（Low/Medium/High/Critical）
    pub declared_risk: Option<String>,
}

fn default_true() -> bool {
    true
}

// ============================================================================
// Skills / Roles
// ============================================================================

/// Skill 定义
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillDefinition {
    /// Skill ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 描述
    pub description: Option<String>,
    /// 配置
    #[serde(flatten)]
    pub config: Value,
}

/// Role 定义
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoleDefinition {
    /// Role ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 描述
    pub description: Option<String>,
    /// 配置
    #[serde(flatten)]
    pub config: Value,
}

// ============================================================================
// Work Mode 声明
// ============================================================================

/// Custom 模式扩展声明。
///
/// 这不是普通菜单项，而是完整工作模式的 ModeConfig overlay。点击 Custom
/// 页签下的某个模式扩展后，当前会话进入 custom:<runtime_id>，
/// 其中 runtime_id = <extension_id>/<mode_id>。Agent 按该声明重新加载角色、
/// 工具白名单、技能、命令、上下文策略、模型偏好和默认 UI 入口。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkModeRegistration {
    /// 模式 ID，扩展内唯一，如 "knowledge-search"
    pub id: String,
    /// 模式显示名；缺省使用 ExtensionManifest.name
    pub name: Option<String>,
    /// 模式说明
    pub description: Option<String>,
    /// Lucide icon 名或扩展图标路径
    pub icon: Option<String>,
    /// 默认角色，引用内建或扩展 contributes.roles 中的 RoleDefinition.id
    pub role: Option<String>,
    /// 模式工具白名单，支持 read/write/edit/bash/git/lsp/mcp.* 等工具 ID 或通配符
    pub available_tools: Option<Vec<String>>,
    /// 默认技能集，引用内建、项目、用户或扩展 contributes.skills 中的 skill id
    pub skills: Option<Vec<String>>,
    /// 模式优先命令，引用 contributes.commands 或系统命令 id
    pub commands: Option<Vec<String>>,
    /// 上下文策略 ID，引用 context_providers 或内建策略
    pub context_policy: Option<String>,
    /// 模式行为约束，注入 Agent system prompt 的规则片段
    pub behavior_rules: Option<Vec<String>>,
    /// 进入模式时默认打开的 view id
    pub entry_view: Option<String>,
    /// 进入模式时建议打开的右侧面板区 view id 列表
    pub default_views: Option<Vec<String>>,
    /// 该模式建议模型，最终仍写入 Session 模型偏好
    pub default_model: Option<String>,
    /// temperature、max_tokens、extended_thinking 等模式模型偏好
    pub model_preferences: Option<WorkModeModelPreferences>,
    /// 能力标签，如 ["rag", "visualization"]
    #[serde(default)]
    pub capabilities: Vec<String>,
}

/// Work mode 模型偏好。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct WorkModeModelPreferences {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub extended_thinking: Option<bool>,
    pub language_quality_emphasis: Option<f64>,
}

// ============================================================================
// Capability 白名单 / 跨扩展导出 / 脚本 / Zone（34 §2.x/§3.4）
// ============================================================================

/// 扩展 UI 白名单授权声明。
///
/// iframe/Worker 桥只允许调用这里显式声明的能力。未声明任何能力时，
/// 扩展 UI 只能做纯静态渲染；任何越权请求 fail-closed 并审计。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CapabilityDeclaration {
    /// 允许调用的宿主 IPC 命令白名单（需已注册）。
    #[serde(default)]
    pub invoke: Vec<String>,
    /// 允许订阅的宿主 UI event/stream 只读投影 pattern。
    #[serde(default)]
    pub events: Vec<String>,
    /// 允许读取的上下文快照键（session/project/context 等）。
    #[serde(default)]
    pub read: Vec<String>,
    /// 跨扩展调用白名单（34 §2.4）。
    #[serde(rename = "extensionCalls")]
    #[serde(default)]
    pub extension_calls: Vec<ExtensionCall>,
    /// 能力标签，供发现机制检索（34 §2.7）。
    #[serde(default)]
    pub provides: Vec<String>,
    /// 网络能力声明；兼容 34 中 capabilities.network 的写法。
    #[serde(default)]
    pub network: Option<NetworkPolicy>,
}

/// 状态存储声明（34 §2.5）。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StorageDeclaration {
    #[serde(default)]
    pub scopes: Vec<StorageScope>,
}

/// 扩展 KV scope。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum StorageScope {
    Global,
    Worktree,
    Ephemeral,
}

/// 网络策略声明（34 §2.6）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum NetworkPolicy {
    None,
    Allowlist {
        #[serde(default)]
        hosts: Vec<NetworkHost>,
    },
    Proxy,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self::None
    }
}

/// 网络 allowlist host 条目。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NetworkHost {
    pub host: String,
    #[serde(rename = "allowSubdomains", default)]
    pub allow_subdomains: bool,
    #[serde(default)]
    pub protocols: Vec<String>,
}

/// i18n 资源声明（34 §10.4 / 28-i18n）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct I18nResource {
    pub lang: String,
    pub entry: String,
}

/// 跨扩展调用白名单条目。
///
/// `actions` 取值：`view.open` / `command.execute` / `event.emit` / `event.subscribe`。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExtensionCall {
    pub target: String,
    #[serde(default)]
    pub actions: Vec<String>,
}

/// 跨扩展面显式导出。
///
/// 仅被显式列出的 view/command 才可被其他扩展经 ExtRouter 调用。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExtensionExports {
    #[serde(default)]
    pub views: Vec<String>,
    #[serde(default)]
    pub commands: Vec<String>,
}

/// 逻辑轨脚本声明（Web Worker 承载）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ScriptRegistration {
    /// 扩展内唯一脚本 ID。
    pub id: String,
    /// 相对扩展安装目录的 Worker entry（ui/ 或 scripts/）。
    pub entry: String,
    /// 运行时机；当前仅投影给前端 worker runtime。
    #[serde(rename = "runOn", default)]
    pub run_on: Option<Vec<String>>,
}

/// 扩展自定义 zone 声明（34 §2.2）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ZoneRegistration {
    /// 扩展内唯一 zone ID。
    pub id: String,
    /// zone 显示名。
    pub name: String,
    /// 锚定语义：依赖的已知 zone 或 {extId}:{zoneId}。
    pub anchor: ZoneAnchor,
}

/// zone 锚定语义。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ZoneAnchor {
    /// 父 zone 引用（已知 zone 名或 {extId}:{zoneId}）。
    pub parent: String,
    /// 在父 zone 内的锚点位置。
    pub position: Option<String>,
    /// 默认尺寸。
    pub size: Option<SizeValue>,
}

/// 后端扩展服务声明（35 §3.4）。
/// 独立进程运行，容器 spawn/kill，经协议通信。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BackendServiceRegistration {
    /// 服务 ID（扩展内唯一）
    pub id: String,
    /// 可执行文件相对路径（位于扩展 ExtensionBackend/ 目录）
    pub entry: String,
    /// 传输方式
    #[serde(default)]
    pub transport: BackendTransport,
    /// 协议
    #[serde(default)]
    pub protocol: BackendProtocol,
    /// 启动参数（可选，追加到可执行文件后）
    #[serde(default)]
    pub args: Vec<String>,
    /// 环境变量（可选）
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// 是否随容器启动自动 spawn（默认 false，按需 spawn）
    #[serde(default)]
    pub autostart: bool,
}

/// 后端扩展服务传输方式。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BackendTransport {
    #[default]
    Stdio,
    Sse,
    WebSocket,
}

/// 后端扩展服务通信协议。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BackendProtocol {
    #[default]
    JsonRpc,
}

// ============================================================================
// WASM 组件轨声明（37 §5.1）
// ============================================================================

/// WASM 组件注册声明（37 §5.1）。
///
/// 逻辑扩展统一编译为 WASM 组件（wasm32-wasip2），容器内 wasmtime 实例化执行。
/// `capabilities` 是能力声明白名单，容器实例化时按声明映射注入 host 接口实现；
/// 未声明 = 不注入该接口 = 调用即失败并审计（fail-closed）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ComponentRegistration {
    /// 组件 ID（扩展内唯一）
    pub id: String,
    /// .wasm 相对路径（位于 ExtensionUI/ 或 ExtensionBackend/ 下）
    pub entry: String,
    /// 组件类型：logic（容器内组件轨）| native（逃生舱，走协议子进程）
    #[serde(default)]
    pub kind: ComponentKind,
    /// 运行时机；缺省为空（按需激活）
    #[serde(rename = "runOn")]
    #[serde(default)]
    pub run_on: Vec<String>,
    /// 能力声明白名单：映射为 host 接口授予
    pub capabilities: ComponentCapabilities,
    /// 是否随容器启动自动激活（默认 false，按需激活）
    #[serde(default)]
    pub autostart: bool,
}

/// WASM 组件类型。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ComponentKind {
    /// 容器内组件轨（wasmtime 实例化执行）
    #[default]
    Logic,
    /// native 逃生舱（协议子进程，可省组件字段改走 backendServices）
    Native,
}

/// WASM 组件能力声明白名单。
///
/// 容器实例化时按声明映射注入对应 host 接口实现；未声明的接口不注入，
/// 组件调用即失败并审计（fail-closed）。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ComponentCapabilities {
    /// 允许调用的 host 操作/上下文命令白名单（对应 operation/context host 接口）。
    #[serde(default)]
    pub invoke: Vec<String>,
    /// 允许访问的存储 scope 白名单（对应 storage host 接口）。
    #[serde(default)]
    pub storage: Vec<String>,
    /// 网络能力声明（对应 network host 接口）；缺省 None = fail-closed。
    #[serde(default)]
    pub network: Option<serde_json::Value>,
    /// 允许订阅的 EventBus pattern 白名单（对应 event host 接口）。
    #[serde(default)]
    pub events: Vec<String>,
}

// ============================================================================
// View 声明
// ============================================================================

/// 视图注册声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ViewRegistration {
    /// 视图唯一 ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 图标
    pub icon: Option<String>,
    /// renderer 的静态入口文件，相对于扩展安装目录。
    /// host:panel 不需要入口；html:sandbox 必须声明入口。
    #[serde(default)]
    pub entry: Option<String>,
    /// 开放 zone 名：内置 zone 或 `{extId}:{zoneId}`。
    #[serde(default)]
    pub zone: Option<String>,
    /// 已弃用：兼容旧 manifest 的 placement；宿主投影时 fallback 到 zone。
    #[serde(default)]
    pub placement: Option<String>,
    /// 宿主 Host view renderer ID，如 `host:panel`。
    pub renderer: String,
    /// renderer 专用配置，只由 UI 域解释，不进入 Kernel。
    #[serde(default)]
    pub config: Option<Value>,
    /// 激活条件
    pub activation_events: Vec<String>,
    /// 是否允许用户关闭
    pub allow_close: Option<bool>,
    /// 默认是否可见
    pub default_visible: Option<bool>,
}

/// UI 沙箱模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum UISandboxMode {
    /// Shadow DOM 隔离（默认）
    ShadowDom,
    /// iframe 隔离
    Iframe,
    /// 无隔离
    None,
}

impl Default for UISandboxMode {
    fn default() -> Self {
        Self::ShadowDom
    }
}

// ============================================================================
// Menu 声明
// ============================================================================

/// 菜单注册声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MenuRegistration {
    /// 菜单项 ID
    pub id: String,
    /// 显示标签
    pub label: String,
    /// 菜单位置：内置 target 或 `{extId}:{targetId}` 开放命名空间。
    pub target: String,
    /// 关联的命令 ID
    pub command: String,
    /// 分组名
    pub group: Option<String>,
    /// 条件表达式
    pub when: Option<String>,
    /// 图标
    pub icon: Option<String>,
    /// 快捷键提示文本
    pub shortcut: Option<String>,
    /// 菜单项风险等级，用于危险操作着色和二次确认。
    pub risk: Option<MenuRisk>,
}

/// 菜单项风险等级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MenuRisk {
    Low,
    Medium,
    High,
}

/// 内置菜单目标常量。Manifest `target` 已开放为字符串；这里仅供宿主内置菜单复用。
pub mod menu_targets {
    pub const TOOLS: &str = "Tools";
    pub const INPUT_PLUS: &str = "InputPlus";
    pub const CHAT_TITLE: &str = "ChatTitle";
    pub const RIGHT_PANEL: &str = "RightPanel";
    pub const GATEWAY: &str = "Gateway";
    pub const WORKTREE_CONTEXT: &str = "WorktreeContext";
    pub const SESSION_CONTEXT: &str = "SessionContext";
}

// ============================================================================
// Command 声明
// ============================================================================

/// 命令注册声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandRegistration {
    /// 命令 ID
    pub id: String,
    /// 显示标题
    pub label: String,
    /// 描述
    pub description: Option<String>,
    /// 图标
    pub icon: Option<String>,
    /// 分类
    pub category: Option<String>,
    /// 条件表达式
    pub when: Option<String>,
    /// 内置声明式动作。命令必须通过宿主已有 action contract 执行。
    pub action: BuiltinAction,
}

/// 内置声明式动作
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum BuiltinAction {
    /// 打开扩展视图（本扩展 view id 或跨扩展 `{extId}:{viewId}`）。
    OpenView {
        #[serde(alias = "view")]
        view_id: String,
    },
    /// 切换扩展视图显隐。
    ToggleView {
        #[serde(alias = "view")]
        view_id: String,
    },
    /// 打开扩展视图对话框。
    OpenDialog {
        #[serde(alias = "view")]
        view_id: String,
        #[serde(default)]
        size: Option<String>,
        #[serde(default)]
        position: Option<String>,
        #[serde(default)]
        modal: Option<bool>,
    },
    /// 触发扩展自身 Web Worker 逻辑轨脚本。
    RunScript {
        #[serde(alias = "script")]
        script_id: String,
        #[serde(default, alias = "payload")]
        args: Option<Value>,
    },
    /// 通过 ExtRouter 向另一个扩展/宿主命名空间发送消息。
    SendMessage {
        target: String,
        #[serde(default, alias = "message")]
        payload: Value,
    },
}

// ============================================================================
// Keybinding 声明
// ============================================================================

/// 快捷键注册声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeybindingRegistration {
    /// 关联的命令 ID
    pub command: String,
    /// 按键组合
    pub key: String,
    /// 生效条件
    pub when: Option<String>,
    /// 作用范围
    pub scope: KeybindingScope,
}

/// 快捷键作用范围
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum KeybindingScope {
    /// 应用级（扩展只能注册此范围）
    App,
}

// ============================================================================
// Toolbar / StatusBar / Inline 注册
// ============================================================================

/// 工具栏按钮声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolbarItemRegistration {
    /// 按钮 ID
    pub id: String,
    /// Tooltip 文本
    pub label: String,
    /// 图标
    pub icon: String,
    /// 关联的命令 ID
    pub command: String,
    /// 目标工具栏
    pub position: ToolbarPosition,
    /// 分组名
    pub group: Option<String>,
    /// 条件表达式
    pub when: Option<String>,
}

/// 工具栏位置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum ToolbarPosition {
    Main,
    Editor,
    Terminal,
}

/// 状态栏项目声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatusBarItemRegistration {
    /// 项目 ID
    pub id: String,
    /// 显示文本
    pub label: String,
    /// 图标
    pub icon: Option<String>,
    /// 左侧或右侧
    pub position: StatusBarPosition,
    /// 点击关联的命令 ID
    pub command: Option<String>,
    /// 排序优先级
    pub priority: Option<u32>,
    /// 条件表达式
    pub when: Option<String>,
}

/// 状态栏位置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum StatusBarPosition {
    Left,
    Right,
}

/// 视图内嵌组件声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InlineExtensionRegistration {
    /// 扩展 ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 目标视图
    pub target: InlineTarget,
    /// 在目标视图中的位置
    pub position: InlinePosition,
    /// ES Module 路径
    pub component: String,
    /// 同一位置最多渲染几个组件
    pub max_items: Option<u32>,
    /// 排序优先级
    pub priority: Option<u32>,
    /// 默认是否可见
    pub visible: Option<bool>,
    /// 条件表达式
    pub when: Option<String>,
    /// 沙箱隔离模式
    pub sandbox: UISandboxMode,
}

/// 内嵌组件目标视图
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum InlineTarget {
    Chat,
    Editor,
    Terminal,
}

/// 内嵌组件位置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum InlinePosition {
    BeforeInput,
    AfterMessages,
    Sidebar,
    Top,
    Bottom,
}

// ============================================================================
// Provider / Middleware / Transport
// ============================================================================

/// Gateway 扩展贡献。Adapter 描述协议转换，Provider 描述运行实例。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GatewayContributions {
    #[serde(default)]
    pub adapters: Vec<GatewayAdapterRegistration>,
    #[serde(default)]
    pub providers: Vec<GatewayProviderRegistration>,
}

/// Gateway 协议 Adapter 声明。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GatewayAdapterRegistration {
    pub id: String,
    pub name: String,
    #[serde(rename = "protocolId")]
    pub protocol_id: String,
    pub kind: String,
    #[serde(default)]
    pub config: Value,
}

/// Gateway Provider 实例声明。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GatewayProviderRegistration {
    pub id: String,
    pub name: String,
    #[serde(rename = "adapterId")]
    pub adapter_id: String,
    #[serde(rename = "baseUrl")]
    pub base_url: String,
    pub auth: GatewayAuthRegistration,
    /// Provider-level capability source. Model declarations remain separate
    /// inputs and are intersected by the Gateway evaluator at request time.
    pub capabilities: ProviderCapabilities,
    /// Provider secret 的受控验证声明。没有该声明的 Provider 不具备验证入口。
    pub validation: GatewayProviderValidationRegistration,
    pub models: Vec<GatewayModelRegistration>,
    #[serde(rename = "defaultModel")]
    pub default_model: String,
}

/// Extension-owned Provider secret 验证声明。
///
/// 该声明只描述宿主可执行的最小 HTTP 验证请求和状态映射，不允许扩展
/// 注入任意 URL、header 模板或脚本。认证头由 `ProviderAuthProfile` 统一构造。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GatewayProviderValidationRegistration {
    /// 相对于 Provider baseUrl 的验证 endpoint。
    pub endpoint: String,
    /// 表示凭据已被真实协议请求验证成功的 HTTP 状态码。
    #[serde(rename = "validStatusCodes")]
    pub valid_status_codes: Vec<u16>,
    /// 表示服务明确拒绝凭据的 HTTP 状态码。
    #[serde(rename = "invalidStatusCodes")]
    pub invalid_status_codes: Vec<u16>,
    /// 单次验证的最大执行时间（毫秒）。超时统一返回 Unknown。
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: u64,
}

/// Gateway Provider 认证声明。secretRef 只引用 Auth Store 中的 opaque ref。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GatewayAuthRegistration {
    pub scheme: String,
    #[serde(rename = "secretRef")]
    pub secret_ref: Option<String>,
    pub header: String,
}

/// Gateway 模型声明。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GatewayModelRegistration {
    pub id: String,
    pub name: String,
    pub capabilities: ProviderCapabilities,
    #[serde(rename = "contextWindow")]
    pub context_window: u32,
    #[serde(rename = "maxOutputTokens")]
    pub max_output_tokens: u32,
}

/// Adapter、Provider 和 Model 共用的声明能力。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct ProviderCapabilities {
    #[serde(rename = "tools", default)]
    pub supports_tools: bool,
    #[serde(rename = "streaming", default)]
    pub supports_streaming: bool,
    #[serde(rename = "multimodal", default)]
    pub supports_multimodal: bool,
    #[serde(rename = "reasoning", default)]
    pub supports_reasoning_effort: bool,
    #[serde(rename = "structuredOutput", default)]
    pub supports_structured_output: bool,
    #[serde(rename = "usage", default)]
    pub supports_usage: bool,
}

/// Gateway 中间件声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MiddlewareRegistration {
    /// 中间件 ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 执行阶段
    pub phase: MiddlewarePhase,
    /// 实现模块路径
    pub module: String,
}

/// 中间件执行阶段
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum MiddlewarePhase {
    PreRequest,
    PostResponse,
    Error,
}

/// MCP 传输适配器声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransportAdapterRegistration {
    /// 适配器 ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 传输类型标识
    pub transport_type: String,
    /// 实现模块路径
    pub module: String,
}

// ============================================================================
// LSP 语言声明
// ============================================================================

/// LSP 语言声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LanguageRegistration {
    /// 语言 ID
    pub language_id: String,
    /// 显示名称
    pub display_name: String,
    /// 文件扩展名
    pub extensions: Vec<String>,
    /// LSP Server 启动命令
    pub server_command: String,
    /// 启动参数
    pub server_args: Option<Vec<String>>,
    /// 初始化配置
    pub initialization_options: Option<Value>,
}

// ============================================================================
// Editor 主题 / 语言 / 扩展
// ============================================================================

/// 编辑器主题声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThemeRegistration {
    /// 主题 ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 主题类型
    #[serde(rename = "type")]
    pub theme_type: ThemeType,
    /// ES Module 路径
    pub module: String,
}

/// 主题类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum ThemeType {
    Light,
    Dark,
    HighContrast,
}

/// 编辑器语言模式声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditorLanguageRegistration {
    /// 语言模式 ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 文件扩展名
    pub extensions: Vec<String>,
    /// ES Module 路径
    pub module: String,
}

/// LSP Server 配置（扩展域通用契约，原 tool::lsp::registry 下沉）。
///
/// 描述一个语言的 LSP Server 启动参数和关联信息。tool::lsp 域消费此契约，
/// extension 域通过 `contributes.languages` 投影生成，两侧不互相依赖。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LSPServerConfig {
    /// 语言标识符（如 "typescript", "python"）
    pub language_id: String,
    /// 语言名称列表（如 ["typescript", "typescriptreact"]）
    pub language_names: Vec<String>,
    /// 关联的文件扩展名（如 [".ts", ".tsx"]）
    pub file_extensions: Vec<String>,
    /// Server 启动命令
    pub server_command: String,
    /// Server 启动参数
    pub server_args: Vec<String>,
    /// 初始化选项（可选，JSON 格式）
    pub initialization_options: Option<Value>,
    /// 需要的能力列表
    pub capabilities_required: Vec<String>,
}

/// 语言注册来源（扩展域通用契约，原 tool::lsp::registry 下沉）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageSource {
    /// 内置语言
    Builtin,
    /// 扩展注册的语言
    Extension { owner: String },
}

impl LanguageSource {
    /// 是否为内置来源。
    pub fn is_builtin(&self) -> bool {
        matches!(self, Self::Builtin)
    }

    /// 来源 owner；内置来源返回 `None`。
    pub fn owner(&self) -> Option<&str> {
        match self {
            Self::Builtin => None,
            Self::Extension { owner } => Some(owner),
        }
    }

    /// 来源标签（诊断/审计用）。
    pub fn label(&self) -> &'static str {
        if self.is_builtin() {
            "builtin"
        } else {
            "extension"
        }
    }
}

// ============================================================================
// Provider 认证契约（原 ai/gateway/provider/profile.rs 下沉）
// ============================================================================

/// Provider 认证方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthScheme {
    /// `Authorization: Bearer <secret>`
    Bearer,
    /// `<header_name>: <secret>`，通常为 `x-api-key`。
    ApiKey,
    /// 由 Provider profile 指定 header，值直接使用 secret。
    Custom,
    /// 不注入认证头。
    None,
}

impl AuthScheme {
    /// 从 Extension manifest 中的 canonical scheme 创建。
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim() {
            "bearer" => Ok(Self::Bearer),
            "api-key" => Ok(Self::ApiKey),
            "custom" => Ok(Self::Custom),
            "none" => Ok(Self::None),
            other => anyhow::bail!("不支持的 Gateway auth scheme: {other}"),
        }
    }
}

/// Gateway 使用的受控 Provider 认证 profile。
///
/// 该结构只描述认证方式和 header 位置，不保存 secret。secret 仍然只通过
/// `secret_ref` 由 Auth port 在发送请求的最小范围内解析。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderAuthProfile {
    scheme: AuthScheme,
    header: Option<String>,
    requires_secret: bool,
}

impl ProviderAuthProfile {
    /// 从 Extension 声明创建 profile。Extension 不能声明可选的认证 secret：
    /// 只要 scheme 不是 `none`，请求阶段就必须存在有效 `secret_ref`。
    pub fn from_manifest(scheme: &str, header: &str) -> Result<Self> {
        let scheme = AuthScheme::parse(scheme)?;
        let header = match scheme {
            AuthScheme::None => {
                if !header.trim().is_empty() {
                    bail!("auth scheme 'none' 不允许声明 header");
                }
                None
            }
            AuthScheme::Bearer => {
                let header = validate_auth_header_name(header)?;
                if !header.eq_ignore_ascii_case("authorization") {
                    bail!("bearer auth 必须使用 Authorization header");
                }
                Some(header)
            }
            AuthScheme::ApiKey | AuthScheme::Custom => Some(validate_auth_header_name(header)?),
        };

        Ok(Self {
            requires_secret: !matches!(scheme, AuthScheme::None),
            scheme,
            header,
        })
    }

    /// 创建不注入认证头的 profile。
    pub fn none() -> Self {
        Self {
            scheme: AuthScheme::None,
            header: None,
            requires_secret: false,
        }
    }

    /// 创建内建 Provider profile 对应的运行时认证 profile。
    pub fn from_builtin(
        scheme: AuthScheme,
        header: Option<&str>,
        requires_secret: bool,
    ) -> Result<Self> {
        let header = match scheme {
            AuthScheme::None => {
                if header.is_some_and(|value| !value.trim().is_empty()) {
                    bail!("auth scheme 'none' 不允许声明 header");
                }
                None
            }
            AuthScheme::Bearer => {
                let header = validate_auth_header_name(header.unwrap_or("Authorization"))?;
                if !header.eq_ignore_ascii_case("authorization") {
                    bail!("bearer auth 必须使用 Authorization header");
                }
                Some(header)
            }
            AuthScheme::ApiKey | AuthScheme::Custom => {
                Some(validate_auth_header_name(header.ok_or_else(|| {
                    anyhow::anyhow!("api-key/custom auth 必须声明 header")
                })?)?)
            }
        };

        Ok(Self {
            scheme,
            header,
            requires_secret,
        })
    }

    pub fn scheme(&self) -> AuthScheme {
        self.scheme
    }

    pub fn header_name(&self) -> Option<&str> {
        self.header.as_deref()
    }

    pub fn requires_secret(&self) -> bool {
        self.requires_secret
    }

    /// 生成宿主认证头。不会把 secret 写入模板、配置或日志。
    pub fn auth_headers(&self, secret: Option<&SecretValue>) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/json"),
        );

        match (self.scheme, secret) {
            (AuthScheme::None, Some(_)) => {
                bail!("auth scheme 'none' 不允许绑定 secret_ref")
            }
            (AuthScheme::None, None) => {}
            (_, None) if self.requires_secret => bail!("Provider 未解析到 secret_ref"),
            (_, None) => {}
            (AuthScheme::Bearer, Some(secret)) => {
                let value = format!("Bearer {}", secret.as_str());
                validate_secret_header_value(&value)?;
                headers.insert(
                    HeaderName::from_static("authorization"),
                    HeaderValue::from_str(&value).map_err(|_| {
                        anyhow::anyhow!("认证 secret 不能作为安全的 HTTP header 注入")
                    })?,
                );
            }
            (AuthScheme::ApiKey | AuthScheme::Custom, Some(secret)) => {
                let header = self
                    .header
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("认证 profile 缺少 header"))?;
                validate_secret_header_value(secret.as_str())?;
                let name = HeaderName::from_bytes(header.as_bytes())
                    .map_err(|_| anyhow::anyhow!("认证 header 名称无效"))?;
                let value = HeaderValue::from_str(secret.as_str())
                    .map_err(|_| anyhow::anyhow!("认证 secret 不能作为安全的 HTTP header 注入"))?;
                headers.insert(name, value);
            }
        }

        Ok(headers)
    }
}

fn validate_auth_header_name(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 128 || value.chars().any(char::is_whitespace) {
        bail!("认证 header 名称无效");
    }
    let name = HeaderName::from_bytes(value.as_bytes())
        .map_err(|_| anyhow::anyhow!("认证 header 名称无效"))?;
    let normalized = name.as_str();
    if matches!(
        normalized,
        "host" | "content-length" | "connection" | "transfer-encoding" | "cookie"
    ) {
        bail!("认证 header 不允许使用受保护的 header: {value}");
    }
    Ok(value.to_string())
}

fn validate_secret_header_value(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 8192 || value.chars().any(char::is_control) {
        bail!("认证 secret 不能作为安全的 HTTP header 注入");
    }
    reqwest::header::HeaderValue::from_str(value)
        .map_err(|_| anyhow::anyhow!("认证 secret 不能作为安全的 HTTP header 注入"))?;
    Ok(())
}

/// 编辑器扩展声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditorExtensionRegistration {
    /// 扩展 ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 描述
    pub description: String,
    /// ES Module 路径
    pub module: String,
    /// 激活条件
    pub activation_events: Vec<String>,
}

// ============================================================================
// Tray / Notification
// ============================================================================

/// 托盘菜单项声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrayItemRegistration {
    /// 菜单项 ID
    pub id: String,
    /// 显示文本
    pub label: String,
    /// 图标
    pub icon: Option<String>,
    /// 关联的命令 ID
    pub command: String,
    /// 插入位置
    pub position: TrayPosition,
    /// 条件表达式
    pub when: Option<String>,
}

/// 托盘菜单位置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum TrayPosition {
    Top,
    Middle,
    Bottom,
}

/// 通知渠道声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotificationChannelRegistration {
    /// 渠道 ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 描述
    pub description: String,
    /// 配置 JSON Schema
    pub config_schema: Value,
    /// 实现模块路径
    pub module: String,
}

// ============================================================================
// Trigger 声明
// ============================================================================

/// Chat 输入框触发器声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TriggerRegistration {
    /// 触发前缀（如 "/pr"）
    pub prefix: String,
    /// 显示名称
    pub label: String,
    /// 描述
    pub description: String,
    /// 图标
    pub icon: Option<String>,
    /// 搜索框占位文本
    pub placeholder: Option<String>,
    /// 搜索函数的 ES Module 路径
    pub search_module: String,
    /// 选中后行为的 ES Module 路径
    pub select_module: String,
    /// 作用范围
    pub scope: TriggerScope,
}

/// 触发器作用范围
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum TriggerScope {
    Input,
    Global,
}

/// 触发器搜索候选项
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TriggerCandidate {
    /// 候选项唯一 ID
    pub id: String,
    /// 主显示文本
    pub label: String,
    /// 副文本
    pub description: Option<String>,
    /// 图标
    pub icon: Option<String>,
    /// 附加数据
    pub metadata: Option<Value>,
}

/// 触发器选中后的动作
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum TriggerAction {
    /// 注入结构化引用标签
    InjectRef {
        ref_type: String,
        ref_id: String,
        label: String,
    },
    /// 注入纯文本
    InjectText { text: String },
    /// 执行已注册的命令
    RunCommand {
        command_id: String,
        args: Option<Value>,
    },
    /// 打开扩展视图
    OpenView {
        view_id: String,
        params: Option<Value>,
    },
    /// 切换内嵌组件显隐
    ToggleInline { extension_id: String },
    /// 更新状态栏项目
    UpdateStatusBar {
        item_id: String,
        label: Option<String>,
        icon: Option<String>,
    },
}

// ============================================================================
// Style / Layout / Behavior 声明
// ============================================================================

/// 样式声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StyleRegistration {
    /// 样式 ID
    pub id: String,
    /// ES Module 路径
    pub module: String,
    /// 作用域
    pub scope: StyleScope,
    /// 自定义 CSS 变量
    pub variables: Option<HashMap<String, String>>,
}

/// 样式作用域
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum StyleScope {
    Extension,
    View,
    Global,
}

/// 布局覆盖声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayoutOverrideRegistration {
    /// 目标组件 ID
    pub target: String,
    /// 定位方式
    pub position: Option<PositionValue>,
    /// 偏移量
    pub offset: Option<OffsetValue>,
    /// 尺寸
    pub size: Option<SizeValue>,
    /// 层级
    pub z_index: Option<i32>,
    /// 过渡动画
    pub transition: Option<String>,
}

/// 定位值
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PositionValue {
    #[serde(rename = "type")]
    pub type_: String,
    pub top: Option<String>,
    pub right: Option<String>,
    pub bottom: Option<String>,
    pub left: Option<String>,
}

/// 偏移值
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OffsetValue {
    pub x: Option<String>,
    pub y: Option<String>,
}

/// 尺寸值
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SizeValue {
    pub width: Option<String>,
    pub height: Option<String>,
    pub min_width: Option<String>,
    pub max_width: Option<String>,
    pub min_height: Option<String>,
    pub max_height: Option<String>,
}

/// 行为声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehaviorRegistration {
    /// 行为 ID
    pub id: String,
    /// 触发条件
    pub trigger: BehaviorTrigger,
    /// 触发后的动作
    pub action: BehaviorAction,
    /// 目标组件 ID
    pub target: Option<String>,
}

/// 行为触发条件
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum BehaviorTrigger {
    Hover {
        delay_ms: Option<u64>,
        leave_delay_ms: Option<u64>,
    },
    Focus {
        target_selector: String,
    },
    Click {
        target_selector: String,
        button: Option<String>,
    },
    Shortcut {
        key: String,
    },
    Resize {
        threshold: Option<f32>,
    },
}

/// 行为动作
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum BehaviorAction {
    ShowPanel {
        view_id: String,
        position: Option<String>,
    },
    ShowTooltip {
        content_module: String,
        position: Option<String>,
    },
    ToggleComponent {
        target_id: String,
    },
    EmitEvent {
        event_name: String,
        payload: Option<Value>,
    },
    RunCommand {
        command_id: String,
    },
}

// ============================================================================
// Hook 声明
// ============================================================================

/// 钩子声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HookRegistration {
    /// 钩子 ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 执行阶段
    pub phase: HookPhase,
    /// 执行优先级
    pub priority: Option<u32>,
    /// ES Module 路径
    pub module: String,
    /// 条件表达式
    pub when: Option<String>,
    /// 宿主执行动作
    #[serde(default)]
    pub action: HookAction,
}

/// 钩子执行阶段
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum HookPhase {
    /// 新会话或会话恢复进入 Agent 运行上下文前。
    SessionStart,
    /// 工具调用授权与执行前，可观察或请求改写工具输入。
    PreToolUse,
    /// 工具调用完成并写回模型前，可观察或请求改写工具结果。
    PostToolUse,
    /// 上下文压缩前，可观察压缩候选并提供保留建议。
    PreCompact,
}

/// Hook 的宿主受控动作。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum HookAction {
    /// 继续执行，不修改工具调用。
    Continue,
    /// 拒绝当前工具调用，并把 reason 作为 tool result 回注模型。
    Deny { reason: String },
}

impl Default for HookAction {
    fn default() -> Self {
        Self::Continue
    }
}

// ============================================================================
// Context / Search / FileWatcher 声明
// ============================================================================

/// 上下文数据源声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextProviderRegistration {
    /// 数据源 ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 描述
    pub description: String,
    /// ES Module 路径
    pub module: String,
    /// 触发模式（正则）
    pub trigger_pattern: Option<String>,
    /// 注入位置
    pub inject_position: Option<InjectPosition>,
    /// 注入优先级
    pub priority: Option<u32>,
    /// 最大注入 Token 数
    pub max_tokens: Option<usize>,
}

/// 注入位置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum InjectPosition {
    BeforeHistory,
    AfterHistory,
    AfterUserMessage,
}

/// 全局搜索提供者声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchProviderRegistration {
    /// 搜索源 ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 图标
    pub icon: Option<String>,
    /// ES Module 路径
    pub module: String,
    /// 搜索范围标签
    pub scope_tags: Vec<String>,
    /// 结果排序优先级
    pub priority: Option<u32>,
}

/// 文件监听器声明
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileWatcherRegistration {
    /// 监听器 ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 文件匹配模式
    pub patterns: Vec<String>,
    /// 监听的事件类型
    pub events: Vec<FileWatchEvent>,
    /// ES Module 路径
    pub module: String,
    /// 防抖延迟
    pub debounce_ms: Option<u64>,
}

/// 文件监听事件
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum FileWatchEvent {
    Created,
    Modified,
    Deleted,
    Renamed,
}

// ============================================================================
// ExtensionState - 扩展运行时状态
// ============================================================================

/// 扩展运行时状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionState {
    /// 扩展 ID
    pub id: String,
    /// 当前状态
    pub status: ExtensionStatus,
    /// 扩展清单
    pub manifest: ExtensionManifest,
    /// 安装路径
    pub install_path: PathBuf,
    /// 安装时间
    pub installed_at: DateTime<Utc>,
    /// 启用时间
    pub enabled_at: Option<DateTime<Utc>>,
    /// 错误信息
    pub error: Option<String>,
}

/// 扩展状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionStatus {
    Installed,
    Loading,
    Enabled,
    Disabling,
    Disabled,
    Unloading,
    Error,
}

impl std::fmt::Display for ExtensionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtensionStatus::Installed => write!(f, "installed"),
            ExtensionStatus::Loading => write!(f, "loading"),
            ExtensionStatus::Enabled => write!(f, "enabled"),
            ExtensionStatus::Disabling => write!(f, "disabling"),
            ExtensionStatus::Disabled => write!(f, "disabled"),
            ExtensionStatus::Unloading => write!(f, "unloading"),
            ExtensionStatus::Error => write!(f, "error"),
        }
    }
}

impl StatusClassify for ExtensionStatus {
    fn status_presentation(&self) -> StatusPresentation {
        match self {
            Self::Loading | Self::Disabling | Self::Unloading => StatusPresentation::active(),
            Self::Enabled => StatusPresentation::succeeded(),
            Self::Installed | Self::Disabled => StatusPresentation::inactive(),
            Self::Error => StatusPresentation::failed(),
        }
    }
}

/// 将 ExtensionStatus 映射到 kernel LifecycleAction（仅限 kernel 管理的状态转换）。
///
/// 返回 `Some(action)` 表示需要调用 kernel lifecycle；`None` 表示仅更新元数据。
pub(crate) fn status_to_kernel_lifecycle(
    status: &ExtensionStatus,
) -> Option<crate::kernel::LifecycleAction> {
    use crate::kernel::LifecycleAction;
    match status {
        ExtensionStatus::Enabled => Some(LifecycleAction::Enable),
        ExtensionStatus::Disabled => Some(LifecycleAction::Disable),
        _ => None,
    }
}

// ============================================================================
// ExtensionCapability - kernel::Capability 实现
// ============================================================================

/// 扩展能力注册项，将 ExtensionState 包装为 kernel InMemoryRegistry 可管理的能力实体。
///
/// metadata 仅存储 ExtensionState。Kernel lifecycle 由 RegistryEntry 独立持有。
#[derive(Debug, Clone)]
pub struct ExtensionCapability {
    id: String,
    _manifest_version: String,
    metadata_json: Value,
}

impl ExtensionCapability {
    pub fn new(state: ExtensionState) -> Self {
        let id = state.id.clone();
        let manifest_version = state.manifest.version.clone();
        let metadata_json = serde_json::to_value(&state).unwrap_or(serde_json::json!({}));
        Self {
            id,
            _manifest_version: manifest_version,
            metadata_json,
        }
    }

    /// 反序列化 metadata 为 ExtensionState。
    pub fn to_extension_state(&self) -> Option<ExtensionState> {
        serde_json::from_value(self.metadata_json.clone()).ok()
    }
}

impl crate::kernel::Capability for ExtensionCapability {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> &str {
        "extension"
    }

    fn version(&self) -> SchemaVersion {
        SchemaVersion::default()
    }

    fn metadata(&self) -> &Value {
        &self.metadata_json
    }
}

// ============================================================================
// ExtensionPermissionConstraint - kernel::Constraint 实现
// ============================================================================

/// 扩展权限约束，将 ExtensionPermissions 映射为 kernel PolicyEngine 可评估的硬性约束。
///
/// 当 action 匹配 filesystem/terminal/network 时，根据声明的白名单决定 Allow/Deny。
/// 未声明任何权限的扩展无法执行对应操作。
#[derive(Debug)]
pub struct ExtensionPermissionConstraint {
    extension_id: String,
    permissions: ExtensionPermissions,
    constraint_id: String,
}

impl ExtensionPermissionConstraint {
    pub fn new(extension_id: impl Into<String>, permissions: ExtensionPermissions) -> Self {
        let extension_id = extension_id.into();
        Self {
            constraint_id: Self::constraint_id_for(&extension_id),
            extension_id,
            permissions,
        }
    }

    pub fn extension_id(&self) -> &str {
        &self.extension_id
    }

    pub fn permissions(&self) -> &ExtensionPermissions {
        &self.permissions
    }

    pub fn constraint_id(&self) -> &str {
        &self.constraint_id
    }

    pub fn constraint_id_for(extension_id: &str) -> String {
        format!("extension.permission:{extension_id}")
    }
}

impl Constraint for ExtensionPermissionConstraint {
    fn id(&self) -> &str {
        self.constraint_id()
    }

    fn priority(&self) -> i32 {
        50
    }

    fn evaluate(&self, input: &PolicyInput) -> Option<PolicyDecision> {
        // 仅处理 extension: 域的 action；其他域的 action 不匹配，返回 None 跳过
        if !input.scope.starts_with("extension") {
            return None;
        }

        let subject_extension = input
            .subject
            .strip_prefix("extension:")
            .unwrap_or(&input.subject);
        if subject_extension != self.extension_id {
            return None;
        }

        match input.action.as_str() {
            action if action.starts_with("filesystem:") => {
                let target = action.strip_prefix("filesystem:").unwrap_or("");
                if self
                    .permissions
                    .filesystem
                    .iter()
                    .any(|p| target.starts_with(p.as_str()))
                {
                    Some(PolicyDecision::Allow {
                        reason: "extension filesystem permission granted".into(),
                    })
                } else {
                    Some(PolicyDecision::Deny {
                        reason: format!(
                            "extension '{}' not permitted for filesystem:{}",
                            self.extension_id, target
                        ),
                    })
                }
            }
            action if action.starts_with("terminal:") => {
                let command = action.strip_prefix("terminal:").unwrap_or("");
                if self.permissions.terminal.iter().any(|p| p == command) {
                    Some(PolicyDecision::Allow {
                        reason: "extension terminal permission granted".into(),
                    })
                } else {
                    Some(PolicyDecision::Deny {
                        reason: format!(
                            "extension '{}' not permitted for terminal:{}",
                            self.extension_id, command
                        ),
                    })
                }
            }
            action if action.starts_with("network:") => {
                let url = action.strip_prefix("network:").unwrap_or("");
                if self
                    .permissions
                    .network
                    .iter()
                    .any(|p| url.starts_with(p.as_str()))
                {
                    Some(PolicyDecision::Allow {
                        reason: "extension network permission granted".into(),
                    })
                } else {
                    Some(PolicyDecision::Deny {
                        reason: format!(
                            "extension '{}' not permitted for network:{}",
                            self.extension_id, url
                        ),
                    })
                }
            }
            _ => None, // 非文件/终端/网络 action，不干预
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extension_manifest_serde_roundtrip() {
        let manifest = ExtensionManifest {
            id: "com.test.extension".into(),
            name: "Test Extension".into(),
            version: "1.0.0".into(),
            description: "A test extension".into(),
            author: "Tester".into(),
            permissions: ExtensionPermissions::default(),
            contributes: ExtensionContributes::default(),
        };

        let json_str = serde_json::to_string(&manifest).unwrap();
        let parsed: ExtensionManifest = serde_json::from_str(&json_str).unwrap();
        assert_eq!(manifest, parsed);
    }

    #[test]
    fn test_extension_status_display() {
        assert_eq!(ExtensionStatus::Installed.to_string(), "installed");
        assert_eq!(ExtensionStatus::Loading.to_string(), "loading");
        assert_eq!(ExtensionStatus::Enabled.to_string(), "enabled");
        assert_eq!(ExtensionStatus::Disabling.to_string(), "disabling");
        assert_eq!(ExtensionStatus::Disabled.to_string(), "disabled");
        assert_eq!(ExtensionStatus::Unloading.to_string(), "unloading");
        assert_eq!(ExtensionStatus::Error.to_string(), "error");
    }

    #[test]
    fn test_extension_status_serde() {
        let status = ExtensionStatus::Enabled;
        let json_str = serde_json::to_string(&status).unwrap();
        assert_eq!(json_str, "\"enabled\"");
        let parsed: ExtensionStatus = serde_json::from_str("\"installed\"").unwrap();
        assert_eq!(parsed, ExtensionStatus::Installed);
    }

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_memory_mb, 512);
        assert_eq!(limits.max_cpu_percent, 50.0);
        assert_eq!(limits.timeout_ms, 30_000);
    }

    #[test]
    fn test_builtin_action_serde() {
        let action = BuiltinAction::OpenView {
            view_id: "test.view".into(),
        };
        let json_str = serde_json::to_string(&action).unwrap();
        let parsed: BuiltinAction = serde_json::from_str(&json_str).unwrap();
        assert_eq!(action, parsed);
    }

    #[test]
    fn test_hook_phase_serde() {
        let phase = HookPhase::PreToolUse;
        let json_str = serde_json::to_string(&phase).unwrap();
        assert_eq!(json_str, "\"PreToolUse\"");
        let parsed: HookPhase = serde_json::from_str("\"PostToolUse\"").unwrap();
        assert_eq!(parsed, HookPhase::PostToolUse);
    }

    #[test]
    fn test_hook_action_serde() {
        let action = HookAction::Deny {
            reason: "blocked".into(),
        };
        let json_str = serde_json::to_string(&action).unwrap();
        let parsed: HookAction = serde_json::from_str(&json_str).unwrap();
        assert_eq!(action, parsed);

        assert_eq!(HookAction::default(), HookAction::Continue);
    }

    #[test]
    fn test_trigger_action_serde() {
        let action = TriggerAction::InjectRef {
            ref_type: "pr".into(),
            ref_id: "42".into(),
            label: "PR #42".into(),
        };
        let json_str = serde_json::to_string(&action).unwrap();
        let parsed: TriggerAction = serde_json::from_str(&json_str).unwrap();
        assert_eq!(action, parsed);
    }

    #[test]
    fn test_behavior_trigger_serde() {
        let trigger = BehaviorTrigger::Hover {
            delay_ms: Some(500),
            leave_delay_ms: Some(300),
        };
        let json_str = serde_json::to_string(&trigger).unwrap();
        let parsed: BehaviorTrigger = serde_json::from_str(&json_str).unwrap();
        assert_eq!(trigger, parsed);
    }

    #[test]
    fn test_behavior_action_serde() {
        let action = BehaviorAction::ShowPanel {
            view_id: "test.panel".into(),
            position: Some("near-cursor".into()),
        };
        let json_str = serde_json::to_string(&action).unwrap();
        let parsed: BehaviorAction = serde_json::from_str(&json_str).unwrap();
        assert_eq!(action, parsed);
    }

    #[test]
    fn test_manifest_from_json_value() {
        let json = json!({
            "id": "com.example.test",
            "name": "Test",
            "version": "0.1.0",
            "description": "desc",
            "author": "auth",
            "permissions": {
                "filesystem": [],
                "terminal": [],
                "network": [],
                "ipc": [],
                "events": [],
                "resources": {
                    "max_memory_mb": 256,
                    "max_cpu_percent": 30.0,
                    "timeout_ms": 10000
                }
            },
            "contributes": {}
        });

        let manifest: ExtensionManifest = serde_json::from_value(json).unwrap();
        assert_eq!(manifest.id, "com.example.test");
        assert_eq!(manifest.permissions.resources.max_memory_mb, 256);
    }

    #[test]
    fn test_extension_state_serde() {
        let state = ExtensionState {
            id: "test-extension".into(),
            status: ExtensionStatus::Enabled,
            manifest: ExtensionManifest {
                id: "test-extension".into(),
                name: "Test".into(),
                version: "1.0.0".into(),
                description: "desc".into(),
                author: "auth".into(),
                permissions: ExtensionPermissions::default(),
                contributes: ExtensionContributes::default(),
            },
            install_path: PathBuf::from("/extensions/test-extension"),
            installed_at: Utc::now(),
            enabled_at: Some(Utc::now()),
            error: None,
        };

        let json_str = serde_json::to_string(&state).unwrap();
        let parsed: ExtensionState = serde_json::from_str(&json_str).unwrap();
        assert_eq!(state.id, parsed.id);
        assert_eq!(state.status, parsed.status);
        assert!(parsed.error.is_none());
    }

    #[test]
    fn test_contributes_with_views_and_menus() {
        let json = json!({
            "id": "com.example.test",
            "name": "Test",
            "version": "1.0.0",
            "description": "desc",
            "author": "auth",
            "permissions": {
                "filesystem": [],
                "terminal": [],
                "network": [],
                "ipc": [],
                "events": [],
                "resources": { "max_memory_mb": 512, "max_cpu_percent": 50.0, "timeout_ms": 30000 }
            },
            "contributes": {
                "views": [{
                    "id": "test.dashboard",
                    "name": "Dashboard",
                    "placement": "rightWorkspace",
                    "renderer": "host:panel",
                    "config": { "summary": true },
                    "activation_events": []
                }],
                "menus": [{
                    "id": "test.menu",
                    "label": "Open Dashboard",
                    "target": "Tools",
                    "command": "test.openDashboard"
                }],
                "commands": [{
                    "id": "test.openDashboard",
                    "label": "Open Dashboard",
                    "action": { "type": "OpenView", "view_id": "test.dashboard" }
                }]
            }
        });

        let manifest: ExtensionManifest = serde_json::from_value(json).unwrap();
        assert!(manifest.contributes.views.is_some());
        assert_eq!(manifest.contributes.views.as_ref().unwrap().len(), 1);
        assert_eq!(
            manifest.contributes.views.as_ref().unwrap()[0].renderer,
            "host:panel"
        );
        assert_eq!(
            manifest.contributes.views.as_ref().unwrap()[0]
                .placement
                .as_deref(),
            Some("rightWorkspace")
        );
        assert!(manifest.contributes.menus.is_some());
        assert!(manifest.contributes.commands.is_some());
    }

    #[test]
    fn test_event_subscriptions_manifest_contract() {
        let json = json!({
            "id": "com.example.events",
            "name": "Events",
            "version": "1.0.0",
            "description": "event extension",
            "author": "auth",
            "permissions": {
                "filesystem": [],
                "terminal": [],
                "network": [],
                "ipc": [],
                "events": ["session.completed"],
                "resources": { "max_memory_mb": 512, "max_cpu_percent": 50.0, "timeout_ms": 30000 }
            },
            "contributes": {
                "eventSubscriptions": [{
                    "id": "session-completed",
                    "topic": "session.completed",
                    "scopeKey": "session:active",
                    "handler": {
                        "module": "./runtime/events",
                        "export": "onSessionCompleted"
                    }
                }]
            }
        });

        let manifest: ExtensionManifest = serde_json::from_value(json).unwrap();
        let subscriptions = manifest.contributes.event_subscriptions.unwrap();
        assert_eq!(subscriptions.len(), 1);
        assert_eq!(subscriptions[0].id, "session-completed");
        assert_eq!(subscriptions[0].topic, "session.completed");
        assert_eq!(
            subscriptions[0].scope_key.as_deref(),
            Some("session:active")
        );
        assert_eq!(subscriptions[0].handler.module, "./runtime/events");
        assert_eq!(subscriptions[0].handler.export, "onSessionCompleted");
    }

    #[test]
    fn test_event_subscription_manifest_rejects_unknown_fields() {
        let json = json!({
            "id": "com.example.events",
            "name": "Events",
            "version": "1.0.0",
            "description": "event extension",
            "author": "auth",
            "permissions": {
                "filesystem": [],
                "terminal": [],
                "network": [],
                "ipc": [],
                "events": [],
                "resources": { "max_memory_mb": 512, "max_cpu_percent": 50.0, "timeout_ms": 30000 }
            },
            "contributes": {
                "eventSubscriptions": [{
                    "id": "session-completed",
                    "topic": "session.completed",
                    "handler": {
                        "module": "./runtime/events",
                        "export": "onSessionCompleted",
                        "unexpected": true
                    }
                }]
            }
        });

        assert!(serde_json::from_value::<ExtensionManifest>(json).is_err());
    }

    #[test]
    fn test_extension_event_dto_serialization_contract() {
        let event = ExtensionEventDto {
            id: "event-1".into(),
            topic: "session.completed".into(),
            version: SchemaVersion::new(1, 2),
            scope_key: "session:active".into(),
            source: "session".into(),
            payload: Some(json!({ "sessionId": "session-1" })),
            created_at: Utc::now(),
        };

        let value = serde_json::to_value(&event).unwrap();
        assert!(value.get("scopeKey").is_some());
        assert!(value.get("created_at").is_some());
        assert!(value.get("scope_key").is_none());
        assert_eq!(value["version"], json!({ "major": 1, "minor": 2 }));
    }

    #[test]
    fn test_backend_service_registration_serde() {
        let service = BackendServiceRegistration {
            id: "language-server".into(),
            entry: "ExtensionBackend/language-server.exe".into(),
            transport: BackendTransport::Stdio,
            protocol: BackendProtocol::JsonRpc,
            args: vec!["--stdio".into()],
            env: [("LOG_LEVEL".to_string(), "debug".to_string())]
                .into_iter()
                .collect(),
            autostart: true,
        };

        let value = serde_json::to_value(&service).unwrap();
        assert_eq!(value["id"], "language-server");
        assert_eq!(value["entry"], "ExtensionBackend/language-server.exe");
        assert_eq!(value["transport"], "stdio");
        assert_eq!(value["protocol"], "jsonrpc");
        assert_eq!(value["args"][0], "--stdio");
        assert_eq!(value["env"]["LOG_LEVEL"], "debug");
        assert_eq!(value["autostart"], true);

        let parsed: BackendServiceRegistration = serde_json::from_value(value).unwrap();
        assert_eq!(parsed, service);
    }

    #[test]
    fn test_backend_service_manifest_uses_camel_case_key() {
        let json = json!({
            "id": "com.example.backend",
            "name": "Backend",
            "version": "1.0.0",
            "description": "backend extension",
            "author": "auth",
            "permissions": {
                "filesystem": [],
                "terminal": [],
                "network": [],
                "ipc": [],
                "events": [],
                "resources": { "max_memory_mb": 512, "max_cpu_percent": 50.0, "timeout_ms": 30000 }
            },
            "contributes": {
                "backendServices": [{
                    "id": "search",
                    "entry": "ExtensionBackend/search",
                    "transport": "sse",
                    "protocol": "jsonrpc",
                    "args": ["--port", "8000"],
                    "env": { "PORT": "8000" },
                    "autostart": false
                }]
            }
        });

        let manifest: ExtensionManifest = serde_json::from_value(json).unwrap();
        let services = manifest.contributes.backend_services.unwrap();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].id, "search");
        assert_eq!(services[0].entry, "ExtensionBackend/search");
        assert_eq!(services[0].transport, BackendTransport::Sse);
        assert_eq!(services[0].protocol, BackendProtocol::JsonRpc);
        assert_eq!(services[0].args, vec!["--port".to_string(), "8000".to_string()]);
        assert_eq!(services[0].env["PORT"], "8000");
        assert!(!services[0].autostart);
    }

    #[test]
    fn test_backend_service_defaults_and_enum_lowercase() {
        // 未声明字段时回退到默认值。
        let json = json!({
            "id": "defaults",
            "entry": "ExtensionBackend/server"
        });
        let service: BackendServiceRegistration = serde_json::from_value(json).unwrap();
        assert_eq!(service.transport, BackendTransport::Stdio);
        assert_eq!(service.protocol, BackendProtocol::JsonRpc);
        assert!(service.args.is_empty());
        assert!(service.env.is_empty());
        assert!(!service.autostart);

        // 枚举 lowercase 序列化 / 反序列化。
        assert_eq!(serde_json::to_string(&BackendTransport::WebSocket).unwrap(), "\"websocket\"");
        assert_eq!(
            serde_json::from_str::<BackendTransport>("\"sse\"").unwrap(),
            BackendTransport::Sse
        );
        assert_eq!(serde_json::to_string(&BackendProtocol::JsonRpc).unwrap(), "\"jsonrpc\"");
        assert_eq!(BackendTransport::default(), BackendTransport::Stdio);
        assert_eq!(BackendProtocol::default(), BackendProtocol::JsonRpc);
    }

    #[test]
    fn test_component_registration_serde_roundtrip() {
        let component = ComponentRegistration {
            id: "app".into(),
            entry: "ExtensionUI/scripts/app.component.wasm".into(),
            kind: ComponentKind::Logic,
            run_on: vec!["activation".into(), "message".into()],
            capabilities: ComponentCapabilities {
                invoke: vec!["operation.execute".into(), "context.getSession".into()],
                storage: vec!["global".into()],
                network: Some(json!({ "type": "allowlist", "hosts": [{ "host": "api.example.com" }] })),
                events: vec!["session.completed".into()],
            },
            autostart: false,
        };

        let value = serde_json::to_value(&component).unwrap();
        assert_eq!(value["id"], "app");
        assert_eq!(value["entry"], "ExtensionUI/scripts/app.component.wasm");
        assert_eq!(value["kind"], "logic");
        assert_eq!(value["runOn"], json!(["activation", "message"]));
        assert_eq!(value["capabilities"]["invoke"][0], "operation.execute");
        assert_eq!(value["autostart"], false);

        let parsed: ComponentRegistration = serde_json::from_value(value).unwrap();
        assert_eq!(parsed, component);
    }

    #[test]
    fn test_component_manifest_uses_camel_case_key() {
        let json = json!({
            "id": "com.example.components",
            "name": "Components",
            "version": "1.0.0",
            "description": "component extension",
            "author": "auth",
            "permissions": {
                "filesystem": [],
                "terminal": [],
                "network": [],
                "ipc": [],
                "events": [],
                "resources": { "max_memory_mb": 512, "max_cpu_percent": 50.0, "timeout_ms": 30000 }
            },
            "contributes": {
                "components": [{
                    "id": "worker",
                    "entry": "ExtensionBackend/logic/worker.component.wasm",
                    "kind": "logic",
                    "runOn": ["activation"],
                    "capabilities": {
                        "invoke": ["operation.execute"],
                        "storage": ["global"],
                        "network": { "type": "allowlist", "hosts": [{ "host": "api.example.com" }] },
                        "events": ["session.completed"]
                    },
                    "autostart": true
                }]
            }
        });

        let manifest: ExtensionManifest = serde_json::from_value(json).unwrap();
        let components = manifest.contributes.components.unwrap();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].id, "worker");
        assert_eq!(
            components[0].entry,
            "ExtensionBackend/logic/worker.component.wasm"
        );
        assert_eq!(components[0].kind, ComponentKind::Logic);
        assert_eq!(components[0].run_on, vec!["activation".to_string()]);
        assert_eq!(
            components[0].capabilities.invoke,
            vec!["operation.execute".to_string()]
        );
        assert_eq!(
            components[0].capabilities.storage,
            vec!["global".to_string()]
        );
        assert!(components[0].capabilities.network.is_some());
        assert_eq!(
            components[0].capabilities.events,
            vec!["session.completed".to_string()]
        );
        assert!(components[0].autostart);
    }

    #[test]
    fn test_component_defaults_and_kind_lowercase() {
        // 未声明字段时回退到默认值。
        let json = json!({
            "id": "defaults",
            "entry": "ExtensionUI/scripts/app.component.wasm",
            "capabilities": {}
        });
        let component: ComponentRegistration = serde_json::from_value(json).unwrap();
        assert_eq!(component.kind, ComponentKind::Logic);
        assert!(component.run_on.is_empty());
        assert!(!component.autostart);
        assert!(component.capabilities.invoke.is_empty());
        assert!(component.capabilities.storage.is_empty());
        assert!(component.capabilities.network.is_none());
        assert!(component.capabilities.events.is_empty());

        // 枚举 lowercase 序列化 / 反序列化。
        assert_eq!(
            serde_json::to_string(&ComponentKind::Logic).unwrap(),
            "\"logic\""
        );
        assert_eq!(
            serde_json::to_string(&ComponentKind::Native).unwrap(),
            "\"native\""
        );
        assert_eq!(
            serde_json::from_str::<ComponentKind>("\"logic\"").unwrap(),
            ComponentKind::Logic
        );
        assert_eq!(
            serde_json::from_str::<ComponentKind>("\"native\"").unwrap(),
            ComponentKind::Native
        );
        assert_eq!(ComponentKind::default(), ComponentKind::Logic);
    }

    #[test]
    fn test_component_manifest_rejects_unknown_fields() {
        let json = json!({
            "id": "com.example.components",
            "name": "Components",
            "version": "1.0.0",
            "description": "component extension",
            "author": "auth",
            "permissions": {
                "filesystem": [],
                "terminal": [],
                "network": [],
                "ipc": [],
                "events": [],
                "resources": { "max_memory_mb": 512, "max_cpu_percent": 50.0, "timeout_ms": 30000 }
            },
            "contributes": {
                "components": [{
                    "id": "worker",
                    "entry": "ExtensionBackend/logic/worker.component.wasm",
                    "capabilities": {},
                    "unexpected": true
                }]
            }
        });

        assert!(serde_json::from_value::<ExtensionManifest>(json).is_err());
    }
}
