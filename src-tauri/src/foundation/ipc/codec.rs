//! 消息编解码
//!
//! 基于设计文档 §2.2 实现，提供消息序列化/反序列化能力
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::IpcError;

/// 编解码器 trait
pub trait Codec: Send + Sync {
    /// 编码消息
    fn encode<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, IpcError>;

    /// 解码消息
    fn decode<T: for<'de> Deserialize<'de>>(&self, data: &[u8]) -> Result<T, IpcError>;

    /// 编码为 JSON Value
    fn encode_value<T: Serialize>(&self, value: &T) -> Result<Value, IpcError>;

    /// 从 JSON Value 解码
    fn decode_value<T: for<'de> Deserialize<'de>>(&self, value: &Value) -> Result<T, IpcError>;
}

/// JSON 编解码器
pub struct JsonCodec;

impl JsonCodec {
    /// 创建新的 JSON 编解码器
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Codec for JsonCodec {
    fn encode<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, IpcError> {
        tracing::debug!("Encoding message to JSON");

        serde_json::to_vec(value).map_err(|e| {
            tracing::error!(error = %e, "Failed to encode message to JSON");
            IpcError::serialization_error(format!("Failed to encode: {}", e))
        })
    }

    fn decode<T: for<'de> Deserialize<'de>>(&self, data: &[u8]) -> Result<T, IpcError> {
        tracing::debug!(data_len = data.len(), "Decoding message from JSON");

        serde_json::from_slice(data).map_err(|e| {
            tracing::error!(error = %e, "Failed to decode message from JSON");
            IpcError::serialization_error(format!("Failed to decode: {}", e))
        })
    }

    fn encode_value<T: Serialize>(&self, value: &T) -> Result<Value, IpcError> {
        tracing::debug!("Encoding message to JSON Value");

        serde_json::to_value(value).map_err(|e| {
            tracing::error!(error = %e, "Failed to encode message to JSON Value");
            IpcError::serialization_error(format!("Failed to encode: {}", e))
        })
    }

    fn decode_value<T: for<'de> Deserialize<'de>>(&self, value: &Value) -> Result<T, IpcError> {
        tracing::debug!("Decoding message from JSON Value");

        serde_json::from_value(value.clone()).map_err(|e| {
            tracing::error!(error = %e, "Failed to decode message from JSON Value");
            IpcError::serialization_error(format!("Failed to decode: {}", e))
        })
    }
}

/// IPC 命令消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcCommand {
    /// 命令名（如 "agent.cancelTask"）
    pub name: String,
    /// 请求参数
    pub payload: Value,
    /// 请求 ID（用于超时追踪）
    pub request_id: String,
    /// 关联会话
    pub session_id: Option<String>,
}

impl IpcCommand {
    /// 创建新的 IPC 命令
    pub fn new(name: impl Into<String>, payload: Value, session_id: Option<String>) -> Self {
        Self {
            name: name.into(),
            payload,
            request_id: uuid::Uuid::new_v4().to_string(),
            session_id,
        }
    }

    /// 创建带会话的命令
    pub fn with_session(
        name: impl Into<String>,
        payload: Value,
        session_id: impl Into<String>,
    ) -> Self {
        Self::new(name, payload, Some(session_id.into()))
    }
}

/// IPC 响应消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    /// 请求 ID
    pub request_id: String,
    /// 响应结果
    pub result: Result<Value, IpcError>,
}

impl IpcResponse {
    /// 创建成功响应
    pub fn success(request_id: impl Into<String>, data: Value) -> Self {
        Self {
            request_id: request_id.into(),
            result: Ok(data),
        }
    }

    /// 创建错误响应
    pub fn error(request_id: impl Into<String>, error: IpcError) -> Self {
        Self {
            request_id: request_id.into(),
            result: Err(error),
        }
    }

    /// 检查是否成功
    pub fn is_success(&self) -> bool {
        self.result.is_ok()
    }

    /// 获取响应数据（如果是成功）
    pub fn data(&self) -> Option<&Value> {
        self.result.as_ref().ok()
    }

    /// 获取错误（如果是失败）
    pub fn get_error(&self) -> Option<&IpcError> {
        self.result.as_ref().err()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_codec_encode_decode() {
        let codec = JsonCodec::new();
        let data = json!({"key": "value", "number": 42});

        // 编码
        let encoded = codec.encode(&data).unwrap();
        assert!(!encoded.is_empty());

        // 解码
        let decoded: Value = codec.decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_json_codec_encode_value() {
        let codec = JsonCodec::new();
        let data = json!({"key": "value"});

        let value = codec.encode_value(&data).unwrap();
        assert_eq!(value, data);
    }

    #[test]
    fn test_json_codec_decode_value() {
        let codec = JsonCodec::new();
        let value = json!({"key": "value"});

        let decoded: Value = codec.decode_value(&value).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn test_json_codec_struct() {
        let codec = JsonCodec::new();

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct TestData {
            name: String,
            age: u32,
        }

        let data = TestData {
            name: "Alice".to_string(),
            age: 30,
        };

        let encoded = codec.encode(&data).unwrap();
        let decoded: TestData = codec.decode(&encoded).unwrap();

        assert_eq!(decoded, data);
    }

    #[test]
    fn test_ipc_command_new() {
        let cmd = IpcCommand::new(
            "agent.cancelTask",
            json!({"taskId": "task_001"}),
            Some("sess_001".to_string()),
        );

        assert_eq!(cmd.name, "agent.cancelTask");
        assert_eq!(cmd.payload, json!({"taskId": "task_001"}));
        assert_eq!(cmd.session_id, Some("sess_001".to_string()));
        assert!(!cmd.request_id.is_empty());
    }

    #[test]
    fn test_ipc_command_with_session() {
        let cmd = IpcCommand::with_session(
            "agent.cancelTask",
            json!({"taskId": "task_001"}),
            "sess_001",
        );

        assert_eq!(cmd.session_id, Some("sess_001".to_string()));
    }

    #[test]
    fn test_ipc_response_success() {
        let response = IpcResponse::success("req_001", json!({"result": "ok"}));

        assert!(response.is_success());
        assert_eq!(response.request_id, "req_001");
        assert_eq!(response.data(), Some(&json!({"result": "ok"})));
        assert!(response.get_error().is_none());
    }

    #[test]
    fn test_ipc_response_error() {
        let error = IpcError::command_not_found("test.command");
        let response = IpcResponse::error("req_001", error);

        assert!(!response.is_success());
        assert_eq!(response.request_id, "req_001");
        assert!(response.data().is_none());
        assert!(response.get_error().is_some());
    }
}
