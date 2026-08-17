//! Agent 决策引擎

pub mod turn_output;
pub mod sidechain;

pub use turn_output::{assistant_visible_content, assistant_visible_delta, has_open_internal_details_block, should_buffer_potential_text_tool_call};

pub struct TaskManager;
impl TaskManager {
    pub fn new() -> Self { Self }
    pub fn get_task_mut(&mut self, _id: &str) -> Option<&mut TaskRecord> { None }
}

pub struct TaskRecord;
impl TaskRecord { pub fn mark_failed(&mut self, _error: &str) {} }

pub enum TaskKind { Agent, Sidechain, Background }
pub enum TaskStatus { Running, Completed, Failed }
pub struct SidechainOutcome;
pub struct AgentTurnContext;
pub struct ChatMessage;
pub struct TodoItem;
pub enum TodoStatus { Pending, Done }

pub fn notify_parent_sidechain_task(_manager: &TaskManager, _task_id: &str) {}
pub fn notify_parent_sidechain_task_best_effort(_manager: &TaskManager, _task_id: &str) {}
pub fn mark_sidechain_failed_and_notify(_manager: &TaskManager, _task_id: &str, _error: &str) {}
pub fn sidechain_outcome_from_assistant_content(_content: &str) -> SidechainOutcome { SidechainOutcome }
pub fn task_description(_task: &TaskRecord) -> &str { "" }
pub fn mode_config_from_key(_key: Option<&str>) -> String { "default".to_string() }
pub fn sidechain_stop_requested(_manager: &TaskManager, _task_id: &str) -> bool { false }
pub fn update_sidechain_progress(_manager: &TaskManager, _task_id: &str, _progress: f32) {}
pub fn apply_goal_runner_command(_cmd: ()) {}
pub fn decide_goal_runner_next_task() -> () {}
pub struct GoalRunnerCommand;
pub struct GoalRunnerDecision;
pub struct GoalRunnerRequest;
pub struct GoalRunnerStatePatch;
