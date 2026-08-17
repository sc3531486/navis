// ── 归属扩展：navis-task ──
// 迁移目标：extensions/navis-task/ExtensionBackend/src/

mod common;
pub(crate) mod composer_commands;
pub(crate) mod context_model;
pub(crate) mod context_usage;
pub(crate) mod git_diff;
pub(crate) mod goal_runner_commands;
pub(crate) mod task_commands;
mod task_projection;

#[cfg(test)]
pub(crate) use task_projection::is_background_task_projection;
pub(crate) use task_projection::{todo_items_from_values, todo_values};
