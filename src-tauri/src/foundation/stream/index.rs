//! 流活动索引 - 管理活跃流
//!
//! 提供全局视角的流管理：哪些流在活跃、属于哪个模块/会话。
//! 支持按来源类型、会话 ID 查询。
//! 主要用于调试、监控和取消令牌查找，不影响正常数据流。
//!
//! # 数据广播（扩展订阅）
//!
//! 自设计 34（§2.8 / §4.3）与 02b-stream（§3.8）落地以来，StreamIndex 同时承载
//! `extension.stream.subscribeSource` 的按需订阅广播：发布点（如 Agent 流推送）
//! 调用 [`StreamIndex::publish`] 按 kind/session 过滤并把数据投递给订阅者；
//! 无订阅者时 publish 零开销（推模型，从源头消灭无效投递）。
//!
//! 订阅者用 tokio broadcast channel 消费，publish 使用 `try_send`（背压走
//! 丢弃 + 计数，不阻塞发布方，保实时性，见 34 §2.8 禁止节流/缓存堆积）。
//!
//! 这里不是 Kernel Index：不声明 Capability、不承载生命周期事实源，
//! 也不作为 EventBus/Pipeline 的替代品。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde_json::Value;
use tokio::sync::broadcast;

use super::types::{StreamCancelToken, StreamSource};

// ============================================================================
// 流信息
// ============================================================================

/// 流信息
#[derive(Debug, Clone)]
pub struct StreamInfo {
    /// 流 ID
    pub stream_id: String,
    /// 流来源标识
    pub source: StreamSource,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 已发送的 chunk 数
    pub chunk_count: u64,
    /// 取消令牌
    pub cancel_token: Option<StreamCancelToken>,
}

// ============================================================================
// 订阅过滤器
// ============================================================================

/// 流订阅过滤器（`extension.stream.subscribeSource` 的 filter）。
///
/// 按 `kind`（必填）与可选 `session_id` 匹配流。`session_id` 为 `None` 时
/// 匹配该 kind 下的任意会话流。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamSubscriptionFilter {
    /// 流类型（如 "agent" / "terminal"）
    pub kind: String,
    /// 可选会话 ID（流 source 的 `session_id` metadata）
    pub session_id: Option<String>,
}

impl StreamSubscriptionFilter {
    /// 创建只按 kind 过滤的订阅。
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            session_id: None,
        }
    }

    /// 追加会话 ID 约束。
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// 判断是否匹配给定 kind + 可选 session_id（发布侧视角）。
    pub fn matches(&self, kind: &str, session_id: Option<&str>) -> bool {
        if self.kind != kind {
            return false;
        }
        match (&self.session_id, session_id) {
            (Some(expected), Some(actual)) => expected == actual,
            (Some(_), None) => false,
            (None, _) => true,
        }
    }

    /// 判断是否匹配给定活跃流（查询侧视角）。
    pub fn matches_stream(&self, info: &StreamInfo) -> bool {
        self.matches(&info.source.kind, info.source.meta("session_id"))
    }
}

// ============================================================================
// 广播订阅者
// ============================================================================

/// 活跃订阅者。
///
/// `tx` 为 tokio broadcast sender，发布方 `publish` 时按 filter 匹配后非阻塞投递。
#[derive(Debug)]
struct ActiveSubscription {
    /// 订阅来源流 ID（`subscribe_for_stream` 写入，供按流取消）
    stream_id: Option<String>,
    /// 订阅过滤器
    filter: StreamSubscriptionFilter,
    /// 广播发送端
    tx: broadcast::Sender<Value>,
}

// ============================================================================
// 流索引
// ============================================================================

