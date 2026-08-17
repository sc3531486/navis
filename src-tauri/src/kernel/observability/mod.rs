use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::sync::OnceLock;
use triomphe::Arc as SharedArc;

use super::{EventBus, EventEnvelope, KernelContext, KernelErrorKind};

/// Shared null payload — allocated once, cloned everywhere (refcount bump only).
fn null_payload() -> SharedArc<Value> {
    static NULL: OnceLock<SharedArc<Value>> = OnceLock::new();
    NULL.get_or_init(|| SharedArc::new(Value::Null)).clone()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionEventKind {
    RunStarted,
    RunCompleted,
    RunFailed,
    RunCancelled,
    StageStarted,
    StageDelta,
    StageCompleted,
    StageFailed,
    CapabilityCalled,
    CapabilityCompleted,
    CapabilityFailed,
}

impl ExecutionEventKind {
    pub fn topic(self) -> &'static str {
        match self {
            Self::RunStarted => "execution.run.started",
            Self::RunCompleted => "execution.run.completed",
            Self::RunFailed => "execution.run.failed",
            Self::RunCancelled => "execution.run.cancelled",
            Self::StageStarted => "execution.stage.started",
            Self::StageDelta => "execution.stage.delta",
            Self::StageCompleted => "execution.stage.completed",
            Self::StageFailed => "execution.stage.failed",
            Self::CapabilityCalled => "execution.capability.called",
            Self::CapabilityCompleted => "execution.capability.completed",
            Self::CapabilityFailed => "execution.capability.failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEvent {
    pub id: String,
    pub kind: ExecutionEventKind,
    pub run_id: String,
    pub stage_id: Option<String>,
    pub capability_id: Option<String>,
    pub context: KernelContext,
    pub sequence: u64,
    pub message: Option<String>,
    pub error_kind: Option<KernelErrorKind>,
    pub duration_ms: Option<u64>,
    pub payload: SharedArc<Value>,
    pub created_at: DateTime<Utc>,
}

impl ExecutionEvent {
    pub fn new(
        kind: ExecutionEventKind,
        run_id: impl Into<String>,
        context: &KernelContext,
        sequence: u64,
    ) -> Self {
        Self {
            id: uuid::Uuid::now_v7().to_string(),
            kind,
            run_id: run_id.into(),
            stage_id: None,
            capability_id: None,
            context: context.clone(),
            sequence,
            message: None,
            error_kind: None,
            duration_ms: None,
            payload: null_payload(),
            created_at: Utc::now(),
        }
    }

    pub fn with_stage_id(mut self, stage_id: impl Into<String>) -> Self {
        self.stage_id = Some(stage_id.into());
        self
    }

    pub fn with_capability_id(mut self, capability_id: impl Into<String>) -> Self {
        self.capability_id = Some(capability_id.into());
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn with_error_kind(mut self, error_kind: KernelErrorKind) -> Self {
        self.error_kind = Some(error_kind);
        self
    }

    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    pub fn with_payload(mut self, payload: Value) -> Self {
        self.payload = SharedArc::new(payload);
        self
    }
}

pub type SharedExecutionEvent = SharedArc<ExecutionEvent>;
pub type ExecutionObserver = Arc<dyn Fn(SharedExecutionEvent) + Send + Sync>;

#[derive(Clone)]
pub struct ExecutionObservationSink {
    observer: ExecutionObserver,
    is_enabled: bool,
}

impl Default for ExecutionObservationSink {
    fn default() -> Self {
        Self::disabled()
    }
}

impl ExecutionObservationSink {
    pub fn disabled() -> Self {
        Self {
            observer: Arc::new(|_| {}),
            is_enabled: false,
        }
    }

    pub fn new(observer: ExecutionObserver) -> Self {
        Self {
            observer,
            is_enabled: true,
        }
    }

    pub fn from_fn(observer: impl Fn(SharedExecutionEvent) + Send + Sync + 'static) -> Self {
        Self::new(Arc::new(observer))
    }

    pub fn observe(&self, event: ExecutionEvent) {
        (self.observer)(SharedArc::new(event));
    }

    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    pub fn event_bus(event_bus: Arc<dyn EventBus>) -> Self {
        Self::from_fn(move |event| {
            let payload = match serde_json::to_value(event.as_ref()) {
                Ok(payload) => payload,
                Err(error) => {
                    tracing::error!(
                        error = %error,
                        run_id = %event.run_id,
                        "execution observation serialization failed"
                    );
                    return;
                }
            };
            if let Err(error) = event_bus.emit(EventEnvelope::new(
                event.kind.topic(),
                event.context.clone(),
                Some(triomphe::Arc::new(payload)),
            )) {
                tracing::warn!(
                    error = %error,
                    run_id = %event.run_id,
                    topic = event.kind.topic(),
                    "execution observation event emit failed"
                );
            }
        })
    }
}
