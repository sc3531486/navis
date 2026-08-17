//! Agent 决策引擎模块（扩展点）
//!
//! 从 `src-tauri/src/ai/agent/` 迁入，管理对话的 Task 编排、工具调用决策、
//! 工作流执行、Extended Thinking、自我进化。
//!
//! # 迁移来源
//!
//! ```text
//! src-tauri/src/ai/agent/
//! ├── confirm_handler.rs      → 用户确认处理
//! ├── context_compress.rs     → 上下文压缩触发
//! ├── goal_runner.rs          → Goal 续跑决策
//! ├── instruction_resolver.rs → 指令解析
//! ├── mod.rs                  → 模块声明 + WorkMode/AgentContext
//! ├── sidechain.rs            → Sidechain 任务管理
//! ├── task_manager.rs         → Task 管理器
//! ├── task_scheduler.rs       → Task 并行调度
//! ├── thinking.rs             → Extended Thinking
//! ├── turn_context.rs         → Turn 上下文
//! ├── turn_output.rs          → Turn 输出归一化
//! └── self_evolution/
//!     ├── mod.rs
//!     └── experience_logger.rs
//! ```
//!
//! # re-export 桥
//!
//! Phase 0 阶段，所有类型从原始模块重导出。
//! 后续 Phase 将物理搬迁源文件到此目录。

pub use crate::ai::agent::confirm_handler;
pub use crate::ai::agent::context_compress;
pub use crate::ai::agent::goal_runner;
pub use crate::ai::agent::instruction_resolver;
pub use crate::ai::agent::self_evolution;
pub use crate::ai::agent::sidechain;
pub use crate::ai::agent::task_manager;
pub use crate::ai::agent::task_scheduler;
pub use crate::ai::agent::thinking;
pub use crate::ai::agent::turn_context;
pub use crate::ai::agent::turn_output;

// 重导出核心类型（保持原 agent/mod.rs 的公开 API）
pub use crate::ai::agent::{
    ConfirmDecision, ConfirmError, ConfirmHandler, ConfirmRequest, ConfirmStatus,
};
pub use crate::ai::agent::{
    CompressionRequest, CompressionResult, CompressionStrategy, ContextCompressManager,
};
pub use crate::ai::agent::{
    apply_goal_runner_command, decide_goal_runner_next_task, GoalRunnerCommand, GoalRunnerDecision,
    GoalRunnerRequest, GoalRunnerStatePatch,
};
pub use crate::ai::agent::{InstructionResolver, ResolvedInstruction};
pub use crate::ai::agent::{
    ExecutionExperience, ExperienceFilter, ExperienceLogger, ExperienceOutcome,
};
pub use crate::ai::agent::{
    mark_sidechain_failed_and_notify, notify_parent_sidechain_task,
    notify_parent_sidechain_task_best_effort, sidechain_outcome_from_assistant_content,
    sidechain_parent_notification_metadata, sidechain_stop_requested, task_description,
    update_sidechain_progress,
};
pub use crate::ai::agent::{
    ChatMessage, SidechainOutcome, TaskKind, TaskManager, TaskRecord, TaskStatus, TodoItem,
    TodoStatus,
};
pub use crate::ai::agent::{
    SchedulerConfig, SchedulerError, TaskBranchContext, TaskBranchResult, TaskBranchSpec,
    TaskProgress, TaskScheduler,
};
pub use crate::ai::agent::{ThinkingConfig, ThinkingManager, ThinkingStatus, ThinkingTrace};
pub use crate::ai::agent::{mode_config_from_key, AgentTurnContext};
