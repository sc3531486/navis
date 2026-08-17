pub struct AgentToolEvent {
    pub tool: String,
    pub status: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub detail: Option<String>,
    pub summary: Option<String>,
    pub output: Option<String>,
    pub progress: Option<f32>,
    pub metadata: Option<serde_json::Value>,
    pub input: Option<String>,
    pub title: Option<String>,
    pub call_id: Option<String>,
    pub gateway_tool: Option<String>,
}
pub struct AgentToolExecution;
pub enum AgentToolPhase { Pre, Main, Post }
pub struct AgentToolProgressCallback;
pub enum AgentToolStatus { Pending, Running, Done }
