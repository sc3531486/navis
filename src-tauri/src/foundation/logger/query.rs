//! QueryLayer - 前端查询 tracing Layer
//!
//! 基于设计文档 §8 实现，维护内存环形缓冲区供前端查询

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

/// 日志条目（供前端查询）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: HashMap<String, String>,
    pub session_id: Option<String>,
}

/// 日志过滤器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFilter {
    pub level: Option<String>,
    pub module: Option<String>,
    pub session_id: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub keyword: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// 环形缓冲区
struct RingBuffer<T> {
    buffer: Vec<Option<T>>,
    capacity: usize,
    head: usize,
    tail: usize,
    size: usize,
}

impl<T: Clone> RingBuffer<T> {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![None; capacity],
            capacity,
            head: 0,
            tail: 0,
            size: 0,
        }
    }

    fn push(&mut self, item: T) {
        self.buffer[self.tail] = Some(item);
        self.tail = (self.tail + 1) % self.capacity;

        if self.size < self.capacity {
            self.size += 1;
        } else {
            // 缓冲区满，覆盖最旧的元素
            self.head = (self.head + 1) % self.capacity;
        }
    }

    fn iter(&self) -> impl Iterator<Item = &T> {
        let mut current = self.head;
        let mut count = 0;

        std::iter::from_fn(move || {
            if count >= self.size {
                return None;
            }

            let item = self.buffer[current].as_ref();
            current = (current + 1) % self.capacity;
            count += 1;

            item
        })
    }

    fn tail_lines(&self, n: usize) -> Vec<&T> {
        let skip = if self.size > n { self.size - n } else { 0 };
        self.iter().skip(skip).collect()
    }
}

/// QueryLayer - tracing Layer，维护内存环形缓冲区供前端查询
#[derive(Clone)]
pub struct QueryLayer {
    buffer: Arc<Mutex<RingBuffer<LogEntry>>>,
}

impl QueryLayer {
    /// 创建新的 QueryLayer
    ///
    /// # Arguments
    /// * `capacity` - 环形缓冲区容量（默认 1000 条）
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(RingBuffer::new(capacity))),
        }
    }

    /// 查询日志（从环形缓冲区）
    pub fn query(&self, filter: &LogFilter) -> Vec<LogEntry> {
        let buffer = self.buffer.lock().unwrap();
        let mut entries: Vec<LogEntry> = buffer
            .iter()
            .filter(|entry| Self::matches_filter(entry, filter))
            .cloned()
            .collect();

        // 按时间倒序
        entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // 分页
        let offset = filter.offset.unwrap_or(0);
        let limit = filter.limit.unwrap_or(entries.len());

        entries.into_iter().skip(offset).take(limit).collect()
    }

    /// 获取最近 N 条日志
    pub fn tail(&self, lines: u32) -> Vec<LogEntry> {
        let buffer = self.buffer.lock().unwrap();
        buffer
            .tail_lines(lines as usize)
            .into_iter()
            .cloned()
            .collect()
    }

    /// 检查日志条目是否匹配过滤器
    fn matches_filter(entry: &LogEntry, filter: &LogFilter) -> bool {
        // 级别过滤
        if let Some(ref level) = filter.level {
            if entry.level.to_lowercase() != level.to_lowercase() {
                return false;
            }
        }

        // 模块过滤
        if let Some(ref module) = filter.module {
            if !entry.target.contains(module.as_str()) {
                return false;
            }
        }

        // 会话过滤
        if let Some(ref session_id) = filter.session_id {
            if entry.session_id.as_ref() != Some(session_id) {
                return false;
            }
        }

        // 时间范围过滤
        if let Some(start_time) = filter.start_time {
            if entry.timestamp < start_time {
                return false;
            }
        }

        if let Some(end_time) = filter.end_time {
            if entry.timestamp > end_time {
                return false;
            }
        }

        // 关键词过滤
        if let Some(ref keyword) = filter.keyword {
            if !entry.message.contains(keyword.as_str()) {
                return false;
            }
        }

        true
    }

    /// 写入日志条目到缓冲区（由 tracing Layer 调用）
    pub fn push_entry(&self, entry: LogEntry) {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.push(entry);
    }
}

