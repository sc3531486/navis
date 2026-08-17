pub mod diagnostics;
pub mod manager;

pub struct LSPManager;
impl LSPManager {
    pub fn new() -> Result<Self, String> { Ok(Self) }
}

pub struct LSPServerConfig { pub command: String, pub args: Vec<String> }
pub enum LanguageSource { Builtin, Extension }
pub fn set_global_manager(_m: std::sync::Arc<LSPManager>) -> Result<(), String> { Ok(()) }
