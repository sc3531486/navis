//! 业务装配层
//!
//! 领域无关容器（设计 35 号文档 v5）：桌面客户端容器是"白板"，不绑定任何业务领域。
//! 容器壳只装配平台原语、受控操作机制与扩展生命周期；业务 State 的创建与装配统一
//! 收口到本层。每个业务领域实现 [`BusinessAssembly`] 并注册进内建业务装配清单，
//! 容器壳通过 `assemble_registered` 依次装配已启用业务，拿到领域无关的装配结果。
//!
//! 当前内建业务为 Navis AI IDE（本次在框架上落地的 Agent IDE）。柜面系统、双录系统
//! 等未来业务按同一 [`BusinessAssembly`] 契约注册，禁止直接改动容器壳。

use std::sync::{Arc, Mutex};

use tauri::Manager;

use crate::domains::agent_core::agent; use crate::domains::ai_platform::gateway;
use crate::extension::skills;
use crate::domains::project::catalog; use crate::domains::session::session;
use crate::domains::session::session::SessionStore;
use crate::domains::agent_core::tool_runtime::pipeline::AgentDefaultAllowConstraint;
use crate::domains::editor::clipboard::policy::register_clipboard_constraints;
use crate::domains::terminal::terminal::TerminalManager;
use crate::extension::lifecycle::{GatewayCapabilityPort, LspCapabilityPort, McpCapabilityPort};
use crate::domains::ai_platform::mcp::protocol::{MCPServerConfig, ToolDefinition, ToolDefinitionOverride};
use crate::domains::editor::backend; use crate::domains::ai_platform::lsp; use crate::domains::ai_platform::mcp;
use crate::ui;

impl GatewayCapabilityPort for gateway::Gateway {
    fn upsert_provider(&self, config: gateway::ProviderConfig) -> anyhow::Result<()> {
        gateway::Gateway::upsert_provider(self, config)
    }

    fn set_provider_capabilities(
        &self,
        provider_id: &str,
        capabilities: gateway::protocol::CapabilitySet,
    ) -> anyhow::Result<()> {
        gateway::Gateway::set_provider_capabilities(self, provider_id, capabilities)
    }

    fn remove_provider_capabilities(&self, provider_id: &str) -> anyhow::Result<()> {
        gateway::Gateway::remove_provider_capabilities(self, provider_id);
        Ok(())
    }

    fn remove_provider(&self, id: &str) -> anyhow::Result<()> {
        gateway::Gateway::remove_provider(self, id)
    }

    fn acquire_protocol(&self, owner: &str, protocol: &gateway::ApiProtocol) -> anyhow::Result<()> {
        gateway::Gateway::acquire_protocol_adapter(self, owner, protocol)
    }

    fn register_custom_protocol(
        &self,
        owner: &str,
        config: gateway::CustomProtocolConfig,
    ) -> anyhow::Result<()> {
        gateway::Gateway::register_custom_protocol(self, owner, config)
    }

    fn release_protocol(&self, owner: &str, protocol: &gateway::ApiProtocol) -> anyhow::Result<()> {
        gateway::Gateway::unregister_protocol_adapter(self, owner, protocol)
    }
}

impl McpCapabilityPort for mcp::MCP {
    fn add_server(&self, config: MCPServerConfig) -> anyhow::Result<()> {
        mcp::MCP::add_server(self, config)
    }

    fn start_server(&self, id: &str) -> anyhow::Result<()> {
        mcp::MCP::start_server(self, id)
    }

    fn remove_server(&self, id: &str) -> anyhow::Result<()> {
        mcp::MCP::remove_server(self, id)
    }

    fn register_tool(&self, tool: ToolDefinition) -> anyhow::Result<()> {
        mcp::MCP::register_tool(self, tool)
    }

    fn unregister_server_tools(&self, server_id: &str) -> anyhow::Result<usize> {
        mcp::MCP::unregister_server_tools(self, server_id)
    }

    fn apply_tool_override(
        &self,
        owner: &str,
        server_id: &str,
        tool_name: &str,
        override_: ToolDefinitionOverride,
    ) -> anyhow::Result<()> {
        mcp::MCP::apply_tool_override(self, owner, server_id, tool_name, override_)
    }

    fn remove_tool_override(
        &self,
        owner: &str,
        server_id: &str,
        tool_name: &str,
    ) -> anyhow::Result<()> {
        mcp::MCP::remove_tool_override(self, owner, server_id, tool_name)
    }
}

impl LspCapabilityPort for lsp::LSPManager {
    fn register_language(
        &self,
        config: lsp::LSPServerConfig,
        source: lsp::LanguageSource,
    ) -> anyhow::Result<()> {
        self.registry()
            .register(config, source)
            .map_err(anyhow::Error::msg)
    }

