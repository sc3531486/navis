//! 后端进程内流式发送/接收（基于 tokio mpsc）
//!
//! 用于同进程内的流式数据传输，如 Gateway → Agent。
//! 不经过 IPC，零序列化开销。
//!
//! # 核心类型
//! - `StreamSender` - 流式发送端
//! - `StreamReceiver` - 流式接收端
//! - `mpsc_pair()` - 创建 (Sender, Receiver) 对的便捷函数

use serde_json::Value;
use tokio::sync::mpsc;

use super::types::{StreamCancelToken, StreamChunk, StreamError, StreamId};

// ============================================================================
// 流式发送端
// ============================================================================

/// 流式发送端（后端→后端）
///
/// 向接收端发送流式 chunk，由 Provider / Gateway 内部使用。
/// 通过 tokio mpsc channel 传输 `StreamChunk`。
pub struct StreamSender {
    /// 发送通道
    tx: mpsc::Sender<StreamChunk>,
    /// 流 ID
    stream_id: StreamId,
    /// 已发送的 chunk 序列号
    sequence: u64,
    /// 取消令牌
    cancel_token: StreamCancelToken,
    /// 是否已经发送终态 chunk
    terminal: bool,
}

impl StreamSender {
    /// 创建新的流式发送器
    pub fn new(tx: mpsc::Sender<StreamChunk>, stream_id: impl Into<String>) -> Self {
        Self::with_cancel_token(tx, stream_id, StreamCancelToken::new())
    }

    /// 使用指定取消令牌创建新的流式发送器。
    pub fn with_cancel_token(
        tx: mpsc::Sender<StreamChunk>,
        stream_id: impl Into<String>,
        cancel_token: StreamCancelToken,
    ) -> Self {
        Self {
            tx,
            stream_id: stream_id.into(),
            sequence: 0,
            cancel_token,
            terminal: false,
        }
    }

    fn ensure_open(&self) -> Result<(), StreamError> {
        if self.terminal {
            return Err(StreamError::Terminated);
        }
        if self.cancel_token.is_cancelled() {
            return Err(StreamError::Cancelled);
        }
        Ok(())
    }

    /// 发送数据
    pub async fn send(&mut self, data: Value) -> Result<(), StreamError> {
        self.ensure_open()?;
        let chunk = StreamChunk::data(&self.stream_id, self.sequence, data);
        self.sequence += 1;
        self.tx
            .send(chunk)
            .await
            .map_err(|_| StreamError::ChannelClosed)
    }

    /// 发送文本增量（便捷方法）
    ///
    /// 将文本包装为 `{"delta": "..."}` 格式发送。
    pub async fn send_delta(&mut self, delta: impl Into<String>) -> Result<(), StreamError> {
        self.send(serde_json::json!({"delta": delta.into()})).await
    }

    /// 发送结束信号
    pub async fn send_done(&mut self) -> Result<(), StreamError> {
        self.ensure_open()?;
        let chunk = StreamChunk::final_chunk(&self.stream_id, self.sequence);
        self.sequence += 1;
        self.tx
            .send(chunk)
            .await
            .map_err(|_| StreamError::ChannelClosed)?;
        self.terminal = true;
        Ok(())
    }

    /// 发送错误并结束流。
    pub async fn send_error(&mut self, message: impl Into<String>) -> Result<(), StreamError> {
        self.ensure_open()?;
        let chunk = StreamChunk::error(&self.stream_id, self.sequence, message);
        self.sequence += 1;
        self.tx
            .send(chunk)
            .await
            .map_err(|_| StreamError::ChannelClosed)?;
        self.terminal = true;
        Ok(())
    }

    /// 发送取消并结束流。
    pub async fn send_cancelled(&mut self, reason: impl Into<String>) -> Result<(), StreamError> {
        if self.terminal {
            return Err(StreamError::Terminated);
        }
        self.cancel_token.cancel();
        let chunk = StreamChunk::cancelled(&self.stream_id, self.sequence, reason);
        self.sequence += 1;
        self.tx
            .send(chunk)
            .await
            .map_err(|_| StreamError::ChannelClosed)?;
        self.terminal = true;
        Ok(())
    }

    /// 主动取消流。
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// 是否已取消。
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    /// 获取共享取消令牌。
    pub fn cancel_token(&self) -> StreamCancelToken {
        self.cancel_token.clone()
    }

    /// 获取流 ID
    pub fn stream_id(&self) -> &str {
        &self.stream_id
    }

