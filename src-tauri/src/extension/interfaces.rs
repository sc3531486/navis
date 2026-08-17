//! 扩展接口定义
//!
//! 框架层定义的扩展接口，业务扩展实现这些接口

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ══════════════════════════════════════════════════════════
// Gateway 接口
// ══════════════════════════════════════════════════════════

/// 模型网关接口
pub trait GatewayPort: Send + Sync {
    /// 发送聊天请求
    fn chat(&self, request: ChatRequest) -> Result<ChatResponse, String>;
    /// 获取模型列表
    fn list_models(&self) -> Vec<ModelInfo>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole { System, User, Assistant, Tool }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
}

// ══════════════════════════════════════════════════════════
// MCP 接口
// ══════════════════════════════════════════════════════════

/// MCP 工具引擎接口
pub trait McpPort: Send + Sync {
    /// 注册工具
    fn register_tool(&self, tool: ToolDefinition) -> Result<(), String>;
    /// 执行工具
    fn execute_tool(&self, name: &str, args: Value) -> Result<Value, String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

// ══════════════════════════════════════════════════════════
// Session 接口
// ══════════════════════════════════════════════════════════

/// 会话管理接口
pub trait SessionPort: Send + Sync {
    /// 创建会话
    fn create_session(&self, name: &str) -> Result<Session, String>;
    /// 获取会话
    fn get_session(&self, id: &str) -> Result<Option<Session>, String>;
    /// 添加消息
    fn add_message(&self, session_id: &str, message: Message) -> Result<(), String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: String,
}

// ══════════════════════════════════════════════════════════
// Agent 接口
// ══════════════════════════════════════════════════════════

/// Agent 运行时接口
pub trait AgentPort: Send + Sync {
    /// 执行 Agent turn
    fn execute_turn(&self, session_id: &str, input: &str) -> Result<String, String>;
}

// ══════════════════════════════════════════════════════════
// Terminal 接口
// ══════════════════════════════════════════════════════════

/// 终端管理接口
pub trait TerminalPort: Send + Sync {
    /// 创建终端
    fn create_pty(&self) -> Result<String, String>;
    /// 写入终端
    fn write_pty(&self, id: &str, data: &str) -> Result<(), String>;
}
