//! UI Framework — 宿主视图投影与扩展机制
//!
//! 框架层模块（🏠）：
//! - dto: 数据传输对象
//! - extension_bridge: 扩展桥接
//! - extension_network: 扩展网络
//! - extension_router: 扩展路由
//! - extension_storage: 扩展存储
//! - extension_stream: 扩展流
//! - extensions: 扩展管理
//! - host_view: 宿主视图
//! - menus: 菜单系统
//! - tauri_events: Tauri 事件
//!
//! 业务层模块已移到各自扩展：
//! - gateway/lsp → navis-ai-platform
//! - messages/sessions → navis-session
//! - terminal → navis-terminal
//! - worktree/project → navis-project
//! - agent_timeline/timeline/runtime → navis-agent-core
//! - tasks → navis-task
//! - settings → navis-settings

mod dto;
pub mod extension_bridge;
pub mod extension_network;
pub mod extension_router;
pub mod extension_storage;
pub mod extension_stream;
pub mod extensions;
pub(crate) mod host_view;
pub mod menus;
pub mod tauri_events;

pub use dto::*;
