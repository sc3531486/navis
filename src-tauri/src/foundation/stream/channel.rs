//! 跨进程流式通道（基于 Tauri Channel + ThrottledEmitter）
//!
//! 用于后端→前端的流式数据推送。
//! 通过 Builder 模式创建，支持节流、来源标识、标签。
//!
//! # 使用场景
//! - Agent/Terminal → 前端的流式数据推送
//! - 扩展自定义流（如 Jira 实时更新）
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde_json::Value;
use tauri::ipc::{Channel, InvokeResponseBody};

use super::emitter::ThrottledEmitter;
use super::index::StreamIndex;
use super::sender::StreamReceiver;
use super::types::{StreamCancelToken, StreamChunk, StreamError, StreamId, StreamSource};

// ============================================================================
// 跨进程流式通道
// ============================================================================

/// 跨进程流式通道
///
/// 封装 Tauri Channel + ThrottledEmitter，提供带节流的前端推送。
/// 支持来源标识、标签和流 ID，便于调试和监控。
///
pub struct StreamChannel {
    /// 节流发射器。StreamChannel 会被终端后台任务跨线程共享。
    emitter: Mutex<ThrottledEmitter>,
    /// 流来源标识
    source: StreamSource,
    /// 标签（用于调试和展示）
    label: String,
    /// 流 ID
    stream_id: StreamId,
    /// 已发送的数据块数
    sent_count: AtomicU64,
    /// 可选流索引
    index: Option<StreamIndex>,
    /// 取消令牌
    cancel_token: StreamCancelToken,
}

impl StreamChannel {
    /// 创建 Builder
    pub fn builder(channel: Channel) -> StreamChannelBuilder {
        StreamChannelBuilder {
            channel,
            source: StreamSource::new("unknown", "unknown"),
            label: String::new(),
            throttle_interval: Duration::from_millis(50),
            index: None,
        }
    }

    /// 推送数据（通过 ThrottledEmitter 节流后发送）
    ///
    /// 使用内部可变性，允许通过 `&self` 调用。
    pub fn push(&self, data: &str, meta: Option<Value>) {
        if self.cancel_token.is_cancelled() {
            return;
        }
        self.emitter
            .lock()
            .expect("stream emitter mutex poisoned")
            .push(data, meta);
        self.sent_count.fetch_add(1, Ordering::Relaxed);
        if let Some(index) = &self.index {
            index.record_send(&self.stream_id);
        }
    }

    /// 简化推送（无元数据）
    pub fn push_text(&self, data: &str) {
        self.push(data, None);
    }

    /// 显式刷新缓冲区
    pub fn flush(&self) {
        if self.cancel_token.is_cancelled() {
            return;
        }
        self.emitter
            .lock()
            .expect("stream emitter mutex poisoned")
            .flush();
    }

    /// 发送结束信号
    pub fn done(&self) {
        if self.cancel_token.is_cancelled() {
            return;
        }
        self.emitter
            .lock()
            .expect("stream emitter mutex poisoned")
            .send_done();
        if let Some(index) = &self.index {
            index.record_send(&self.stream_id);
        }
    }

    /// 取消流。
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// 获取来源
    pub fn source(&self) -> &StreamSource {
        &self.source
    }

    /// 获取标签
    pub fn label(&self) -> &str {
        &self.label
    }

    /// 获取 Stream ID
    pub fn stream_id(&self) -> &str {
        &self.stream_id
    }

    /// 获取已发送的数据块数
    pub fn sent_count(&self) -> u64 {
        self.sent_count.load(Ordering::Relaxed)
    }

    /// 获取缓冲区中的待发送数据量
    pub fn pending_count(&self) -> usize {
        self.emitter
            .lock()
            .expect("stream emitter mutex poisoned")
            .pending_count()
    }

    /// 获取底层 Tauri Channel 引用
    pub fn channel(&self) -> Channel {
        self.emitter
            .lock()
            .expect("stream emitter mutex poisoned")
            .channel()
            .clone()
    }

    /// 设置节流间隔
    pub fn set_interval(&self, interval: Duration) {
        self.emitter
            .lock()
            .expect("stream emitter mutex poisoned")
            .set_interval(interval);
    }
}

impl Drop for StreamChannel {
    fn drop(&mut self) {
        if let Some(index) = &self.index {
            index.untrack(&self.stream_id);
        }
    }
}

// ============================================================================
// Builder
// ============================================================================