    /// 获取已发送的 chunk 数量
    pub fn sent_count(&self) -> u64 {
        self.sequence
    }
}

// ============================================================================
// 流式接收端
// ============================================================================

/// 流式接收端（后端→后端）
///
/// 从发送端接收流式 chunk，由 Gateway 返回给调用方。
/// 通过 tokio mpsc channel 接收 `StreamChunk`。
pub struct StreamReceiver {
    /// 接收通道
    rx: mpsc::Receiver<StreamChunk>,
    /// 流 ID
    stream_id: StreamId,
    /// 取消令牌
    cancel_token: StreamCancelToken,
    /// 是否已经收到终态 chunk
    terminal_seen: bool,
}

impl StreamReceiver {
    /// 创建新的流式接收器
    pub fn new(rx: mpsc::Receiver<StreamChunk>, stream_id: impl Into<String>) -> Self {
        Self::with_cancel_token(rx, stream_id, StreamCancelToken::new())
    }

    /// 使用指定取消令牌创建新的流式接收器。
    pub fn with_cancel_token(
        rx: mpsc::Receiver<StreamChunk>,
        stream_id: impl Into<String>,
        cancel_token: StreamCancelToken,
    ) -> Self {
        Self {
            rx,
            stream_id: stream_id.into(),
            cancel_token,
            terminal_seen: false,
        }
    }

    /// 接收下一个 chunk
    pub async fn recv(&mut self) -> Option<StreamChunk> {
        let chunk = self.rx.recv().await?;
        if chunk.is_terminal() {
            self.terminal_seen = true;
        }
        Some(chunk)
    }

    /// 是否已经收到终态 chunk。
    pub fn is_terminal(&self) -> bool {
        self.terminal_seen
    }

    /// 主动取消流。
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// 是否已取消。
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    /// 获取共享取消令牌。
    pub fn cancel_token(&self) -> StreamCancelToken {
        self.cancel_token.clone()
    }

    /// 收集所有 delta 文本
    ///
    /// 用于测试或非流式场景，收集所有增量文本到一个字符串。
    /// 自动从 `{"delta": "..."}` 格式中提取文本。
    pub async fn collect_text(&mut self) -> String {
        let mut text = String::new();
        while let Some(chunk) = self.recv().await {
            if chunk.is_done() || chunk.is_error() {
                break;
            }
            if let Some(delta) = chunk.data.get("delta").and_then(|v| v.as_str()) {
                text.push_str(delta);
            }
        }
        text
    }

    /// 获取流 ID
    pub fn stream_id(&self) -> &str {
        &self.stream_id
    }
}
// ============================================================================
// 通道创建
// ============================================================================