    fn unregister_language(&self, language_id: &str, owner: &str) -> anyhow::Result<()> {
        self.registry()
            .unregister(language_id, owner)
            .map_err(anyhow::Error::msg)
    }
}
/// 业务装配产物：AI IDE 业务的全部 State 实例。
///
/// 每个字段均已在 `assemble_ai_ide` 内 `app.manage(...)`（Tauri State 必须 manage），
/// 命令签名按原类型不变。`StreamIndex` 属于容器流基础设施，留在容器壳装配。
#[allow(dead_code)]
pub(crate) struct BusinessContext {
    pub manager: Arc<session::SessionManager>,
    pub gateway: Arc<gateway::Gateway>,
    pub mcp: Arc<mcp::MCP>,
    pub lsp: Arc<lsp::LSPManager>,
    pub task_manager: Arc<Mutex<agent::TaskManager>>,
    pub composer_runtime: Arc<Mutex<session::composer_runtime::ComposerRuntime>>,
    pub project_manager: Arc<Mutex<catalog::ProjectManager>>,
    pub skills: Arc<Mutex<skills::Skills>>,
    pub terminal: Arc<TerminalManager>,
    pub backend_manager: Arc<backend::BackendProcessManager>,
    pub approval_store: Arc<ui::ToolApprovalStore>,
}

/// 业务装配上下文：由容器壳注入的平台原语集合。
///
/// 业务扩展只允许消费这些领域无关原语，不允许反向依赖容器壳内部实现。
pub(crate) struct BusinessAssemblyContext<'a> {
    pub app: &'a mut tauri::App,
    pub event_bus: Arc<dyn crate::kernel::EventBus>,
    pub storage: Arc<crate::app::infra::Storage>,
    pub sandbox: Arc<crate::security::sandbox::Sandbox>,
    pub policy_engine: Arc<crate::kernel::PolicyEngine>,
    pub auth: Arc<crate::security::auth::Auth>,
    pub config: Arc<Mutex<crate::foundation::config::Config>>,
    pub app_data_dir: &'a std::path::Path,
}

/// 内建业务扩展装配契约。
///
/// 一个业务扩展负责装配一组内聚的 Tauri State，并向容器提供生命周期接线所需的
/// 最小投影。未来柜面系统、双录系统等业务扩展都实现本 trait，容器壳不做任何
/// 领域特判。
pub(crate) trait BusinessAssembly: Send + Sync {
    /// 业务扩展唯一标识，形如 `navis.builtin.ai-ide`。
    fn id(&self) -> &'static str;

