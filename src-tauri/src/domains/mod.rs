//! 业务域物理归宿 — 扩展点代码的统一组织层
//!
//! 所有业务领域代码按扩展归类到此目录下。框架层（kernel/extension/foundation/security/app/ui）
//! 不依赖此模块；此模块只被 lib.rs 声明，提供从 crate::domains::* 访问业务类型的路径。
//!
//! ── 扩展映射 ──
//! navis-ai-platform: gateway/, mcp/, lsp/
//! navis-agent-core: agent/, context/, tool_runtime/, runtime/
//! navis-session: session/
//! navis-project: catalog/, knowledge/, memory/
//! navis-terminal: terminal/
//! navis-editor: file/, git/, clipboard/, backend/
//! navis-task: task/
//! navis-memory: memory/

pub mod ai_platform;
pub mod agent_core;
pub mod session;
pub mod project;
pub mod terminal;
pub mod editor;
pub mod task;
pub mod memory;