/// tracing Layer 实现
impl<S: tracing::Subscriber> Layer<S> for QueryLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        // 提取事件信息
        let metadata = event.metadata();
        let level = metadata.level().to_string();
        let target = metadata.target().to_string();

        // 提取字段
        let mut fields = HashMap::new();
        let mut visitor = FieldVisitor(&mut fields);
        event.record(&mut visitor);

        // 提取消息
        let message = fields.remove("message").unwrap_or_default();

        // 提取 session_id
        let session_id = fields.remove("session_id");

        // 创建日志条目
        let entry = LogEntry {
            timestamp: Utc::now(),
            level,
            target,
            message,
            fields,
            session_id,
        };

        // 写入缓冲区
        self.push_entry(entry);
    }
}

/// 字段访问者
struct FieldVisitor<'a>(&'a mut HashMap<String, String>);

impl<'a> tracing::field::Visit for FieldVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().to_string(), format!("{:?}", value));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.insert(field.name().to_string(), value.to_string());
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.0.insert(field.name().to_string(), value.to_string());
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.0.insert(field.name().to_string(), value.to_string());
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.0.insert(field.name().to_string(), value.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_basic() {
        let mut buffer = RingBuffer::new(3);
        assert_eq!(buffer.size, 0);

        buffer.push(1);
        buffer.push(2);
        buffer.push(3);

        assert_eq!(buffer.size, 3);
        let items: Vec<&i32> = buffer.iter().collect();
        assert_eq!(items, vec![&1, &2, &3]);
    }

    #[test]
    fn test_ring_buffer_overflow() {
        let mut buffer = RingBuffer::new(3);

        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        buffer.push(4); // 覆盖 1

        assert_eq!(buffer.size, 3);
        let items: Vec<&i32> = buffer.iter().collect();
        assert_eq!(items, vec![&2, &3, &4]);
    }

    #[test]
    fn test_ring_buffer_tail() {
        let mut buffer = RingBuffer::new(5);

        for i in 1..=5 {
            buffer.push(i);
        }

        let tail = buffer.tail_lines(3);
        assert_eq!(tail, vec![&3, &4, &5]);
    }

    #[test]
    fn test_query_layer_filter() {
        let layer = QueryLayer::new(100);

        // 插入测试数据
        layer.push_entry(LogEntry {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            target: "gateway".to_string(),
            message: "Request completed".to_string(),
            fields: HashMap::new(),
            session_id: Some("sess_1".to_string()),
        });

        layer.push_entry(LogEntry {
            timestamp: Utc::now(),
            level: "ERROR".to_string(),
            target: "agent".to_string(),
            message: "Tool call failed".to_string(),
            fields: HashMap::new(),
            session_id: Some("sess_2".to_string()),
        });

        // 测试级别过滤
        let filter = LogFilter {
            level: Some("INFO".to_string()),
            module: None,
            session_id: None,
            start_time: None,
            end_time: None,
            keyword: None,
            limit: None,
            offset: None,
        };

        let results = layer.query(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].level, "INFO");

        // 测试模块过滤
        let filter = LogFilter {
            level: None,
            module: Some("gateway".to_string()),
            session_id: None,
            start_time: None,
            end_time: None,
            keyword: None,
            limit: None,
            offset: None,
        };

        let results = layer.query(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].target, "gateway");

        // 测试会话过滤
        let filter = LogFilter {
            level: None,
            module: None,
            session_id: Some("sess_1".to_string()),
            start_time: None,
            end_time: None,
            keyword: None,
            limit: None,
            offset: None,
        };

        let results = layer.query(&filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, Some("sess_1".to_string()));
    }

    #[test]
    fn test_query_layer_tail() {
        let layer = QueryLayer::new(100);

        for i in 1..=10 {
            layer.push_entry(LogEntry {
                timestamp: Utc::now(),
                level: "INFO".to_string(),
                target: "test".to_string(),
                message: format!("Message {}", i),
                fields: HashMap::new(),
                session_id: None,
            });
        }

        let tail = layer.tail(3);
        assert_eq!(tail.len(), 3);
        assert_eq!(tail[0].message, "Message 8");
        assert_eq!(tail[1].message, "Message 9");
        assert_eq!(tail[2].message, "Message 10");
    }
}