/// StreamChannel Builder
///
/// 使用 Builder 模式构建 StreamChannel，支持链式调用配置。
///
/// # 示例
/// ```ignore
/// let channel = StreamChannel::builder(channel)
///     .source(StreamSource::new("agent", "sess_001"))
///     .label("LLM streaming")
///     .throttle(Duration::from_millis(50))
///     .build();
/// ```
pub struct StreamChannelBuilder {
    channel: Channel,
    source: StreamSource,
    label: String,
    throttle_interval: Duration,
    index: Option<StreamIndex>,
}

impl StreamChannelBuilder {
    /// 设置流来源
    pub fn source(mut self, source: StreamSource) -> Self {
        self.source = source;
        self
    }

    /// 设置标签（用于调试和展示）
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// 设置节流间隔
    pub fn throttle(mut self, interval: Duration) -> Self {
        self.throttle_interval = interval;
        self
    }

    /// 绑定流索引。构建时自动跟踪，Drop 时自动移除。
    pub fn index(mut self, index: StreamIndex) -> Self {
        self.index = Some(index);
        self
    }

    /// 构建 StreamChannel
    pub fn build(self) -> StreamChannel {
        let stream_id = format!("{}:{}", self.source, uuid::Uuid::new_v4());
        let cancel_token = StreamCancelToken::new();
        let emitter = ThrottledEmitter::with_stream(
            self.channel.clone(),
            stream_id.clone(),
            self.throttle_interval,
        );
        if let Some(index) = &self.index {
            index.track_with_cancel(&stream_id, self.source.clone(), Some(cancel_token.clone()));
        }

        tracing::info!(
            stream_id = %stream_id,
            source = %self.source,
            label = %self.label,
            throttle_ms = self.throttle_interval.as_millis(),
            "StreamChannel created"
        );

        StreamChannel {
            emitter: Mutex::new(emitter),
            source: self.source,
            label: self.label,
            stream_id,
            sent_count: AtomicU64::new(0),
            index: self.index,
            cancel_token,
        }
    }
}

// ============================================================================
// Receiver -> Tauri Channel outlet
// ============================================================================

/// 发送 JSON payload 到 Tauri Channel。
pub fn send_channel_value(channel: &Channel, value: Value) -> Result<(), StreamError> {
    channel
        .send(InvokeResponseBody::Json(value.to_string()))
        .map_err(|_| StreamError::ChannelClosed)
}

/// 将后端进程内 StreamReceiver 转发到前端 Tauri Channel。
///
/// 业务模块只负责创建内部流；跨进程转发统一走这里，避免各模块重复拼
/// `{ done: true }` / `{ error: ... }` payload。
pub async fn forward_receiver_to_channel(
    mut receiver: StreamReceiver,
    channel: Channel,
) -> Result<(), StreamError> {
    let stream_id = receiver.stream_id().to_string();
    let mut sequence = 0;
    while let Some(chunk) = receiver.recv().await {
        let is_terminal = chunk.is_terminal();
        sequence = chunk.sequence.saturating_add(1);
        if let Err(error) = send_channel_value(&channel, chunk.channel_payload()) {
            receiver.cancel();
            return Err(error);
        }
        if is_terminal {
            return Ok(());
        }
    }

    if receiver.is_cancelled() {
        let chunk = StreamChunk::cancelled(&stream_id, sequence, "Stream cancelled");
        send_channel_value(&channel, chunk.channel_payload())?;
        return Ok(());
    }

    let chunk = StreamChunk::error(&stream_id, sequence, StreamError::Incomplete.to_string());
    send_channel_value(&channel, chunk.channel_payload())?;
    Err(StreamError::Incomplete)
}

