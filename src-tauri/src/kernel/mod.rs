//! Navis kernel primitives.
//!
//! The kernel layer only defines generic infrastructure:
//! discovery, execution, notification, and authorization.

pub mod cordis;
pub mod audit;
pub mod core;
pub mod event;
pub mod observability;
pub mod pipeline;
pub mod policy;
pub mod registry;
pub mod snapshot;

#[cfg(test)]
mod boundary_test;

pub use cordis::{CordisContext, Fiber, FiberManager, FiberState, NamedEntries, ScopedLayers, Service};
pub use audit::{
    AuditDigest, AuditRecord, AuditRecorder, AuditSink, AuditStats, AuditStatus, BufferedAuditSink,
    FieldMeta, InMemoryAuditSink,
};
pub use core::{
    CapabilityId, KernelContext, KernelError, KernelErrorKind, KernelObjectInfo, KernelObjectState,
    KernelResource, KernelResult, KernelScope, PolicyErrorKind, PolicyId, ResourceLease,
    SchemaVersion, ShutdownMode, SpanId, StageId, SubscriptionId, Topic, TraceId,
};
pub use event::{
    AsyncEventHandler, EventBus, EventBusStats, EventEnvelope, EventHandler, EventSubscription,
    InMemoryEventBus, SharedEventEnvelope,
};
pub use observability::{
    ExecutionEvent, ExecutionEventKind, ExecutionObservationSink, ExecutionObserver,
    SharedExecutionEvent,
};
pub use pipeline::{Next, Pipeline, PipelineContext, PipelineRetryPolicy, PipelineStats, Stage};
pub use policy::{
    Constraint, ConstraintInfo, PolicyCheckpoint, PolicyDecision, PolicyEngine, PolicyInput,
    PolicyStats,
};
pub use registry::{
    AsyncRegistry, Capability, CapabilityInfo, CapabilityLifecycle, InMemoryRegistry,
    LifecycleAction, LifecycleState, Registry, RegistryEntry, RegistryLoader, RegistryStats,
};
pub use snapshot::KernelSnapshot;
