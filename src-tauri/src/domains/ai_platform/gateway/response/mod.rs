//! Gateway 响应

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse { pub model: String, pub id: String, pub choices: Vec<Value>, pub usage: Option<Value>, pub finish_reason: Option<String> }
