use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Debug, Clone, Serialize, Deserialize)] pub enum MessageRole { System, User, Assistant, Tool }
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct ChatMessage { pub role: MessageRole, pub content: String }
impl ChatMessage { pub fn system(c: impl Into<String>) -> Self { Self { role: MessageRole::System, content: c.into() } } pub fn user(c: impl Into<String>) -> Self { Self { role: MessageRole::User, content: c.into() } } }
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct ChatRequest { pub model: String, pub messages: Vec<ChatMessage> }
impl ChatRequest { pub fn new(m: impl Into<String>, msgs: Vec<ChatMessage>) -> Self { Self { model: m.into(), messages: msgs } } pub fn with_tools(self, _: Vec<ToolDefinition>) -> Self { self } }
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct ToolCall { pub id: String, pub name: String, pub arguments: Value }
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct ToolDefinition { pub name: String, pub description: String, pub input_schema: Value }
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct GatewayConfig;
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct ModelConfig;
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct ProviderConfig;
#[derive(Debug, Clone, Serialize, Deserialize)] pub enum ApiProtocol { ChatCompletions, Responses, Custom }
impl ApiProtocol { pub fn from_str(s: &str) -> Self { match s { "responses" => Self::Responses, _ => Self::ChatCompletions } } }
#[derive(Debug, Clone, Serialize, Deserialize)] pub enum MessageContent { Text(String), Parts(Vec<Value>) }