    /// 在容器壳提供的上下文中装配本业务扩展。
    fn assemble(
        &self,
        ctx: &mut BusinessAssemblyContext<'_>,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

/// 内建 AI IDE 业务扩展：装配会话、任务、项目、终端、MCP、LSP、Gateway、Skills。
struct AiIdeBusiness;

impl BusinessAssembly for AiIdeBusiness {
    fn id(&self) -> &'static str {
        "navis.builtin.ai-ide"
    }

    fn assemble(
        &self,
        ctx: &mut BusinessAssemblyContext<'_>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let context = assemble_ai_ide(ctx)?;
        // 将 AI IDE 业务上下文作为 Tauri State 托管，容器壳可在装配后读取并用其
        // 注入扩展生命周期能力端口。
        ctx.app.manage(context);
        Ok(())
    }
}

/// 返回当前容器启用的内建业务扩展清单。
///
/// 这是"万物皆扩展"在 Rust 容器侧的装配边界：新增业务领域只需在此追加一个
/// `Arc<dyn BusinessAssembly>`，容器壳不感知具体业务。
pub(crate) fn builtin_business_assemblies() -> Vec<Arc<dyn BusinessAssembly>> {
    vec![Arc::new(AiIdeBusiness)]
}

/// 装配全部已启用的内建业务扩展。
///
/// 当设置 `NAVIS_WHITEBOARD=1` 时跳过业务装配，容器以白板空壳启动（只具备平台
/// 原语与扩展生命周期，业务命令在未装配时不可用）。
#[allow(clippy::type_complexity)]
pub(crate) fn assemble_registered(
    app: &mut tauri::App,
    event_bus: Arc<dyn crate::kernel::EventBus>,
    storage: Arc<crate::app::infra::Storage>,
    sandbox: Arc<crate::security::sandbox::Sandbox>,
    policy_engine: Arc<crate::kernel::PolicyEngine>,
    auth: Arc<crate::security::auth::Auth>,
    config: Arc<Mutex<crate::foundation::config::Config>>,
    app_data_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(value) = std::env::var("NAVIS_WHITEBOARD") {
        if !value.is_empty() && value != "0" {
            tracing::warn!(
                "NAVIS_WHITEBOARD={}：跳过业务装配，容器将以白板空壳启动（业务命令不可用）",
                value
            );
            return Ok(());
        }
    }

    for assembly in builtin_business_assemblies() {
        tracing::info!(extension_id = assembly.id(), "Assembling builtin business extension");
        let mut ctx = BusinessAssemblyContext {
            app: &mut *app,
            event_bus: event_bus.clone(),
            storage: storage.clone(),
            sandbox: sandbox.clone(),
            policy_engine: policy_engine.clone(),
            auth: auth.clone(),
            config: config.clone(),
            app_data_dir,
        };
        assembly.assemble(&mut ctx)?;
    }

    Ok(())
}

/// 装配 AI IDE 业务 State。
///
/// 接收容器壳已装配的平台原语，返回业务上下文。装配顺序与重构前 `app::run` 的
/// `setup()` 完全一致，依赖链（storage → session_manager → … → mcp → lsp → gateway
/// → skills → project_manager）不可破坏。
#[allow(clippy::type_complexity)]
fn assemble_ai_ide(
    ctx: &mut BusinessAssemblyContext<'_>,
) -> Result<BusinessContext, Box<dyn std::error::Error>> {
    let event_bus = ctx.event_bus.clone();
    let storage = ctx.storage.clone();
    let sandbox = ctx.sandbox.clone();
    let policy_engine = ctx.policy_engine.clone();
    let auth = ctx.auth.clone();
    let config = ctx.config.clone();
    let app_data_dir = ctx.app_data_dir;
    let app = &mut *ctx.app;

    // Gateway 审计记录器（容器 storage 提供）
    let gateway_audit_recorder = storage.audit_recorder();

    // 1. 会话管理器
    let manager = Arc::new(session::SessionManager::new(
        SessionStore::new(storage.connection()),
        storage.audit_recorder(),
        event_bus.clone(),
    ));
    app.manage(manager.clone());

    // 2. Agent 任务管理器
    let task_manager = Arc::new(Mutex::new(agent::TaskManager::new()));
    app.manage(task_manager.clone());

    // 3. Composer 运行时
    let composer_runtime = Arc::new(Mutex::new(
        session::composer_runtime::ComposerRuntime::new(),
    ));
    app.manage(composer_runtime.clone());

    // 4. 工具审批存储（StreamIndex 是容器流基础设施，留在容器壳装配）
    let approval_store = Arc::new(ui::ToolApprovalStore::with_project_rule_store(
        app_data_dir.join("approval-rules.json"),
    ));
    app.manage(approval_store.clone());

    // 5. 终端管理器
    let terminal = Arc::new(TerminalManager::new(
        event_bus.clone(),
        sandbox.clone(),
        policy_engine.clone(),
    ));
    app.manage(terminal.clone());

    // 6. 业务约束注册（剪贴板约束 + Agent 默认允许约束；沙箱安全约束留在容器壳）
    register_clipboard_constraints(&policy_engine, sandbox.get_approval_mode())?;
    policy_engine.add(AgentDefaultAllowConstraint)?;

    // 7. MCP 引擎
    let mcp = Arc::new(mcp::MCP::with_deps_storage_and_policy(
        event_bus.clone(),
        sandbox.clone(),
        storage.clone(),
        policy_engine.clone(),
    )?);
    app.manage(mcp.clone());

    // 8. 项目目录管理器（依赖 config 的 recentWorktrees）
    let mut project_manager = catalog::ProjectManager::new(event_bus.clone());
    let config_guard = config
        .lock()
        .map_err(|_| anyhow::anyhow!("config lock poisoned"))?;
    if let Some(recent_worktrees) = config_guard.get("project.recentWorktrees") {
        let serialized = serde_json::to_string(&recent_worktrees)?;
        project_manager.load_recent_worktrees(&serialized)?;
    }
    drop(config_guard);
    let project_manager = Arc::new(Mutex::new(project_manager));
    app.manage(project_manager.clone());

    // 9. LSP 管理器
    let lsp = Arc::new(
        lsp::LSPManager::new(event_bus.clone()).map_err(|error| anyhow::anyhow!(error))?,
    );
    let _ = lsp::set_global_manager(lsp.clone());
    app.manage(lsp.clone());

    // 10. Gateway（依赖 config 的 gateway 配置 + auth）
    let config_guard = config
        .lock()
        .map_err(|_| anyhow::anyhow!("config lock poisoned"))?;
    let gateway_config = ui::gateway::gateway_config_from_config(&config_guard)
        .map_err(|error| anyhow::anyhow!(error))?;
    drop(config_guard);
    let gateway = Arc::new(gateway::Gateway::init(
        gateway_config,
        event_bus.clone(),
        gateway_audit_recorder,
        auth.clone(),
    )?);
    app.manage(gateway.clone());

    // 11. Skills（依赖 config Arc + event_bus）
    let mut skills = skills::Skills::with_event_bus(config.clone(), event_bus.clone())?;
    skills.load_all()?;
    let skills = Arc::new(Mutex::new(skills));
    app.manage(skills.clone());

    // 12. 后端扩展进程管理器（依赖 sandbox）：容器持有 spawn/kill 能力，
    //     供扩展生命周期 autostart spawn 与未来按需 spawn 命令使用。
    let backend_manager = Arc::new(backend::BackendProcessManager::new(sandbox.clone()));
    app.manage(backend_manager.clone());

    Ok(BusinessContext {
        manager,
        gateway,
        mcp,
        lsp,
        task_manager,
        composer_runtime,
        project_manager,
        skills,
        terminal,
        backend_manager,
        approval_store,
    })
}