/// 流活动索引（线程安全）
///
/// 线程安全的活跃流管理器，用于调试、监控和取消令牌查找。
/// 通过 `Arc<RwLock<HashMap>>` 实现共享状态。
///
/// 除活跃流元数据外，还登记 broadcast 订阅者（[`ActiveSubscription`]），
/// 支持按 kind/session 过滤的数据广播（见 34 §2.8 按需订阅）。
#[derive(Debug, Clone)]
pub struct StreamIndex {
    streams: Arc<RwLock<HashMap<String, StreamInfo>>>,
    subscriptions: Arc<RwLock<HashMap<u64, ActiveSubscription>>>,
    next_subscription_id: Arc<AtomicU64>,
}

impl StreamIndex {
    /// 创建新的流活动索引
    pub fn new() -> Self {
        Self {
            streams: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            next_subscription_id: Arc::new(AtomicU64::new(0)),
        }
    }

    /// 跟踪一条新的活动流。
    pub fn track(&self, stream_id: &str, source: StreamSource) {
        self.track_with_cancel(stream_id, source, None);
    }

    /// 跟踪一条新的活动流并绑定取消令牌。
    pub fn track_with_cancel(
        &self,
        stream_id: &str,
        source: StreamSource,
        cancel_token: Option<StreamCancelToken>,
    ) {
        let info = StreamInfo {
            stream_id: stream_id.to_string(),
            source: source.clone(),
            created_at: Utc::now(),
            chunk_count: 0,
            cancel_token,
        };

        self.streams
            .write()
            .unwrap()
            .insert(stream_id.to_string(), info);

        tracing::debug!(
            stream_id = %stream_id,
            source = %source,
            "流已加入 StreamIndex 活动索引"
        );
    }

    /// 从活动索引移除流。
    ///
    /// 流结束或关闭时调用。
    pub fn untrack(&self, stream_id: &str) {
        let removed = self.streams.write().unwrap().remove(stream_id);

        if removed.is_some() {
            tracing::debug!(
                stream_id = %stream_id,
                "流已从 StreamIndex 活动索引移除"
            );
        }
    }

    /// 记录发送（递增 chunk_count）
    ///
    /// 每次通过 Channel 发送数据时调用，用于统计。
    pub fn record_send(&self, stream_id: &str) {
        if let Some(info) = self.streams.write().unwrap().get_mut(stream_id) {
            info.chunk_count += 1;
        }
    }

    /// 取消指定流。
    pub fn cancel(&self, stream_id: &str) -> bool {
        self.cancel_info(stream_id).is_some()
    }

    /// 取消指定流并返回取消时的活动流快照。
    pub fn cancel_info(&self, stream_id: &str) -> Option<StreamInfo> {
        let info = self.streams.read().unwrap().get(stream_id).cloned();
        if let Some(info) = &info {
            if let Some(token) = &info.cancel_token {
                token.cancel();
            }
            tracing::debug!(stream_id = %stream_id, "StreamIndex 已取消流");
        }
        info
    }

    /// 按来源类型查询
    pub fn list_by_kind(&self, kind: &str) -> Vec<StreamInfo> {
        self.streams
            .read()
            .unwrap()
            .values()
            .filter(|i| i.source.is_kind(kind))
            .cloned()
            .collect()
    }

    /// 按会话 ID 查询（从 metadata 中读取 session_id）
    pub fn list_by_session(&self, session_id: &str) -> Vec<StreamInfo> {
        self.streams
            .read()
            .unwrap()
            .values()
            .filter(|i| i.source.meta("session_id") == Some(session_id))
            .cloned()
            .collect()
    }

    /// 获取所有活跃流
    pub fn list_all(&self) -> Vec<StreamInfo> {
        self.streams.read().unwrap().values().cloned().collect()
    }

    /// 活跃流数量
    pub fn count(&self) -> usize {
        self.streams.read().unwrap().len()
    }

    /// 判断指定流是否仍在索引中。
    pub fn contains(&self, stream_id: &str) -> bool {
        self.streams.read().unwrap().contains_key(stream_id)
    }