/// 将后端进程内 StreamReceiver 转发到前端 Tauri Channel，并绑定 Index 生命周期。
///
/// 索引用于前端取消、活跃流查询与发送计数。转发结束后会自动移除。
pub async fn forward_receiver_to_channel_with_index(
    mut receiver: StreamReceiver,
    channel: Channel,
    index: StreamIndex,
    source: StreamSource,
) -> Result<(), StreamError> {
    let stream_id = receiver.stream_id().to_string();
    let cancel_token = receiver.cancel_token();
    index.track_with_cancel(&stream_id, source, Some(cancel_token));

    let mut sequence = 0;
    let result = async {
        while let Some(chunk) = receiver.recv().await {
            let is_terminal = chunk.is_terminal();
            sequence = chunk.sequence.saturating_add(1);
            if let Err(error) = send_channel_value(&channel, chunk.channel_payload()) {
                receiver.cancel();
                return Err(error);
            }
            index.record_send(&stream_id);
            if is_terminal {
                return Ok(());
            }
        }

        if receiver.is_cancelled() {
            let chunk = StreamChunk::cancelled(&stream_id, sequence, "Stream cancelled");
            send_channel_value(&channel, chunk.channel_payload())?;
            index.record_send(&stream_id);
            return Ok(());
        }

        let chunk = StreamChunk::error(&stream_id, sequence, StreamError::Incomplete.to_string());
        send_channel_value(&channel, chunk.channel_payload())?;
        index.record_send(&stream_id);
        Err(StreamError::Incomplete)
    }
    .await;

    index.untrack(&stream_id);
    result
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_channel() -> StreamChannel {
        let channel = Channel::new(move |_value| Ok(()));
        StreamChannel::builder(channel).build()
    }

    fn create_test_channel_with_interval(interval: Duration) -> StreamChannel {
        let channel = Channel::new(move |_value| Ok(()));
        StreamChannel::builder(channel).throttle(interval).build()
    }

    #[test]
    fn test_stream_channel_builder_defaults() {
        let ch = create_test_channel();
        assert_eq!(ch.sent_count(), 0);
        assert_eq!(ch.pending_count(), 0);
        assert_eq!(ch.source().kind, "unknown");
        assert_eq!(ch.label(), "");
    }

    #[test]
    fn test_stream_channel_builder_with_source() {
        let channel = Channel::new(move |_value| Ok(()));
        let ch = StreamChannel::builder(channel)
            .source(StreamSource::new("terminal", "term_001"))
            .label("终端输出")
            .build();

        assert_eq!(ch.source().kind, "terminal");
        assert_eq!(ch.source().id, "term_001");
        assert_eq!(ch.label(), "终端输出");
        assert!(ch.stream_id().starts_with("terminal:term_001:"));
    }

    #[test]
    fn test_stream_channel_push_text() {
        let ch = create_test_channel();
        ch.push_text("hello");
        assert_eq!(ch.sent_count(), 1);
        assert_eq!(ch.pending_count(), 1);
    }

    #[test]
    fn test_stream_channel_push_with_meta() {
        let ch = create_test_channel();
        ch.push("data1", Some(json!({"stream": "stdout"})));
        ch.push("data2", Some(json!({"stream": "stderr"})));
        assert_eq!(ch.sent_count(), 2);
        assert_eq!(ch.pending_count(), 2);
    }

    #[test]
    fn test_stream_channel_flush() {
        let ch = create_test_channel();
        ch.push_text("data1");
        ch.push_text("data2");
        assert_eq!(ch.pending_count(), 2);

        ch.flush();
        assert_eq!(ch.pending_count(), 0);
    }

    #[test]
    fn test_stream_channel_done() {
        let ch = create_test_channel();
        ch.done();
    }

    #[test]
    fn test_stream_channel_auto_flush() {
        let ch = create_test_channel_with_interval(Duration::from_millis(10));

        ch.push_text("chunk1");
        assert_eq!(ch.pending_count(), 1);

        std::thread::sleep(Duration::from_millis(15));

        ch.push_text("chunk2");
        // 应该已自动 flush
        assert_eq!(ch.pending_count(), 0);
    }

    #[test]
    fn test_stream_channel_set_interval() {
        let ch = create_test_channel();

        ch.set_interval(Duration::from_millis(100));
        ch.push_text("data");
        assert_eq!(ch.pending_count(), 1);

        ch.set_interval(Duration::from_nanos(1));
        ch.flush();
        ch.push_text("data2");
        assert_eq!(ch.pending_count(), 0);
    }

    #[test]
    fn test_stream_channel_source_with_meta() {
        let channel = Channel::new(move |_value| Ok(()));
        let ch = StreamChannel::builder(channel)
            .source(
                StreamSource::new("agent", "sess_001")
                    .with_meta("model", "claude-opus")
                    .with_meta("session_id", "abc-123"),
            )
            .build();

        assert_eq!(ch.source().meta("model"), Some("claude-opus"));
        assert_eq!(ch.source().meta("session_id"), Some("abc-123"));
    }

    #[test]
    fn test_stream_channel_builder_throttle() {
        let channel = Channel::new(move |_value| Ok(()));
        let ch = StreamChannel::builder(channel)
            .throttle(Duration::from_millis(100))
            .build();

        // 设置较长节流间隔，push 不会自动 flush
        ch.push_text("data");
        assert_eq!(ch.pending_count(), 1);
    }

    #[test]
    fn test_stream_channel_refcell_safety() {
        // 测试 RefCell 内部可变性：通过 &self 调用 push/flush
        let ch = create_test_channel();

        ch.push_text("data1");
        ch.push_text("data2");
        ch.flush();
        ch.push_text("data3");

        assert_eq!(ch.sent_count(), 3);
        assert_eq!(ch.pending_count(), 1);
    }
}
