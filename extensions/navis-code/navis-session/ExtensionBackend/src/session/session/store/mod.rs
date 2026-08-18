//! 会话存储

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String, pub name: Option<String>, pub worktree_root: Option<String>,
    pub status: SessionStatus, pub model: Option<String>, pub provider_id: Option<String>,
    pub metadata: Option<Value>, pub system_prompt: Option<String>,
    pub permission_policy: Option<String>, pub ui_metadata: Value,
    pub total_tokens: i64, pub created_at: String, pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus { Active, Archived, Deleted }

pub struct SessionManager;
impl SessionManager {
    pub fn new() -> Self { Self }
    pub fn get(&self, _id: &str) -> Result<Option<Session>, String> { Ok(None) }
    pub fn update(&self, _id: &str, _name: Option<&str>, _model: Option<&str>, _sp: Option<&str>) -> Result<(), String> { Ok(()) }
    pub fn update_metadata(&self, _id: &str, _m: Option<Value>) -> Result<(), String> { Ok(()) }
    pub fn update_message_content(&self, _: &str, _: &str, _: MessageContent, _: Option<i64>, _: Option<Value>) -> Result<(), String> { Ok(()) }
    pub fn rename(&self, _: &str, _: &str) -> Result<(), String> { Ok(()) }
    pub fn add_message(&self, _: &str, _: Message) -> Result<(), String> { Ok(()) }
    pub fn get_messages(&self, _: &str, _: Option<i64>, _: Option<i64>) -> Result<Vec<Message>, String> { Ok(vec![]) }
}

pub struct SessionStore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message { pub id: String, pub session_id: String, pub role: MessageRole, pub content: MessageContent, pub token_count: Option<i64>, pub created_at: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent { Text(String), Parts(Vec<ContentPart>) }

impl std::fmt::Display for MessageContent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self { MessageContent::Text(s) => write!(f, "{}", s), _ => Ok(()) }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentPart { Text(TextContent), Image(ImageContent), File(FileContent) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent { pub text: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent { pub media_type: String, pub data: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent { pub file_name: String, pub content: String }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole { User, Assistant, System, Tool }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TimelineStatus { Pending, Running, Completed, Error }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTimelinePart { pub id: String, pub session_id: String, pub turn_id: String, pub message_id: String, pub sequence: i64, pub kind: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactedRange;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionChange;
