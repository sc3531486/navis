//! 流式数据类型定义
//!
//! 定义 Navis 流式通信的核心数据结构。
//! 所有流式数据传输都基于这些类型。

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

// ============================================================================
// 类型别名
// ============================================================================

/// 流 ID（UUID 字符串）
pub type StreamId = String;

// ============================================================================
// 流来源标识
// ============================================================================

/// 流来源标识（开放结构体，任意模块/扩展都能创建）
///
/// 已知类型用 `stream_kind` 常量保证拼写正确，
/// 未知类型直接写字符串，扩展不需要改 Stream 模块源码。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSource {
    /// 流类型标识（如 "terminal" / "agent" / "extension" / "jira-live-feed"）
    pub kind: String,
    /// 业务 ID（terminal_id / session_id / extension_id 等）
    pub id: String,
    /// 任意附加信息
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// 已知流类型常量
pub mod stream_kind {
    /// 终端流
    pub const TERMINAL: &str = "terminal";
    /// Agent 流
    pub const AGENT: &str = "agent";
    /// 扩展流
    pub const EXTENSION: &str = "extension";
    /// Gateway 流
    pub const GATEWAY: &str = "gateway";
}

impl StreamSource {
    /// 创建新的流来源
    pub fn new(kind: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            id: id.into(),
            metadata: HashMap::new(),
        }
    }

    /// 添加元数据
    pub fn with_meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// 获取元数据
    pub fn meta(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }

    /// 是否为指定类型
    pub fn is_kind(&self, kind: &str) -> bool {
        self.kind == kind
    }
}

impl std::fmt::Display for StreamSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.kind, self.id)
    }
}

// ============================================================================
// 流式数据块
// ============================================================================

/// 流式数据块类型。
///
/// `Data` 承载业务数据，`Error` 表示流已异常结束，`Done` 表示正常结束。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StreamChunkKind {
    Data,
    Error,
    Done,
    Cancelled,
}

/// 流式数据块（后端→后端通信使用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// 流 ID
    pub stream_id: StreamId,
    /// 序列号（单调递增）
    pub sequence: u64,
    /// Chunk 类型
    pub kind: StreamChunkKind,
    /// 业务数据
    pub data: Value,
    /// 是否为最后一个 chunk
    pub is_final: bool,
}

impl StreamChunk {
    /// 创建数据 chunk
    pub fn data(stream_id: &str, sequence: u64, data: Value) -> Self {
        Self {
            stream_id: stream_id.to_string(),
            sequence,
            kind: StreamChunkKind::Data,
            data,
            is_final: false,
        }
    }

    /// 创建错误 chunk。
    pub fn error(stream_id: &str, sequence: u64, message: impl Into<String>) -> Self {
        Self {
            stream_id: stream_id.to_string(),
            sequence,
            kind: StreamChunkKind::Error,
            data: json!({ "error": message.into() }),
            is_final: true,
        }
    }

    /// 创建结束 chunk
    pub fn final_chunk(stream_id: &str, sequence: u64) -> Self {
        Self {
            stream_id: stream_id.to_string(),
            sequence,
            kind: StreamChunkKind::Done,
            data: Value::Null,
            is_final: true,
        }
    }

    /// 创建取消 chunk。
    pub fn cancelled(stream_id: &str, sequence: u64, reason: impl Into<String>) -> Self {
        Self {
            stream_id: stream_id.to_string(),
            sequence,
            kind: StreamChunkKind::Cancelled,
            data: json!({ "reason": reason.into() }),
            is_final: true,
        }
    }

    /// 是否为结束 chunk
    pub fn is_done(&self) -> bool {
        self.kind == StreamChunkKind::Done
    }

    /// 是否为错误 chunk。
    pub fn is_error(&self) -> bool {
        self.kind == StreamChunkKind::Error
    }

    /// 是否为取消 chunk。
    pub fn is_cancelled(&self) -> bool {
        self.kind == StreamChunkKind::Cancelled
    }

    /// 是否为终态 chunk。
    pub fn is_terminal(&self) -> bool {
        self.is_done() || self.is_error() || self.is_cancelled()
    }

    /// 转成前端 Channel payload。
    ///
    /// 前端统一收到标准 envelope，而不是各业务模块自定义 `{ done: true }`
    /// / `{ error: ... }` 形状。业务数据始终放在 `data` 内。
    pub fn channel_payload(&self) -> Value {
        json!({
            "streamId": self.stream_id,
            "sequence": self.sequence,
            "kind": self.kind,
            "data": self.data,
            "isFinal": self.is_final,
        })
    }
}

// ============================================================================
// 取消令牌
// ============================================================================

/// Stream 取消令牌。
///
/// 由发送端、接收端、UI Channel 出口共享，用于表达“用户主动取消 / 前端 Channel
/// 断开 / 宿主关闭面板”等非错误终止。
#[derive(Debug, Clone, Default)]
pub struct StreamCancelToken {
    cancelled: Arc<AtomicBool>,
    notify: Arc<tokio::sync::Notify>,
}

