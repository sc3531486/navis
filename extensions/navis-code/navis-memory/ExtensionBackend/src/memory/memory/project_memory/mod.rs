//! 项目记忆
pub struct MemoryStore;
impl MemoryStore {
    pub fn new() -> Self { Self }
    pub fn save(&self, _memory: &()) -> Result<(), String> { Ok(()) }
    pub fn search(&self, _query: &str) -> Result<Vec<()>, String> { Ok(vec![]) }
}
