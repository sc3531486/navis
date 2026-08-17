use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use super::{KernelContext, KernelError, KernelResult, SpanId, TraceId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMeta {
    pub name: String,
    pub value_type: String,
    pub byte_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditDigest {
    Truncated {
        text: String,
        original_bytes: usize,
        truncated: bool,
    },
    Metadata {
        fields: Vec<FieldMeta>,
    },
    Redacted {
        content_type: String,
    },
    None,
}

impl AuditDigest {
    pub fn from_text(text: impl Into<String>, limit: usize) -> Self {
        let text = text.into();
        let original_bytes = text.len();
        if original_bytes <= limit {
            return Self::Truncated {
                text,
                original_bytes,
                truncated: false,
            };
        }

        let mut truncated_text = String::new();
        for character in text.chars() {
            if truncated_text.len() + character.len_utf8() > limit {
                break;
            }
            truncated_text.push(character);
        }

        Self::Truncated {
            text: truncated_text,
            original_bytes,
            truncated: true,
        }
    }

    pub fn from_value_metadata(value: &Value) -> Self {
        let Value::Object(map) = value else {
            return Self::Metadata {
                fields: vec![FieldMeta {
                    name: "$".into(),
                    value_type: value_type(value).into(),
                    byte_size: estimate_byte_size(value),
                }],
            };
        };

        Self::Metadata {
            fields: map
                .iter()
                .map(|(name, value)| FieldMeta {
                    name: name.clone(),
                    value_type: value_type(value).into(),
                    byte_size: estimate_byte_size(value),
                })
                .collect(),
        }
    }
}

fn value_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// 估算 JSON 值序列化后的字节数，零堆分配。
///
/// 精度：字符串用 `str::len()`，整数用数学数位计算，
/// 浮点用 f64 估算（±1-2 字节误差），容器递归加 JSON 分隔符。
fn estimate_byte_size(value: &Value) -> usize {
    match value {
        Value::Null => 4,
        Value::Bool(b) => {
            if *b {
                4
            } else {
                5
            }
        }
        Value::Number(n) => estimate_number_size(n),
        Value::String(s) => s.len() + 2,
        Value::Array(arr) => {
            if arr.is_empty() {
                return 2;
            }
            let inner: usize = arr.iter().map(|v| estimate_byte_size(v)).sum();
            inner + arr.len() - 1 + 2
        }
        Value::Object(map) => {
            if map.is_empty() {
                return 2;
            }
            let inner: usize = map
                .iter()
                .map(|(k, v)| k.len() + 3 + estimate_byte_size(v))
                .sum();
            inner + map.len() - 1 + 2
        }
    }
}

/// 估算 JSON Number 序列化字节数，零堆分配。
fn estimate_number_size(n: &serde_json::Number) -> usize {
    if let Some(v) = n.as_u64() {
        return count_digits(v);
    }
    if let Some(v) = n.as_i64() {
        return if v < 0 {
            count_digits((-v) as u64) + 1
        } else {
            count_digits(v as u64)
        };
    }
    // 浮点：整数部分 + 小数点 + 6 位精度 + 符号
    if let Some(v) = n.as_f64() {
        let abs = v.abs();
        if abs < 1.0 {
            return 8;
        } // "0.xxxxxx"
        let int_part = count_digits(abs as u64);
        return int_part + 8; // "." + 6 位 + 可能的符号
    }
    1 // fallback
}

/// 计算非负整数的十进制位数。
fn count_digits(mut n: u64) -> usize {
    if n == 0 {
        return 1;
    }
    let mut count = 0;
    while n > 0 {
        count += 1;
        n /= 10;
    }
    count
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditStatus {
    Success,
    Failed,
    Truncated,
    Retried,
    Cancelled,
}

impl std::fmt::Display for AuditStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Truncated => "truncated",
            Self::Retried => "retried",
            Self::Cancelled => "cancelled",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: String,
    pub schema_version: i32,
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub parent_span_id: Option<SpanId>,
    pub scope: String,
    pub source: String,
    pub operation_id: String,
    pub action: String,
    pub policy_decision: Option<Value>,
    pub duration_ms: Option<i64>,
    pub input_digest: AuditDigest,
    pub output_digest: AuditDigest,
    pub status: AuditStatus,
    pub created_at: DateTime<Utc>,
}

impl AuditRecord {
    pub fn new(
        context: &KernelContext,
        operation_id: impl Into<String>,
        action: impl Into<String>,
        status: AuditStatus,
    ) -> Self {
        Self {
            id: uuid::Uuid::now_v7().to_string(),
            schema_version: 1,
            trace_id: context.trace_id.clone(),
            span_id: SpanId::generate(),
            parent_span_id: None,
            scope: context.scope_key_ref().to_string(),
            source: context.source.clone(),
            operation_id: operation_id.into(),
            action: action.into(),
            policy_decision: None,
            duration_ms: None,
            input_digest: AuditDigest::None,
            output_digest: AuditDigest::None,
            status,
            created_at: Utc::now(),
        }
    }

    pub fn with_policy_decision(mut self, decision: Value) -> Self {
        self.policy_decision = Some(decision);
        self
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration_ms = Some(duration.as_millis() as i64);
        self
    }

    pub fn with_duration_ms(mut self, duration_ms: i64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    pub fn with_input_digest(mut self, digest: AuditDigest) -> Self {
        self.input_digest = digest;
        self
    }

    pub fn with_output_digest(mut self, digest: AuditDigest) -> Self {
        self.output_digest = digest;
        self
    }

    pub fn with_value_input_metadata(self, value: &Value) -> Self {
        self.with_input_digest(AuditDigest::from_value_metadata(value))
    }

    pub fn with_text_output(self, text: impl Into<String>, limit: usize) -> Self {
        self.with_output_digest(AuditDigest::from_text(text, limit))
    }
}

pub trait AuditSink: Send + Sync {
    fn record(&self, record: &AuditRecord) -> KernelResult<()>;

    fn record_shared(&self, record: Arc<AuditRecord>) -> KernelResult<()> {
        self.record(record.as_ref())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditStats {
    pub enabled: bool,
    pub attempted_records: u64,
    pub succeeded_records: u64,
    pub failed_records: u64,
}

#[derive(Default)]
struct AuditCounters {
    attempted_records: AtomicU64,
    succeeded_records: AtomicU64,
    failed_records: AtomicU64,
}

#[derive(Clone, Default)]
pub struct AuditRecorder {
    sink: Option<Arc<dyn AuditSink>>,
    counters: Arc<AuditCounters>,
}

impl AuditRecorder {
    pub fn new(sink: Arc<dyn AuditSink>) -> Self {
        Self {
            sink: Some(sink),
            counters: Arc::new(AuditCounters::default()),
        }
    }

    pub fn disabled() -> Self {
        Self {
            sink: None,
            counters: Arc::new(AuditCounters::default()),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.sink.is_some()
    }

    pub fn stats(&self) -> AuditStats {
        AuditStats {
            enabled: self.is_enabled(),
            attempted_records: self.counters.attempted_records.load(Ordering::Relaxed),
            succeeded_records: self.counters.succeeded_records.load(Ordering::Relaxed),
            failed_records: self.counters.failed_records.load(Ordering::Relaxed),
        }
    }

    pub fn record_owned(&self, record: AuditRecord) -> KernelResult<()> {
        self.record_shared(Arc::new(record))
    }

    pub fn record_shared(&self, record: Arc<AuditRecord>) -> KernelResult<()> {
        self.counters
            .attempted_records
            .fetch_add(1, Ordering::Relaxed);
        tracing::info!(
            audit_id = %record.id,
            trace_id = %record.trace_id,
            span_id = %record.span_id,
            parent_span_id = record.parent_span_id.as_ref().map(|id| id.as_str()),
            schema_version = record.schema_version,
            scope = %record.scope,
            source = %record.source,
            operation_id = %record.operation_id,
            action = %record.action,
            status = %record.status,
            duration_ms = record.duration_ms,
            "kernel audit record"
        );
        if let Some(sink) = &self.sink {
            match sink.record_shared(record) {
                Ok(()) => {
                    self.counters
                        .succeeded_records
                        .fetch_add(1, Ordering::Relaxed);
                }
                Err(error) => {
                    self.counters.failed_records.fetch_add(1, Ordering::Relaxed);
                    return Err(error);
                }
            }
        } else {
            self.counters
                .succeeded_records
                .fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }
}

pub struct InMemoryAuditSink {
    records: RwLock<Vec<AuditRecord>>,
}

impl Default for InMemoryAuditSink {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryAuditSink {
    pub fn new() -> Self {
        Self {
            records: RwLock::new(Vec::new()),
        }
    }

    pub fn records(&self) -> Vec<AuditRecord> {
        self.records.read().clone()
    }
}

impl AuditSink for InMemoryAuditSink {
    fn record(&self, record: &AuditRecord) -> KernelResult<()> {
        tracing::trace!(
            audit_id = %record.id,
            "writing audit entry to memory sink"
        );
        self.records.write().push(record.clone());
        Ok(())
    }
}

pub struct BufferedAuditSink {
    tx: crossbeam_channel::Sender<Arc<AuditRecord>>,
}

impl BufferedAuditSink {
    pub fn new(backend: Arc<dyn AuditSink>) -> KernelResult<Self> {
        Self::with_options(backend, 8192, 64, Duration::from_millis(100))
    }

    pub fn with_options(
        backend: Arc<dyn AuditSink>,
        capacity: usize,
        batch_size: usize,
        flush_interval: Duration,
    ) -> KernelResult<Self> {
        let (tx, rx) = crossbeam_channel::bounded::<Arc<AuditRecord>>(capacity.max(1));
        std::thread::Builder::new()
            .name("navis-kernel-audit-buffer".into())
            .spawn(move || flush_audit_records(rx, backend, batch_size, flush_interval))
            .map_err(|error| KernelError::AuditSinkFailed {
                message: error.to_string(),
            })?;

        Ok(Self { tx })
    }
}

impl AuditSink for BufferedAuditSink {
    fn record(&self, record: &AuditRecord) -> KernelResult<()> {
        self.record_shared(Arc::new(record.clone()))
    }

    fn record_shared(&self, record: Arc<AuditRecord>) -> KernelResult<()> {
        self.tx
            .try_send(record)
            .map_err(|error| KernelError::AuditSinkFailed {
                message: format!("buffered audit channel failed: {error}"),
            })
    }
}

fn flush_audit_records(
    rx: crossbeam_channel::Receiver<Arc<AuditRecord>>,
    backend: Arc<dyn AuditSink>,
    batch_size: usize,
    flush_interval: Duration,
) {
    let mut batch = Vec::with_capacity(batch_size.max(1));

    loop {
        match rx.recv_timeout(flush_interval) {
            Ok(record) => {
                batch.push(record);

                while batch.len() < batch.capacity() {
                    match rx.try_recv() {
                        Ok(record) => batch.push(record),
                        Err(crossbeam_channel::TryRecvError::Empty) => break,
                        Err(crossbeam_channel::TryRecvError::Disconnected) => break,
                    }
                }

                for record in batch.drain(..) {
                    if let Err(error) = backend.record_shared(record) {
                        tracing::error!(error = %error, "buffered audit sink flush failed");
                    }
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                for record in batch.drain(..) {
                    if let Err(error) = backend.record_shared(record) {
                        tracing::error!(error = %error, "buffered audit sink flush failed");
                    }
                }
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::{KernelContext, KernelScope};

    #[test]
    fn text_digest_tracks_truncation() {
        let digest = AuditDigest::from_text("abcdef", 3);
        match digest {
            AuditDigest::Truncated {
                text,
                original_bytes,
                truncated,
            } => {
                assert_eq!(text, "abc");
                assert_eq!(original_bytes, 6);
                assert!(truncated);
            }
            _ => panic!("unexpected digest"),
        }
    }

    #[test]
    fn in_memory_sink_records() {
        let sink = Arc::new(InMemoryAuditSink::new());
        let recorder = AuditRecorder::new(sink.clone());
        let context = KernelContext::new("test", KernelScope::global());
        let record = AuditRecord::new(&context, "operation", "run", AuditStatus::Success);

        recorder.record_owned(record).unwrap();

        assert_eq!(sink.records().len(), 1);
        assert!(recorder.is_enabled());
        assert!(!AuditRecorder::disabled().is_enabled());
        let stats = recorder.stats();
        assert_eq!(stats.attempted_records, 1);
        assert_eq!(stats.succeeded_records, 1);
        assert_eq!(stats.failed_records, 0);
    }

    #[test]
    fn audit_record_builder_sets_optional_fields() {
        let context = KernelContext::new("test", KernelScope::global());
        let record = AuditRecord::new(&context, "operation", "run", AuditStatus::Success)
            .with_policy_decision(serde_json::json!({ "decision": "allow" }))
            .with_duration(Duration::from_millis(12))
            .with_value_input_metadata(&serde_json::json!({ "path": "file.rs" }))
            .with_text_output("hello world", 5);

        assert_eq!(record.duration_ms, Some(12));
        assert!(record.policy_decision.is_some());
        assert!(matches!(record.input_digest, AuditDigest::Metadata { .. }));
        assert!(matches!(
            record.output_digest,
            AuditDigest::Truncated {
                truncated: true,
                ..
            }
        ));
    }

    #[test]
    fn buffered_sink_records_without_blocking_caller() {
        let backend = Arc::new(InMemoryAuditSink::new());
        let sink =
            BufferedAuditSink::with_options(backend.clone(), 8, 2, Duration::from_millis(10))
                .unwrap();
        let context = KernelContext::new("test", KernelScope::global());
        let record = AuditRecord::new(&context, "operation", "run", AuditStatus::Success);

        sink.record(&record).unwrap();

        for _ in 0..20 {
            if backend.records().len() == 1 {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(backend.records().len(), 1);
    }
}