impl StreamCancelToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    /// 等待取消信号；已经取消时立即返回。
    pub async fn wait_cancelled(&self) {
        if self.is_cancelled() {
            return;
        }
        let notified = self.notify.notified();
        if self.is_cancelled() {
            return;
        }
        notified.await;
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

// ============================================================================
// 流式错误
// ============================================================================

/// 流式处理错误
#[derive(Debug, Clone, thiserror::Error)]
pub enum StreamError {
    /// 通道已关闭
    #[error("Stream channel closed")]
    ChannelClosed,
    /// 超时
    #[error("Stream timeout")]
    Timeout,
    /// 流未找到
    #[error("Stream not found: {0}")]
    NotFound(String),
    /// 流已取消
    #[error("Stream cancelled")]
    Cancelled,
    /// 流已经发送终态 chunk，不能继续发送数据。
    #[error("Stream already terminated")]
    Terminated,
    /// 流在没有终态 chunk 的情况下提前结束。
    #[error("Stream ended before terminal chunk")]
    Incomplete,
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ======================================================================
    // StreamSource 测试
    // ======================================================================

    #[test]
    fn test_stream_source_new() {
        let source = StreamSource::new("terminal", "term_001");
        assert_eq!(source.kind, "terminal");
        assert_eq!(source.id, "term_001");
        assert!(source.metadata.is_empty());
    }

    #[test]
    fn test_stream_source_with_meta() {
        let source = StreamSource::new("agent", "sess_001")
            .with_meta("session_id", "abc-123")
            .with_meta("model", "claude-opus");

        assert_eq!(source.meta("session_id"), Some("abc-123"));
        assert_eq!(source.meta("model"), Some("claude-opus"));
        assert_eq!(source.meta("nonexistent"), None);
    }

    #[test]
    fn test_stream_source_is_kind() {
        let source = StreamSource::new("terminal", "term_001");
        assert!(source.is_kind("terminal"));
        assert!(!source.is_kind("agent"));
    }

    #[test]
    fn test_stream_source_display() {
        let source = StreamSource::new("terminal", "term_001");
        assert_eq!(source.to_string(), "terminal:term_001");
    }

    #[test]
    fn test_stream_source_serialization() {
        let source = StreamSource::new("agent", "sess_001").with_meta("model", "claude-opus");

        let json_str = serde_json::to_string(&source).unwrap();
        let deserialized: StreamSource = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.kind, "agent");
        assert_eq!(deserialized.id, "sess_001");
        assert_eq!(deserialized.meta("model"), Some("claude-opus"));
    }

    // ======================================================================
    // stream_kind 常量测试
    // ======================================================================

    #[test]
    fn test_stream_kind_constants() {
        assert_eq!(stream_kind::TERMINAL, "terminal");
        assert_eq!(stream_kind::AGENT, "agent");
        assert_eq!(stream_kind::EXTENSION, "extension");
        assert_eq!(stream_kind::GATEWAY, "gateway");
    }

    // ======================================================================
    // StreamChunk 测试
    // ======================================================================

    #[test]
    fn test_stream_chunk_data() {
        let chunk = StreamChunk::data("stream_1", 0, json!({"delta": "Hello"}));
        assert_eq!(chunk.stream_id, "stream_1");
        assert_eq!(chunk.sequence, 0);
        assert_eq!(chunk.data, json!({"delta": "Hello"}));
        assert!(!chunk.is_done());
    }

    #[test]
    fn test_stream_chunk_final() {
        let chunk = StreamChunk::final_chunk("stream_1", 5);
        assert_eq!(chunk.stream_id, "stream_1");
        assert_eq!(chunk.sequence, 5);
        assert!(chunk.data.is_null());
        assert!(chunk.is_done());
    }

    #[test]
    fn test_stream_chunk_serialization() {
        let chunk = StreamChunk::data("stream_1", 0, json!({"delta": "test"}));
        let json_str = serde_json::to_string(&chunk).unwrap();
        let deserialized: StreamChunk = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.stream_id, "stream_1");
        assert_eq!(deserialized.sequence, 0);
        assert!(!deserialized.is_done());
    }

    // ======================================================================
    // StreamError 测试
    // ======================================================================

    #[test]
    fn test_stream_error_display() {
        assert_eq!(
            StreamError::ChannelClosed.to_string(),
            "Stream channel closed"
        );
        assert_eq!(StreamError::Timeout.to_string(), "Stream timeout");
        assert_eq!(
            StreamError::NotFound("test_stream".to_string()).to_string(),
            "Stream not found: test_stream"
        );
        assert_eq!(
            StreamError::Terminated.to_string(),
            "Stream already terminated"
        );
        assert_eq!(
            StreamError::Incomplete.to_string(),
            "Stream ended before terminal chunk"
        );
    }
    #[tokio::test]
    async fn test_stream_cancel_token_wait_is_woken_by_cancel() {
        let token = StreamCancelToken::new();
        let waiter = token.clone();
        let handle = tokio::spawn(async move {
            waiter.wait_cancelled().await;
            waiter.is_cancelled()
        });

        token.cancel();

        assert!(handle.await.unwrap());
    }
}