/// 创建后端进程内流式通道对
///
/// 使用 UUID 自动生成 stream_id，格式为 `{kind}:{uuid}`。
///
/// # Arguments
/// * `kind` - 流类型标识（如 "gateway"、"agent"）
/// * `buffer` - 通道缓冲区大小
///
/// # Returns
/// (StreamSender, StreamReceiver) 对
pub fn mpsc_pair(kind: &str, buffer: usize) -> (StreamSender, StreamReceiver) {
    let stream_id = format!("{}:{}", kind, uuid::Uuid::new_v4());
    let (tx, rx) = mpsc::channel(buffer);
    let cancel_token = StreamCancelToken::new();
    tracing::debug!(stream_id = %stream_id, buffer = buffer, "Stream mpsc pair created");
    (
        StreamSender::with_cancel_token(tx, &stream_id, cancel_token.clone()),
        StreamReceiver::with_cancel_token(rx, &stream_id, cancel_token),
    )
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_sender_send_delta() {
        let (mut sender, mut receiver) = mpsc_pair("test", 16);

        sender.send_delta("Hello").await.unwrap();
        sender.send_delta(" World").await.unwrap();

        assert_eq!(sender.sent_count(), 2);

        let chunk1 = receiver.recv().await.unwrap();
        assert_eq!(chunk1.data, json!({"delta": "Hello"}));
        assert!(!chunk1.is_done());

        let chunk2 = receiver.recv().await.unwrap();
        assert_eq!(chunk2.data, json!({"delta": " World"}));
    }

    #[tokio::test]
    async fn test_sender_send_done() {
        let (mut sender, mut receiver) = mpsc_pair("test", 16);

        sender.send_done().await.unwrap();

        let chunk = receiver.recv().await.unwrap();
        assert!(chunk.is_done());
    }

    #[tokio::test]
    async fn test_sender_rejects_data_after_terminal_chunk() {
        let (mut sender, mut receiver) = mpsc_pair("test", 16);

        sender.send_done().await.unwrap();
        assert!(matches!(
            sender.send_delta("late data").await,
            Err(StreamError::Terminated)
        ));

        assert!(receiver.recv().await.unwrap().is_done());
        assert!(receiver.rx.try_recv().is_err());
    }
    #[tokio::test]
    async fn test_sender_channel_closed() {
        let (tx, rx) = mpsc::channel(16);
        let mut sender = StreamSender::new(tx, "stream_1");
        drop(rx); // 关闭接收端

        let result = sender.send(json!("test")).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StreamError::ChannelClosed));
    }

    #[tokio::test]
    async fn test_sender_send_value() {
        let (mut sender, mut receiver) = mpsc_pair("test", 16);

        sender.send(json!({"key": "value"})).await.unwrap();
        sender.send_done().await.unwrap();

        let chunk = receiver.recv().await.unwrap();
        assert_eq!(chunk.data, json!({"key": "value"}));
        assert!(!chunk.is_done());

        let done = receiver.recv().await.unwrap();
        assert!(done.is_done());
    }

    #[tokio::test]
    async fn test_receiver_collect_text() {
        let (mut sender, mut receiver) = mpsc_pair("test", 16);

        tokio::spawn(async move {
            sender.send_delta("你好").await.unwrap();
            sender.send_delta("世界").await.unwrap();
            sender.send_done().await.unwrap();
        });

        let text = receiver.collect_text().await;
        assert_eq!(text, "你好世界");
    }

    #[tokio::test]
    async fn test_receiver_stream_id() {
        let (sender, receiver) = mpsc_pair("test", 16);
        assert_eq!(sender.stream_id(), receiver.stream_id());
    }

    #[tokio::test]
    async fn test_create_stream_basic() {
        let (mut sender, mut receiver) = mpsc_pair("stream_1", 16);

        sender.send_delta("Hello").await.unwrap();
        sender.send_delta(" World").await.unwrap();
        sender.send_done().await.unwrap();

        let chunk1 = receiver.recv().await.unwrap();
        assert_eq!(chunk1.data, json!({"delta": "Hello"}));
        assert!(!chunk1.is_done());

        let chunk2 = receiver.recv().await.unwrap();
        assert_eq!(chunk2.data, json!({"delta": " World"}));

        let chunk3 = receiver.recv().await.unwrap();
        assert!(chunk3.is_done());

        // 关闭发送端后，接收端应返回 None
        drop(sender);
        assert!(receiver.recv().await.is_none());
    }

    #[tokio::test]
    async fn test_stream_empty() {
        let (mut sender, mut receiver) = mpsc_pair("stream_1", 16);

        // 直接关闭
        sender.send_done().await.unwrap();
        drop(sender);

        let chunk = receiver.recv().await.unwrap();
        assert!(chunk.is_done());

        let text = receiver.collect_text().await;
        assert_eq!(text, "");
    }

    #[tokio::test]
    async fn test_stream_buffer_full() {
        let (mut sender, mut receiver) = mpsc_pair("stream_1", 2);

        let handle = tokio::spawn(async move {
            for i in 0..5 {
                sender.send_delta(format!("chunk_{}", i)).await.unwrap();
            }
            sender.send_done().await.unwrap();
        });

        let mut chunks = Vec::new();
        while let Some(chunk) = receiver.recv().await {
            chunks.push(chunk);
        }
        assert_eq!(chunks.len(), 6); // 5 deltas + 1 done

        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_mpsc_pair_id_format() {
        let (sender, receiver) = mpsc_pair("gateway", 16);

        // stream_id 格式应为 "gateway:{uuid}"
        assert!(sender.stream_id().starts_with("gateway:"));
        assert!(receiver.stream_id().starts_with("gateway:"));
        assert_eq!(sender.stream_id(), receiver.stream_id());
    }

    #[tokio::test]
    async fn test_sender_sequence_monotonic() {
        let (mut sender, mut receiver) = mpsc_pair("test", 16);

        sender.send_delta("a").await.unwrap();
        sender.send_delta("b").await.unwrap();
        sender.send_delta("c").await.unwrap();

        let c1 = receiver.recv().await.unwrap();
        let c2 = receiver.recv().await.unwrap();
        let c3 = receiver.recv().await.unwrap();

        assert_eq!(c1.sequence, 0);
        assert_eq!(c2.sequence, 1);
        assert_eq!(c3.sequence, 2);
    }
}