    /// 登记一个数据订阅者，返回订阅 ID。
    ///
    /// 订阅者按 [`StreamSubscriptionFilter`] 匹配后续 `publish` 的流数据。
    pub fn subscribe(
        &self,
        filter: StreamSubscriptionFilter,
        tx: broadcast::Sender<Value>,
    ) -> u64 {
        self.subscribe_internal(None, filter, tx)
    }

    /// 登记一个绑定来源流 ID 的数据订阅者（`subscribeSource` 落地）。
    ///
    /// 与 [`StreamIndex::subscribe`] 语义一致，额外记录 `stream_id`，
    /// 使 [`StreamIndex::unsubscribe_by_stream`] 可按流取消。
    pub fn subscribe_for_stream(
        &self,
        stream_id: &str,
        filter: StreamSubscriptionFilter,
        tx: broadcast::Sender<Value>,
    ) -> u64 {
        self.subscribe_internal(Some(stream_id.to_string()), filter, tx)
    }

    fn subscribe_internal(
        &self,
        stream_id: Option<String>,
        filter: StreamSubscriptionFilter,
        tx: broadcast::Sender<Value>,
    ) -> u64 {
        let id = self.next_subscription_id.fetch_add(1, Ordering::Relaxed) + 1;
        self.subscriptions
            .write()
            .unwrap()
            .insert(id, ActiveSubscription { stream_id, filter, tx });
        tracing::debug!(
            subscription_id = id,
            "StreamIndex 新增数据订阅者"
        );
        id
    }

    /// 按订阅 ID 移除订阅者。返回是否移除成功。
    pub fn unsubscribe(&self, subscription_id: u64) -> bool {
        let removed = self.subscriptions.write().unwrap().remove(&subscription_id);
        if removed.is_some() {
            tracing::debug!(subscription_id, "StreamIndex 移除数据订阅者");
        }
        removed.is_some()
    }

    /// 按来源流 ID 移除订阅者（用于 `extension.stream.unsubscribe(streamId)`）。
    /// 返回移除数量。
    pub fn unsubscribe_by_stream(&self, stream_id: &str) -> usize {
        let mut subscriptions = self.subscriptions.write().unwrap();
        let before = subscriptions.len();
        subscriptions.retain(|_, subscription| {
            subscription.stream_id.as_deref() != Some(stream_id)
        });
        let removed = before - subscriptions.len();
        if removed > 0 {
            tracing::debug!(stream_id, removed, "StreamIndex 按流移除数据订阅者");
        }
        removed
    }

    /// 按 kind + 可选 session_id 广播一条数据给匹配订阅者。
    ///
    /// 使用 tokio broadcast 同步非阻塞 `send`：慢消费者落后时队列覆写为
    /// `Lagged`（丢弃 + 计数），从不阻塞发布方，保实时性（34 §2.8）。
    /// 返回成功投递的订阅者数。
    pub fn publish(&self, kind: &str, session_id: Option<&str>, data: Value) -> usize {
        let subscribers = self.subscriptions.read().unwrap();
        let mut sent = 0usize;
        for subscription in subscribers.values() {
            if !subscription.filter.matches(kind, session_id) {
                continue;
            }
            match subscription.tx.send(data.clone()) {
                Ok(_) => sent += 1,
                Err(broadcast::error::SendError(_)) => {}
            }
        }
        if sent > 0 {
            tracing::debug!(kind, session_id, sent, "StreamIndex 广播流数据");
        }
        sent
    }
}

impl Default for StreamIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_index_track_and_count() {
        let index = StreamIndex::new();
        assert_eq!(index.count(), 0);

        index.track(
            "stream_001",
            StreamSource::new("terminal", "term_001").with_meta("session_id", "sess_001"),
        );
        assert_eq!(index.count(), 1);

