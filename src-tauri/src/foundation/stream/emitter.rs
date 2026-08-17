//! 节流发射器 - 基于 Tauri Channel 的高频输出合并组件
//!
//! 将短时间窗口内的多个输出 chunk 合并为单次 `Channel<Value>::send()` 调用。
//! 适用于：终端输出、编译日志、测试输出等高频场景。
//!
//! 发射器直接持有 `tauri::ipc::Channel<Value>`，数据直推前端。

use serde_json::{json, Value};
use std::cell::Cell;
use std::time::{Duration, Instant};
use tauri::ipc::{Channel, InvokeResponseBody};

use super::types::StreamChunk;

/// 默认节流间隔（50ms）
pub const DEFAULT_THROTTLE_INTERVAL: Duration = Duration::from_millis(50);

/// 缓冲区中的输出块
pub struct BufferedItem {
    /// 合并用的文本数据
    pub text: String,
    /// 附加元数据（如 stdout/stderr 标记）
    pub meta: Option<Value>,
}

/// 节流发射器
///
/// 将高频输出合并后通过 Tauri Channel 直推前端，减少跨进程 IPC 调用频率。
/// 持有 `Channel<Value>`（非 Arc 包装），生命周期由 Tauri 框架管理。
pub struct ThrottledEmitter {
    /// 节流间隔
    interval: Duration,
    /// 缓冲区
    buffer: Vec<BufferedItem>,
    /// 上次刷新时间
    last_flush: Instant,
    /// Tauri Channel（数据直推前端）
    channel: Channel,
    /// 流 ID
    stream_id: String,
    /// 已发送 chunk 序列号
    sequence: Cell<u64>,
}

impl ThrottledEmitter {
    /// 创建新的节流发射器（使用默认 50ms 间隔）
    pub fn new(channel: Channel) -> Self {
        Self::with_interval(channel, DEFAULT_THROTTLE_INTERVAL)
    }

    /// 创建指定间隔的节流发射器
    pub fn with_interval(channel: Channel, interval: Duration) -> Self {
        Self::with_stream(channel, "stream", interval)
    }

    /// 创建绑定 stream_id 的节流发射器。
    pub fn with_stream(channel: Channel, stream_id: impl Into<String>, interval: Duration) -> Self {
        Self {
            interval,
            buffer: Vec::new(),
            last_flush: Instant::now(),
            channel,
            stream_id: stream_id.into(),
            sequence: Cell::new(0),
        }
    }

    /// 推送数据到缓冲区
    ///
    /// 如果距上次 flush 超过节流间隔，自动 flush。
    pub fn push(&mut self, text: &str, meta: Option<Value>) {
        self.buffer.push(BufferedItem {
            text: text.to_string(),
            meta,
        });

        if self.last_flush.elapsed() >= self.interval {
            self.flush();
        }
    }

    /// 简化推送（无元数据）
    pub fn push_text(&mut self, text: &str) {
        self.push(text, None);
    }

    /// 刷新缓冲区，合并所有缓冲数据后通过 Channel 发送到前端
    pub fn flush(&mut self) {
        if self.buffer.is_empty() {
            return;
        }

        let combined_text: String = self.buffer.iter().map(|item| item.text.as_str()).collect();
        let metas: Vec<Value> = self.buffer.drain(..).filter_map(|item| item.meta).collect();

        let mut payload = json!({ "data": combined_text });

        // 收集到的 meta 放入数组
        if !metas.is_empty() {
            payload["meta"] = json!(metas);
        }

        let sequence = self.sequence.get();
        self.sequence.set(sequence + 1);
        let chunk = StreamChunk::data(&self.stream_id, sequence, payload);
        let body =
            InvokeResponseBody::Json(serde_json::to_string(&chunk.channel_payload()).unwrap());
        let _ = self.channel.send(body);
        self.last_flush = Instant::now();

        tracing::trace!(
            combined_len = combined_text.len(),
            "ThrottledEmitter flushed via Tauri Channel"
        );
    }

    /// 获取缓冲区中的待发送数据量
    pub fn pending_count(&self) -> usize {
        self.buffer.len()
    }

    /// 发送最终 chunk（流结束信号）
    ///
    /// 通知前端该流已完成。之后 Channel 可安全 Drop。
    pub fn send_done(&self) {
        let sequence = self.sequence.get();
        self.sequence.set(sequence + 1);
        let chunk = StreamChunk::final_chunk(&self.stream_id, sequence);
        let body =
            InvokeResponseBody::Json(serde_json::to_string(&chunk.channel_payload()).unwrap());
        let _ = self.channel.send(body);
    }

    /// 获取底层 Channel 引用（用于需要直接 send 的场景）
    pub fn channel(&self) -> &Channel {
        &self.channel
    }

    /// 设置节流间隔
    ///
    /// 允许在创建后调整节流间隔（例如在测试中使用更短的间隔）。
    pub fn set_interval(&mut self, interval: Duration) {
        self.interval = interval;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_emitter() -> ThrottledEmitter {
        let channel = Channel::new(move |_value| Ok(()));
        ThrottledEmitter::new(channel)
    }

    fn create_test_emitter_with_interval(interval: Duration) -> ThrottledEmitter {
        let channel = Channel::new(move |_value| Ok(()));
        ThrottledEmitter::with_interval(channel, interval)
    }

    #[test]
    fn test_throttled_emitter_push_text() {
        let mut emitter = create_test_emitter();

        emitter.push_text("buffered data");
        assert_eq!(emitter.pending_count(), 1);

        emitter.flush();
        assert_eq!(emitter.pending_count(), 0);
    }

    #[test]
    fn test_throttled_emitter_push_with_meta() {
        let mut emitter = create_test_emitter();

        emitter.push("hello", Some(json!({"stream": "stdout"})));
        emitter.push(" world", Some(json!({"stream": "stderr"})));
        assert_eq!(emitter.pending_count(), 2);

        emitter.flush();
        assert_eq!(emitter.pending_count(), 0);
    }

    #[test]
    fn test_throttled_emitter_flush_on_interval() {
        let mut emitter = create_test_emitter_with_interval(Duration::from_millis(10));

        emitter.push_text("chunk1");
        assert_eq!(emitter.pending_count(), 1);

        std::thread::sleep(Duration::from_millis(15));

        emitter.push_text("chunk2");

        assert_eq!(emitter.pending_count(), 0);
    }

    #[test]
    fn test_throttled_emitter_combined_chunks() {
        let mut emitter = create_test_emitter();

        emitter.push_text("Hello");
        emitter.push_text(", ");
        emitter.push_text("World!");

        assert_eq!(emitter.pending_count(), 3);

        emitter.flush();
        assert_eq!(emitter.pending_count(), 0);
    }

    #[test]
    fn test_throttled_emitter_flush_empty() {
        let mut emitter = create_test_emitter();

        emitter.flush();
        assert_eq!(emitter.pending_count(), 0);
    }

    #[test]
    fn test_throttled_emitter_send_done() {
        let emitter = create_test_emitter();
        emitter.send_done();
    }

    #[test]
    fn test_throttled_emitter_channel_access() {
        let emitter = create_test_emitter();
        let _channel = emitter.channel();
    }

    #[test]
    fn test_throttled_emitter_set_interval() {
        let mut emitter = create_test_emitter();

        emitter.set_interval(Duration::from_millis(100));
        emitter.push_text("data");
        assert_eq!(emitter.pending_count(), 1);

        emitter.set_interval(Duration::from_nanos(1));
        emitter.flush();
        emitter.push_text("data2");
        assert_eq!(emitter.pending_count(), 0);
    }
}
