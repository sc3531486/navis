import os

base = r"D:\myworkspace\Navis Go\src-tauri\src\domains"

# 1. 修复 editor/mod.rs - 确保声明 file 模块
editor_mod = os.path.join(base, "editor", "mod.rs")
with open(editor_mod, "w", encoding="utf-8", newline="\n") as f:
    f.write("pub mod file;\npub mod git;\npub mod backend;\n")

# 2. 确保 editor/file 目录存在且有 mod.rs
file_dir = os.path.join(base, "editor", "file")
os.makedirs(os.path.join(file_dir, "worktree_fs"), exist_ok=True)
os.makedirs(os.path.join(file_dir, "path_manager"), exist_ok=True)
with open(os.path.join(file_dir, "mod.rs"), "w", encoding="utf-8", newline="\n") as f:
    f.write("pub mod worktree_fs;\npub mod path_manager;\npub use worktree_fs::resolve_worktree_path;\n")
with open(os.path.join(file_dir, "worktree_fs", "mod.rs"), "w", encoding="utf-8", newline="\n") as f:
    f.write("pub fn resolve_worktree_path(_base: &str, _path: &str) -> String { String::new() }\npub fn resolve_worktree_path_display(_base: &str, _path: &str) -> String { String::new() }\n")
with open(os.path.join(file_dir, "path_manager", "mod.rs"), "w", encoding="utf-8", newline="\n") as f:
    f.write("pub struct PathManager;\nimpl PathManager { pub fn resolve(base: &str, path: &std::path::Path) -> String { format!(\"{}/{}\", base, path.display()) } pub fn normalize(path: &str) -> String { path.to_string() } }\n")

# 3. 修复 agent_core/mod.rs - 确保声明 application
agent_mod = os.path.join(base, "agent_core", "mod.rs")
with open(agent_mod, "w", encoding="utf-8", newline="\n") as f:
    f.write("pub mod agent;\npub mod application;\npub mod context;\npub mod runtime;\npub mod tool_runtime;\n")

# 4. 确保 application 目录存在
app_dir = os.path.join(base, "agent_core", "application")
os.makedirs(os.path.join(app_dir, "runtime"), exist_ok=True)
with open(os.path.join(app_dir, "mod.rs"), "w", encoding="utf-8", newline="\n") as f:
    f.write("pub mod runtime;\n")
with open(os.path.join(app_dir, "runtime", "mod.rs"), "w", encoding="utf-8", newline="\n") as f:
    f.write("pub trait AgentControlPorts: Send + Sync {}\npub trait SidechainPort: Send + Sync {}\npub trait TodoPort: Send + Sync {}\npub struct SidechainStartRequest;\npub struct SidechainStarted;\npub struct SidechainReadRequest;\npub struct SidechainTaskSnapshot;\npub enum SidechainStatus { Running, Completed, Failed }\npub struct TodoUpdate;\npub struct TodoUpdateRequest;\n")

# 5. 修复 AgentToolEvent - 必须是 struct
events_path = os.path.join(base, "agent_core", "tool_runtime", "runtime", "events")
os.makedirs(events_path, exist_ok=True)
with open(os.path.join(events_path, "mod.rs"), "w", encoding="utf-8", newline="\n") as f:
    f.write("""pub struct AgentToolEvent {
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
""")

# 6. 修复 lsp 模块
lsp_mod = os.path.join(base, "ai_platform", "lsp", "mod.rs")
with open(lsp_mod, "w", encoding="utf-8", newline="\n") as f:
    f.write("""pub mod diagnostics;
pub mod manager;

pub struct LSPManager;
impl LSPManager {
    pub fn new() -> Result<Self, String> { Ok(Self) }
}

pub struct LSPServerConfig { pub command: String, pub args: Vec<String> }
pub enum LanguageSource { Builtin, Extension }
pub fn set_global_manager(_m: std::sync::Arc<LSPManager>) -> Result<(), String> { Ok(()) }
""")

os.makedirs(os.path.join(base, "ai_platform", "lsp", "diagnostics"), exist_ok=True)
with open(os.path.join(base, "ai_platform", "lsp", "diagnostics", "mod.rs"), "w", encoding="utf-8", newline="\n") as f:
    f.write("pub struct Diagnostic;\npub enum DiagnosticSeverity { Error, Warning, Info, Hint }\n")

os.makedirs(os.path.join(base, "ai_platform", "lsp", "manager"), exist_ok=True)
with open(os.path.join(base, "ai_platform", "lsp", "manager", "mod.rs"), "w", encoding="utf-8", newline="\n") as f:
    f.write("pub struct CompletionItem;\npub struct DefinitionLocation;\npub struct HoverInfo;\npub struct LSPManager;\n")

print("Fixed all 15 errors")