        index.track("stream_002", StreamSource::new("agent", "agent_001"));
        assert_eq!(index.count(), 2);
    }

    #[test]
    fn test_index_untrack() {
        let index = StreamIndex::new();

        index.track("stream_001", StreamSource::new("terminal", "term_001"));
        assert_eq!(index.count(), 1);

        index.untrack("stream_001");
        assert_eq!(index.count(), 0);

        // 移除不存在的流不应 panic
        index.untrack("nonexistent");
    }

    #[test]
    fn test_index_record_send() {
        let index = StreamIndex::new();

        index.track("stream_001", StreamSource::new("terminal", "term_001"));

        let active = index.list_all();
        assert_eq!(active[0].chunk_count, 0);

        index.record_send("stream_001");
        index.record_send("stream_001");
        index.record_send("stream_001");

        let active = index.list_all();
        assert_eq!(active[0].chunk_count, 3);

        // 记录不存在的流不应 panic
        index.record_send("nonexistent");
    }

    #[test]
    fn test_index_list_by_kind() {
        let index = StreamIndex::new();

        index.track(
            "s1",
            StreamSource::new("terminal", "t1").with_meta("session_id", "sess_001"),
        );
        index.track(
            "s2",
            StreamSource::new("terminal", "t2").with_meta("session_id", "sess_001"),
        );
        index.track(
            "s3",
            StreamSource::new("agent", "a1").with_meta("session_id", "sess_002"),
        );
        index.track("s4", StreamSource::new("gateway", "g1"));

        assert_eq!(index.list_by_kind("terminal").len(), 2);
        assert_eq!(index.list_by_kind("agent").len(), 1);
        assert_eq!(index.list_by_kind("gateway").len(), 1);
        assert_eq!(index.list_by_kind("nonexistent").len(), 0);
    }

    #[test]
    fn test_index_list_by_session() {
        let index = StreamIndex::new();

        index.track(
            "s1",
            StreamSource::new("terminal", "t1").with_meta("session_id", "sess_001"),
        );
        index.track(
            "s2",
            StreamSource::new("terminal", "t2").with_meta("session_id", "sess_001"),
        );
        index.track(
            "s3",
            StreamSource::new("agent", "a1").with_meta("session_id", "sess_002"),
        );
        index.track("s4", StreamSource::new("gateway", "g1"));

        assert_eq!(index.list_by_session("sess_001").len(), 2);
        assert_eq!(index.list_by_session("sess_002").len(), 1);
        assert_eq!(index.list_by_session("sess_999").len(), 0);
    }

    #[test]
    fn test_index_list_all() {
        let index = StreamIndex::new();

        index.track("s1", StreamSource::new("terminal", "t1"));
        index.track("s2", StreamSource::new("agent", "a1"));

        let active = index.list_all();
        assert_eq!(active.len(), 2);

        let ids: Vec<&str> = active.iter().map(|i| i.stream_id.as_str()).collect();
        assert!(ids.contains(&"s1"));
        assert!(ids.contains(&"s2"));
    }

    #[test]
    fn test_index_stream_info_fields() {
        let index = StreamIndex::new();

        let source = StreamSource::new("terminal", "term_001").with_meta("session_id", "sess_001");
        index.track("stream_001", source);

        let active = index.list_all();
        let info = &active[0];

        assert_eq!(info.stream_id, "stream_001");
        assert_eq!(info.source.kind, "terminal");
        assert_eq!(info.source.id, "term_001");
        assert_eq!(info.source.meta("session_id"), Some("sess_001"));
        assert_eq!(info.chunk_count, 0);
    }

    #[test]
    fn test_index_clone_shares_state() {
        let index1 = StreamIndex::new();
        let index2 = index1.clone();

        index1.track("stream_001", StreamSource::new("terminal", "t1"));
        assert_eq!(index2.count(), 1);

        index2.track("stream_002", StreamSource::new("agent", "a1"));
        assert_eq!(index1.count(), 2);
    }

    #[test]
    fn test_index_subscribe_publish_by_kind_and_session() {
        let index = StreamIndex::new();
        let (tx, mut rx) = broadcast::channel(16);
        let subscription_id = index.subscribe(
            StreamSubscriptionFilter::new("agent").with_session_id("sess_001"),
            tx,
        );
        assert_eq!(subscription_id, 1);

        // kind 匹配但 session 不匹配 → 不投递
        assert_eq!(index.publish("agent", Some("sess_002"), json!({ "n": 1 })), 0);
        assert!(rx.try_recv().is_err());

        // kind + session 都匹配 → 投递
        assert_eq!(index.publish("agent", Some("sess_001"), json!({ "n": 2 })), 1);
        assert_eq!(rx.try_recv().unwrap(), json!({ "n": 2 }));

        // kind 不匹配 → 不投递
        assert_eq!(index.publish("terminal", Some("sess_001"), json!({ "n": 3 })), 0);
        assert!(rx.try_recv().is_err());

        // 取消订阅后不再投递
        assert!(index.unsubscribe(subscription_id));
        assert_eq!(index.publish("agent", Some("sess_001"), json!({ "n": 4 })), 0);
        assert!(rx.try_recv().is_err());

        // 重复取消返回 false
        assert!(!index.unsubscribe(subscription_id));
    }

    #[test]
    fn test_index_subscribe_filter_without_session_matches_any() {
        let index = StreamIndex::new();
        let (tx, mut rx) = broadcast::channel(16);
        index.subscribe(StreamSubscriptionFilter::new("agent"), tx);

        // 无 session 约束的订阅匹配任意会话流，也匹配未指定 session 的发布
        assert_eq!(index.publish("agent", Some("sess_a"), json!({ "n": 1 })), 1);
        assert_eq!(index.publish("agent", None, json!({ "n": 2 })), 1);
        assert_eq!(rx.try_recv().unwrap(), json!({ "n": 1 }));
        assert_eq!(rx.try_recv().unwrap(), json!({ "n": 2 }));
    }

    #[test]
    fn test_index_publish_broadcasts_to_multiple_subscribers() {
        let index = StreamIndex::new();
        let (tx_a, mut rx_a) = broadcast::channel(16);
        let (tx_b, mut rx_b) = broadcast::channel(16);
        index.subscribe(StreamSubscriptionFilter::new("agent"), tx_a);
        index.subscribe(StreamSubscriptionFilter::new("agent"), tx_b);

        assert_eq!(index.publish("agent", Some("sess_001"), json!({ "n": 5 })), 2);
        assert_eq!(rx_a.try_recv().unwrap(), json!({ "n": 5 }));
        assert_eq!(rx_b.try_recv().unwrap(), json!({ "n": 5 }));
    }

    #[test]
    fn test_index_unsubscribe_by_stream() {
        let index = StreamIndex::new();
        let (tx, mut rx) = broadcast::channel(16);
        index.subscribe_for_stream(
            "stream:agent:uuid-1",
            StreamSubscriptionFilter::new("agent"),
            tx,
        );

        // 按流取消一次成功，二次无操作
        assert_eq!(index.unsubscribe_by_stream("stream:agent:uuid-1"), 1);
        assert_eq!(index.unsubscribe_by_stream("stream:agent:uuid-1"), 0);

        assert_eq!(index.publish("agent", None, json!({ "n": 6 })), 0);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn test_index_subscription_filter_matches_stream() {
        let index = StreamIndex::new();
        index.track(
            "stream_001",
            StreamSource::new("agent", "a1").with_meta("session_id", "sess_001"),
        );
        index.track(
            "stream_002",
            StreamSource::new("agent", "a2").with_meta("session_id", "sess_002"),
        );
        index.track("stream_003", StreamSource::new("terminal", "t1"));

        let all = index.list_all();
        let by_kind = StreamSubscriptionFilter::new("agent");
        assert_eq!(all.iter().filter(|info| by_kind.matches_stream(info)).count(), 2);

        let by_kind_and_session =
            StreamSubscriptionFilter::new("agent").with_session_id("sess_001");
        let matched: Vec<&StreamInfo> = all
            .iter()
            .filter(|info| by_kind_and_session.matches_stream(info))
            .collect();
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].stream_id, "stream_001");
    }
}
