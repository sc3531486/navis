mod business;
pub mod infra;

use std::fs;
use std::sync::{Arc, Mutex};

use chrono::Utc;

use tauri::Manager;

use crate::extension;
use crate::foundation::{config, stream};
use crate::kernel;
use crate::security::{auth, sandbox};
use crate::ui;


/// 扫描一个扩展源目录（app_data 或开发模式仓库源），把合法扩展注册进 ExtensionStore。
///
/// 目录名必须等于 manifest `id`（否则跳过并告警）；已注册的 id 跳过，避免重复。
fn scan_extension_source(
    extension_loader: &extension::ExtensionLoader,
    extension_store: &extension::ExtensionStore,
    source_dir: &std::path::Path,
    source_label: &str,
) -> anyhow::Result<()> {
    if !source_dir.is_dir() {
        tracing::warn!(
            source = %source_label,
            path = %source_dir.display(),
            "Extension source directory not found, skipped"
        );
        return Ok(());
    }
    let canonical_source_dir = source_dir.canonicalize()?;
    for entry in fs::read_dir(&canonical_source_dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() || !file_type.is_dir() {
            continue;
        }
        let extension_dir = entry.path();
        if !extension_dir.join("extension.json").is_file() {
            continue;
        }

        match extension_loader.load_manifest(&extension_dir) {
            Ok(manifest)
                if extension_dir.file_name().and_then(|name| name.to_str())
                    == Some(&manifest.id) =>
            {
                if extension_store.contains(&manifest.id) {
                    tracing::info!(
                        source = %source_label,
                        extension_id = %manifest.id,
                        "Extension already registered, skipped duplicate"
                    );
                    continue;
                }
                let state = extension::ExtensionState {
                    id: manifest.id.clone(),
                    status: extension::ExtensionStatus::Installed,
                    manifest,
                    install_path: extension_dir,
                    installed_at: Utc::now(),
                    enabled_at: None,
                    error: None,
                };
                extension_store.register(state)?;
            }
            Ok(manifest) => {
                tracing::warn!(
                    source = %source_label,
                    path = %extension_dir.display(),
                    manifest_id = %manifest.id,
                    "Skipped extension whose directory does not match its manifest ID"
                );
            }
            Err(error) => {
                tracing::warn!(
                    source = %source_label,
                    path = %extension_dir.display(),
                    error = %error,
                    "Failed to load extension manifest"
                );
            }
        }
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let icon =
                    tauri::image::Image::from_bytes(include_bytes!("../../icons/128x128.png"))?;
                window.set_icon(icon)?;
            }

            let runtime_handle = tauri::async_runtime::handle().inner().clone();
            let event_bus: Arc<dyn kernel::EventBus> =
                Arc::new(kernel::InMemoryEventBus::new(1000, runtime_handle));
            ui::tauri_events::publish_kernel_events_to_tauri(
                app.handle().clone(),
                event_bus.clone(),
            );
            let app_data_dir = app.path().app_data_dir()?;
            let db_path = app_data_dir
                .clone()
                .join(infra::db::DB_DIR_NAME)
                .join(infra::db::DB_FILE_NAME);
            let storage = Arc::new(infra::Storage::new(&db_path, None, event_bus.clone())?);
            app.manage(storage.clone());
            let auth = Arc::new(auth::Auth::open(
                &db_path,
                storage.encryption().cloned(),
                event_bus.clone(),
            )?);
            app.manage(auth.clone());
            let audit_recorder = storage.audit_recorder();
            let sandbox = Arc::new(
                sandbox::Sandbox::with_config(event_bus.clone(), Some(&app_data_dir), None)
                    .with_audit_recorder(audit_recorder),
            );
            let policy_engine = Arc::new(kernel::PolicyEngine::new());
            app.manage(policy_engine.clone());
            // 沙箱安全约束属于容器职责，在业务装配前注册到共享 PolicyEngine。
            sandbox::constraint::register_sandbox_constraints(&policy_engine, sandbox.clone())?;
            // 容器流基础设施（StreamIndex 是平台原语，不属于业务）。
            app.manage(Arc::new(stream::StreamIndex::new()));

            // 配置装配（容器与扩展系统共用；业务通过 Arc<Mutex<Config>> 读值）。
            let mut config = config::Config::new(event_bus.clone());
            let config_path = app_data_dir.join("config.json");
            config.load_user_config(&config_path)?;
            let config = Arc::new(Mutex::new(config));
            app.manage(config.clone());
            app.manage(Arc::new(
                ui::extension_storage::ExtensionEphemeralStorage::default(),
            ));

            // 业务装配（从容器壳物理抽离）。
            business::assemble_registered(
                app,
                event_bus.clone(),
                storage.clone(),
                sandbox.clone(),
                policy_engine.clone(),
                auth.clone(),
                config.clone(),
                &app_data_dir,
            )?;

            // 领域无关受控操作注册表（设计 35 §3.2）：机制在容器，操作定义由扩展注册。
            let operation_registry = Arc::new(extension::operation_runtime::OperationRegistry::default());
            app.manage(operation_registry.clone());
            let extension_store = Arc::new(extension::ExtensionStore::new(event_bus.clone()));
            let extensions_dir = app_data_dir.join("extensions");
            fs::create_dir_all(&extensions_dir)?;
            let extension_loader = extension::ExtensionLoader::new();
            // 运行时扩展目录（<app_data>/extensions/）：安装态扩展。
            scan_extension_source(&extension_loader, &extension_store, &extensions_dir, "app_data")?;
            // 开发模式扩展源（仓库 extensions/）：通过 NAVIS_EXTENSIONS_DIR 指定，
            // 让固定目录 ExtensionUI/ExtensionBackend 中的扩展在 dev 下可直接装载。
            if let Ok(dev_extensions_dir) = std::env::var("NAVIS_EXTENSIONS_DIR") {
                if !dev_extensions_dir.trim().is_empty() {
                    scan_extension_source(
                        &extension_loader,
                        &extension_store,
                        std::path::Path::new(&dev_extensions_dir),
                        "dev_source",
                    )?;
                }
            }
            let extension_installer = Arc::new(extension::ExtensionInstaller::new(
                extensions_dir,
                extension_store.clone(),
                event_bus.clone(),
            ));
            // 组件注册表（设计 37 §C1-4）：扩展组件装配入口，依赖沙箱、操作注册表与扩展仓库。
            let component_registry = Arc::new(crate::extension::component::ComponentRegistry::new(
                sandbox.clone(),
                operation_registry.clone(),
                extension_store.clone(),
            ));
            app.manage(component_registry.clone());
            let extension_cordis_runtime =
                Arc::new(extension::context::HostExtensionContext::new());
            let provider_validation = Arc::new(
                extension::ExtensionProviderValidationRegistry::new(Arc::new(
                    extension::HttpExtensionProviderValidationAdapter::new(
                        auth::key_validator::reqwest_validation_transport()?,
                        auth.clone(),
                    ),
                )),
            );
            let event_subscription_port = Arc::new(
                extension::lifecycle::KernelEventSubscriptionAdapter::new(event_bus.clone()),
            );

            // 容器级 capability service：与业务无关的平台能力端口，白板空壳也注册。
            // capability port 可选注入：业务能力（MCP/LSP/Gateway/BackendManager）只在
            // 业务装配分支注册，白板空壳下缺省，扩展 apply 内 `ctx.get` 返回 None →
            // fail-closed（与 ExtensionLifecycle 现有一致语义）。
            extension_cordis_runtime.register_capability_service::<crate::kernel::PolicyEngine>(
                extension::lifecycle::cordis::SERVICE_POLICY_ENGINE,
                policy_engine.clone(),
            )?;
            extension_cordis_runtime
                .register_capability_service::<dyn extension::ExtensionProviderValidationPort>(
                    extension::lifecycle::cordis::SERVICE_PROVIDER_VALIDATION,
                    provider_validation.clone(),
                )?;
            extension_cordis_runtime
                .register_capability_service::<dyn extension::lifecycle::EventSubscriptionPort>(
                    extension::lifecycle::cordis::SERVICE_EVENT_SUBSCRIPTION,
                    event_subscription_port.clone(),
                )?;
            extension_cordis_runtime
                .register_capability_service::<extension::component::ComponentRegistry>(
                    extension::lifecycle::cordis::SERVICE_COMPONENT_REGISTRY,
                    component_registry.clone(),
                )?;

            // 业务装配结果以 Tauri State 存在；白板空壳启动时为空，扩展生命周期
            // 只装配平台能力端口，不装配 AI 业务端口。
            let extension_lifecycle = match app.try_state::<business::BusinessContext>() {
                Some(business) => {
                    extension_cordis_runtime
                        .register_capability_service::<dyn extension::lifecycle::McpCapabilityPort>(
                            extension::lifecycle::cordis::SERVICE_MCP,
                            business.mcp.clone(),
                        )?;
                    extension_cordis_runtime
                        .register_capability_service::<dyn extension::lifecycle::LspCapabilityPort>(
                            extension::lifecycle::cordis::SERVICE_LSP,
                            business.lsp.clone(),
                        )?;
                    extension_cordis_runtime
                        .register_capability_service::<dyn extension::lifecycle::GatewayCapabilityPort>(
                            extension::lifecycle::cordis::SERVICE_GATEWAY,
                            business.gateway.clone(),
                        )?;
                    extension_cordis_runtime
                        .register_capability_service::<crate::domains::editor::backend::BackendProcessManager>(
                            extension::lifecycle::cordis::SERVICE_BACKEND_MANAGER,
                            business.backend_manager.clone(),
                        )?;
                    // agentLoop 编排服务缝（38 §2.5）：默认实现为 D4 占位，容器
                    // 组合根在业务分支注册；白板空壳（无 business）不注册 agentLoop，
                    // 扩展 apply 内 `ctx.get` 返回 None → fail-closed。
                    extension_cordis_runtime
                        .register_capability_service::<dyn extension::lifecycle::cordis::AgentLoopPort>(
                            extension::lifecycle::cordis::SERVICE_AGENT_LOOP,
                            Arc::new(extension::lifecycle::cordis::DefaultAgentLoopPort),
                        )?;
                    Arc::new(
                        extension::ExtensionLifecycle::new(
                            extension_store.clone(),
                            business.skills.clone(),
                            event_bus.clone(),
                        )
                        .with_mcp(business.mcp.clone())
                        .with_lsp(business.lsp.clone())
                        .with_policy_engine(policy_engine.clone())
                        .with_gateway(business.gateway.clone())
                        .with_provider_validation(provider_validation)
                        .with_event_subscription_port(event_subscription_port)
                        .with_backend_manager(business.backend_manager.clone())
                        .with_component_registry(component_registry.clone())
                        .with_cordis(extension_cordis_runtime.clone()),
                    )
                }
                None => Arc::new(
                    extension::ExtensionLifecycle::new_without_skills(
                        extension_store.clone(),
                        event_bus.clone(),
                    )
                    .with_policy_engine(policy_engine.clone())
                    .with_provider_validation(provider_validation)
                    .with_event_subscription_port(event_subscription_port)
                    .with_component_registry(component_registry.clone())
                    .with_cordis(extension_cordis_runtime.clone()),
                ),
            };
            // 生命周期 capability 服务：default_apply 经 ctx.get 惰性解析全局
            // ExtensionLifecycle 后提交真实注册（D1c）。
            extension_cordis_runtime
                .register_capability_service::<extension::ExtensionLifecycle>(
                    extension::lifecycle::cordis::SERVICE_LIFECYCLE,
                    extension_lifecycle.clone(),
                )?;
            app.manage(extension_cordis_runtime);
            app.manage(extension_store);
            app.manage(extension_installer);
            app.manage(extension_lifecycle);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ui::menus::ui_list_menus,
            ui::menus::ui_list_extension_commands,
            ui::menus::ui_list_extension_keybindings,
            ui::menus::ui_list_slash_commands,
            ui::extension_bridge::ui_extension_bridge_invoke,
            ui::extension_bridge::ui_extension_bridge_authorize_event,
            ui::extension_router::ui_extension_route_call,
            ui::extension_network::ui_extension_network_proxy,
            ui::extension_storage::ui_extension_storage_clear,
            ui::extension_storage::ui_extension_storage_delete,
            ui::extension_storage::ui_extension_storage_set,
            ui::extension_storage::ui_extension_storage_get,
            extension::operation_runtime::ui_operation_register,
            extension::operation_runtime::ui_operation_execute,
            extension::operation_runtime::ui_operation_list,
            ui::extensions::ui_get_extension_config,
            ui::extensions::ui_set_extension_config,
            ui::extensions::ui_list_extension_points,
            ui::extensions::ui_extension_discovery_query,
            ui::extensions::ui_list_extension_locales,
            ui::extensions::ui_list_extension_scripts,
            ui::extensions::ui_list_zones,
            ui::extensions::ui_list_extensions,
            ui::extensions::ui_get_extension_view,
            ui::extensions::ui_list_extension_views,
            ui::extensions::ui_read_extension_entry_html,
            ui::extensions::ui_set_extension_enabled,
            ui::extensions::ui_install_extension,
            ui::extensions::ui_uninstall_extension,
            ui::extensions::ui_list_custom_work_modes,
            ui::gateway::ui_list_gateway_providers,
            ui::gateway::ui_list_gateway_models,
            ui::gateway::ui_get_gateway_catalog,
            ui::worktree::ui_list_recent_worktrees,
            ui::worktree::ui_record_recent_worktree,
            ui::worktree::ui_remove_recent_worktree,
            ui::worktree::ui_get_session_worktree_snapshot,
            ui::worktree::ui_read_session_worktree_file,
            ui::worktree::ui_write_session_worktree_file,
            ui::gateway::ui_discover_gateway_models,
            ui::gateway::ui_get_gateway_config,
            ui::gateway::ui_save_gateway_config,
            ui::settings::ui_get_editor_settings,
            ui::settings::ui_save_editor_settings,
            ui::settings::ui_open_session_external_editor,
            ui::settings::ui_get_language,
            ui::settings::ui_set_language,
            ui::sessions::ui_list_session_tree,
            ui::messages::ui_list_session_messages,
            ui::messages::ui_list_session_changes,
            ui::tasks::composer_commands::ui_get_session_composer_run_state,
            ui::tasks::composer_commands::ui_set_session_composer_run_state,
            ui::tasks::composer_commands::ui_submit_composer_task,
            ui::tasks::composer_commands::ui_finish_composer_task,
            ui::tasks::composer_commands::ui_clear_running_composer_task,
            ui::tasks::composer_commands::ui_remove_queued_composer_task,
            ui::tasks::composer_commands::ui_promote_queued_composer_task,
            ui::tasks::goal_runner_commands::ui_start_goal_runner,
            ui::tasks::goal_runner_commands::ui_pause_goal_runner,
            ui::tasks::goal_runner_commands::ui_resume_goal_runner,
            ui::tasks::goal_runner_commands::ui_stop_goal_runner,
            ui::tasks::context_usage::ui_get_session_context_usage,
            ui::tasks::git_diff::ui_get_session_git_diff,
            ui::tasks::git_diff::ui_create_session_git_repo,
            ui::lsp::lsp_completion,
            ui::lsp::lsp_hover,
            ui::lsp::lsp_definition,
            ui::lsp::lsp_diagnostics,
            ui::lsp::lsp_format,
            ui::tasks::task_commands::ui_list_tasks,
            ui::tasks::task_commands::ui_stop_task,
            ui::tasks::task_commands::ui_clear_finished_tasks,
            ui::tasks::task_commands::ui_list_session_todos,
            ui::tasks::task_commands::ui_cancel_stream,
            ui::tasks::task_commands::ui_respond_tool_approval,
            ui::terminal::ui_terminal_create_pty,
            ui::terminal::ui_terminal_write_pty,
            ui::terminal::ui_terminal_resize_pty,
            ui::terminal::ui_terminal_close_pty,
            ui::ui_stream_session_message,
            ui::sessions::ui_create_session,
            ui::sessions::ui_set_active_session,
            ui::sessions::ui_rename_session,
            ui::sessions::ui_set_session_model,
            ui::sessions::ui_set_session_permission_policy,
            ui::sessions::ui_set_session_transcript_view,
            ui::sessions::ui_set_session_reasoning_effort,
            ui::sessions::ui_set_session_worktree_root,
            ui::sessions::ui_archive_session,
            ui::sessions::ui_delete_session,
            ui::sessions::ui_fork_session,
            ui::sessions::ui_set_session_pinned,
            ui::sessions::ui_set_session_unread,
            ui::sessions::ui_move_session_to_worktree,
            ui::sessions::ui_rename_worktree,
            ui::sessions::ui_delete_worktree,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

