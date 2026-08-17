use async_trait::async_trait;
use backon::{BackoffBuilder, ExponentialBuilder};
use chrono::Utc;
use downcast_rs::{impl_downcast, DowncastSync};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;

use super::{
    AuditRecorder, ExecutionEvent, ExecutionEventKind, ExecutionObservationSink, KernelContext,
    KernelError, KernelObjectInfo, KernelObjectState, KernelResource, KernelResult, ResourceLease,
    ShutdownMode,
};

pub trait PipelineData: DowncastSync + Send {
    fn type_name(&self) -> &'static str;
}

impl<T> PipelineData for T
where
    T: DowncastSync + Send,
{
    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}

impl_downcast!(sync PipelineData);

#[derive(Debug, Clone, Copy)]
pub struct PipelineRetryPolicy {
    pub max_retries: usize,
    pub min_delay: Duration,
    pub max_delay: Duration,
    pub jitter: bool,
}

impl Default for PipelineRetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            min_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            jitter: true,
        }
    }
}

impl PipelineRetryPolicy {
    pub fn new(max_retries: usize) -> Self {
        Self {
            max_retries,
            ..Self::default()
        }
    }

    pub fn with_min_delay(mut self, min_delay: Duration) -> Self {
        self.min_delay = min_delay;
        self
    }

    pub fn with_max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = max_delay;
        self
    }

    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    fn backoff(self) -> ExponentialBuilder {
        let builder = ExponentialBuilder::default()
            .with_min_delay(self.min_delay)
            .with_max_delay(self.max_delay)
            .with_max_times(self.max_retries);

        if self.jitter {
            builder.with_jitter()
        } else {
            builder
        }
    }
}

pub struct PipelineContext {
    data: Box<dyn PipelineData>,
    pub context: KernelContext,
    pub cancellation: CancellationToken,
    pub observations: ExecutionObservationSink,
    pub audit: AuditRecorder,
    run_id: Arc<str>,
    next_observation_sequence: u64,
}

impl PipelineContext {
    pub fn new(data: impl PipelineData + 'static, context: KernelContext) -> Self {
        let run_id: Arc<str> = context.trace_id.to_string().into();
        Self {
            data: Box::new(data),
            context,
            cancellation: CancellationToken::new(),
            observations: ExecutionObservationSink::disabled(),
            audit: AuditRecorder::disabled(),
            run_id,
            next_observation_sequence: 0,
        }
    }

    pub fn with_cancellation(mut self, cancellation: CancellationToken) -> Self {
        self.cancellation = cancellation;
        self
    }

    pub fn with_observations(mut self, observations: ExecutionObservationSink) -> Self {
        self.observations = observations;
        self
    }

    pub fn with_observer_fn(
        mut self,
        observer: impl Fn(super::SharedExecutionEvent) + Send + Sync + 'static,
    ) -> Self {
        self.observations = ExecutionObservationSink::from_fn(observer);
        self
    }

    pub fn with_audit(mut self, audit: AuditRecorder) -> Self {
        self.audit = audit;
        self
    }

    pub fn without_audit(mut self) -> Self {
        self.audit = AuditRecorder::disabled();
        self
    }

    pub fn data<T: PipelineData>(&self) -> KernelResult<&T> {
        self.data
            .downcast_ref::<T>()
            .ok_or_else(|| KernelError::PayloadTypeMismatch {
                expected: std::any::type_name::<T>(),
                actual: self.data.type_name(),
            })
    }

    pub fn data_mut<T: PipelineData>(&mut self) -> KernelResult<&mut T> {
        let actual = self.data.type_name();
        self.data
            .downcast_mut::<T>()
            .ok_or_else(|| KernelError::PayloadTypeMismatch {
                expected: std::any::type_name::<T>(),
                actual,
            })
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    fn next_observation_sequence(&mut self) -> u64 {
        let sequence = self.next_observation_sequence;
        self.next_observation_sequence = self.next_observation_sequence.saturating_add(1);
        sequence
    }

    pub fn observe(&mut self, kind: ExecutionEventKind) {
        if !self.observations.is_enabled() {
            return;
        }
        let event = self.execution_event(kind);
        self.observations.observe(event);
    }

    fn execution_event(&mut self, kind: ExecutionEventKind) -> ExecutionEvent {
        let sequence = self.next_observation_sequence();
        ExecutionEvent::new(
            kind,
            Arc::clone(&self.run_id).to_string(),
            &self.context,
            sequence,
        )
    }

    pub fn observe_stage_delta(
        &mut self,
        stage_id: impl Into<String>,
        message: impl Into<String>,
        payload: Value,
    ) {
        if !self.observations.is_enabled() {
            return;
        }
        let event = self
            .execution_event(ExecutionEventKind::StageDelta)
            .with_stage_id(stage_id)
            .with_message(message)
            .with_payload(payload);
        self.observations.observe(event);
    }

    pub fn ensure_active(&self) -> KernelResult<()> {
        if self.cancellation.is_cancelled() {
            return Err(KernelError::Cancelled);
        }

        if let Some(deadline) = self.context.deadline {
            if Utc::now() > deadline {
                return Err(KernelError::DeadlineExceeded);
            }
        }

        Ok(())
    }
}

/// A cooperative async pipeline stage.
///
/// Cancellation is checked by the pipeline before each stage boundary. A stage
/// that performs long-running work, waits on external IO, or loops internally
/// must call [`PipelineContext::ensure_active`] itself at sensible checkpoints.
/// The pipeline does not preempt or abort a stage that ignores the cancellation
/// token while inside [`Stage::process`].
#[async_trait]
pub trait Stage: Send + Sync {
    fn id(&self) -> &str;
    async fn process(&self, context: &mut PipelineContext, next: Next<'_>) -> KernelResult<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineStats {
    pub active_runs: usize,
    pub completed_runs: u64,
    pub failed_runs: u64,
    pub total_runs: u64,
    pub total_duration_ms: u64,
}

#[derive(Default)]
struct PipelineCounters {
    active_runs: AtomicUsize,
    completed_runs: AtomicU64,
    failed_runs: AtomicU64,
    total_duration_ms: AtomicU64,
}

impl PipelineCounters {
    fn stats(&self) -> PipelineStats {
        let completed_runs = self.completed_runs.load(Ordering::Relaxed);
        let failed_runs = self.failed_runs.load(Ordering::Relaxed);
        PipelineStats {
            active_runs: self.active_runs.load(Ordering::Relaxed),
            completed_runs,
            failed_runs,
            total_runs: completed_runs + failed_runs,
            total_duration_ms: self.total_duration_ms.load(Ordering::Relaxed),
        }
    }
}

pub struct Pipeline {
    stages: Vec<Arc<dyn Stage>>,
    stage_ids: Vec<String>,
    stage_id_set: HashSet<String>,
    required_stage_ids: Vec<String>,
    counters: Arc<PipelineCounters>,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self {
            stages: Vec::new(),
            stage_ids: Vec::new(),
            stage_id_set: HashSet::new(),
            required_stage_ids: Vec::new(),
            counters: Arc::new(PipelineCounters::default()),
        }
    }
}

impl Pipeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, stage: impl Stage + 'static) {
        self.push_arc(Arc::new(stage));
    }

    pub fn push_arc(&mut self, stage: Arc<dyn Stage>) {
        let stage_id = stage.id().to_string();
        self.stage_id_set.insert(stage_id.clone());
        self.stage_ids.push(stage_id);
        self.stages.push(stage);
    }

    pub fn require_stage(mut self, stage_id: impl Into<String>) -> Self {
        self.required_stage_ids.push(stage_id.into());
        self
    }

    pub fn stage_ids(&self) -> Vec<String> {
        self.stage_ids.clone()
    }

    pub fn stats(&self) -> PipelineStats {
        self.counters.stats()
    }

    pub fn objects(&self) -> Vec<KernelObjectInfo> {
        let stats = self.stats();
        let mut objects = vec![KernelObjectInfo::new(
            "pipeline",
            "pipeline",
            if stats.active_runs > 0 {
                KernelObjectState::Running
            } else {
                KernelObjectState::Registered
            },
            "global",
        )
        .with_metadata(json!({
            "stageCount": self.stages.len(),
            "requiredStageCount": self.required_stage_ids.len(),
            "activeRuns": stats.active_runs,
            "completedRuns": stats.completed_runs,
            "failedRuns": stats.failed_runs,
        }))];

        objects.extend(self.stage_ids.iter().map(|stage_id| {
            KernelObjectInfo::new(
                stage_id.clone(),
                "pipeline.stage",
                KernelObjectState::Registered,
                "global",
            )
        }));
        objects
    }

    pub async fn run(&self, context: &mut PipelineContext) -> KernelResult<()> {
        let stage_count = self.stages.len();
        let scope = context.context.scope_key_ref();
        let span = tracing::info_span!(
            "kernel.pipeline.run",
            trace_id = %context.context.trace_id,
            scope = %scope,
            source = %context.context.source,
            stage_count
        );
        let _entered = span.enter();

        let run_started_at = std::time::Instant::now();
        let run_lease = ResourceLease::new(context.context.trace_id.to_string())
            .with_owner(context.context.source.clone());
        self.counters.active_runs.fetch_add(1, Ordering::Relaxed);
        context.observe(ExecutionEventKind::RunStarted);
        tracing::debug!(
            lease_id = %run_lease.id,
            object_id = %run_lease.object_id,
            "pipeline run lease acquired"
        );

        let result = async {
            tracing::debug!("validating pipeline");
            self.validate_required_stages()?;
            context.ensure_active()?;

            Next {
                stages: &self.stages,
                stage_ids: &self.stage_ids,
                next_index: 0,
            }
            .call(context)
            .await
        }
        .await;

        self.counters.active_runs.fetch_sub(1, Ordering::Relaxed);
        let run_duration_ms = run_started_at.elapsed().as_millis() as u64;
        self.counters
            .total_duration_ms
            .fetch_add(run_duration_ms, Ordering::Relaxed);
        match &result {
            Ok(()) => {
                self.counters.completed_runs.fetch_add(1, Ordering::Relaxed);
                if context.observations.is_enabled() {
                    let event = context
                        .execution_event(ExecutionEventKind::RunCompleted)
                        .with_duration(run_duration_ms);
                    context.observations.observe(event);
                }
                tracing::info!("pipeline completed");
            }
            Err(error) => {
                self.counters.failed_runs.fetch_add(1, Ordering::Relaxed);
                let kind = if matches!(error.kind(), super::KernelErrorKind::Cancelled) {
                    ExecutionEventKind::RunCancelled
                } else {
                    ExecutionEventKind::RunFailed
                };
                if context.observations.is_enabled() {
                    let event = context
                        .execution_event(kind)
                        .with_error_kind(error.kind())
                        .with_duration(run_duration_ms)
                        .with_message(error.to_string());
                    context.observations.observe(event);
                }
                tracing::error!(error = %error, "pipeline failed");
            }
        }

        result
    }

    /// 在同步调用边界运行 Pipeline。
    ///
    /// 内核 Pipeline 的 Stage 是 async trait；桌面端仍有大量同步 API
    /// 边界。由内核统一提供 blocking runner，避免应用模块各自手写
    /// executor 或散落 `block_on`。如果当前线程已进入 Tokio runtime，
    /// 使用当前 [`tokio::runtime::Handle`]；否则临时创建 runtime。
    pub fn run_blocking(&self, context: &mut PipelineContext) -> KernelResult<()> {
        block_on_pipeline_future(self.run(context))
    }

    pub async fn run_with_retry(
        &self,
        context: &mut PipelineContext,
        policy: PipelineRetryPolicy,
    ) -> KernelResult<()> {
        self.run_with_retry_if(context, policy, KernelError::is_retryable)
            .await
    }

    pub async fn run_with_retry_if(
        &self,
        context: &mut PipelineContext,
        policy: PipelineRetryPolicy,
        mut should_retry: impl FnMut(&KernelError) -> bool,
    ) -> KernelResult<()> {
        let mut backoff = policy.backoff().build();

        loop {
            match self.run(context).await {
                Ok(()) => return Ok(()),
                Err(error) if should_retry(&error) => match backoff.next() {
                    Some(delay) => {
                        tracing::warn!(
                            error = %error,
                            retry_delay_ms = delay.as_millis() as u64,
                            "retrying pipeline"
                        );
                        tokio::time::sleep(delay).await;
                    }
                    None => return Err(error),
                },
                Err(error) => return Err(error),
            }
        }
    }

    /// 在同步调用边界运行带重试的 Pipeline。
    pub fn run_with_retry_blocking(
        &self,
        context: &mut PipelineContext,
        policy: PipelineRetryPolicy,
    ) -> KernelResult<()> {
        block_on_pipeline_future(self.run_with_retry(context, policy))
    }

    /// 在同步调用边界运行带条件重试的 Pipeline。
    pub fn run_with_retry_if_blocking(
        &self,
        context: &mut PipelineContext,
        policy: PipelineRetryPolicy,
        should_retry: impl FnMut(&KernelError) -> bool + Send,
    ) -> KernelResult<()> {
        block_on_pipeline_future(self.run_with_retry_if(context, policy, should_retry))
    }

    fn validate_required_stages(&self) -> KernelResult<()> {
        for required in &self.required_stage_ids {
            if !self.stage_id_set.contains(required) {
                return Err(KernelError::RequiredStageMissing {
                    id: required.clone(),
                });
            }
        }
        Ok(())
    }
}

fn block_on_pipeline_future<F>(future: F) -> KernelResult<()>
where
    F: std::future::Future<Output = KernelResult<()>> + Send,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => std::thread::scope(|scope| {
            scope
                .spawn(move || handle.block_on(future))
                .join()
                .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
        }),
        Err(_) => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| {
                KernelError::invalid_input(format!("failed to create pipeline runtime: {error}"))
            })?
            .block_on(future),
    }
}

impl KernelResource for Pipeline {
    fn object_info(&self) -> KernelObjectInfo {
        self.objects().into_iter().next().unwrap_or_else(|| {
            KernelObjectInfo::new("pipeline", "pipeline", KernelObjectState::Unknown, "global")
        })
    }

    fn active_leases(&self) -> usize {
        self.stats().active_runs
    }

    fn shutdown(&self, mode: ShutdownMode) -> KernelResult<()> {
        match mode {
            ShutdownMode::Immediate => Ok(()),
            ShutdownMode::Graceful | ShutdownMode::Deadline(_) if self.active_leases() == 0 => {
                Ok(())
            }
            ShutdownMode::Graceful | ShutdownMode::Deadline(_) => Err(KernelError::invalid_input(
                "pipeline has active resource leases",
            )),
        }
    }
}

pub struct Next<'a> {
    stages: &'a [Arc<dyn Stage>],
    stage_ids: &'a [String],
    next_index: usize,
}

impl<'a> Next<'a> {
    pub fn call(self, context: &'a mut PipelineContext) -> BoxFuture<'a, KernelResult<()>> {
        Box::pin(async move {
            context.ensure_active()?;

            if self.next_index >= self.stages.len() {
                return Ok(());
            }

            let stage = &self.stages[self.next_index];
            let stage_id = self.stage_ids[self.next_index].as_str();
            let span = tracing::debug_span!(
                "kernel.pipeline.stage",
                stage_id = %stage_id,
                next_index = self.next_index
            );
            let _entered = span.enter();
            let next = Next {
                stages: self.stages,
                stage_ids: self.stage_ids,
                next_index: self.next_index + 1,
            };

            let started_at = std::time::Instant::now();
            if context.observations.is_enabled() {
                let event = context
                    .execution_event(ExecutionEventKind::StageStarted)
                    .with_stage_id(stage_id);
                context.observations.observe(event);
            }

            let result = stage.process(context, next).await;
            let duration_ms = started_at.elapsed().as_millis() as u64;
            match result {
                Ok(()) => {
                    if context.observations.is_enabled() {
                        let event = context
                            .execution_event(ExecutionEventKind::StageCompleted)
                            .with_stage_id(stage_id)
                            .with_duration(duration_ms);
                        context.observations.observe(event);
                    }
                    tracing::debug!(duration_ms, "stage completed");
                    Ok(())
                }
                Err(error) => {
                    let kind = error.kind();
                    let message = error.to_string();
                    if context.observations.is_enabled() {
                        let event = context
                            .execution_event(ExecutionEventKind::StageFailed)
                            .with_stage_id(stage_id)
                            .with_error_kind(kind)
                            .with_duration(duration_ms)
                            .with_message(message.clone());
                        context.observations.observe(event);
                    }
                    Err(KernelError::StageFailed {
                        id: stage_id.to_string(),
                        kind,
                        message,
                    })
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::KernelScope;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Default)]
    struct CounterData {
        value: usize,
    }

    struct IncrementStage;

    #[async_trait]
    impl Stage for IncrementStage {
        fn id(&self) -> &str {
            "increment"
        }

        async fn process(&self, context: &mut PipelineContext, next: Next<'_>) -> KernelResult<()> {
            context.data_mut::<CounterData>()?.value += 1;
            next.call(context).await
        }
    }

    #[tokio::test]
    async fn stages_run_in_order() {
        let mut pipeline = Pipeline::new();
        pipeline.push(IncrementStage);
        pipeline.push(IncrementStage);

        let mut context = PipelineContext::new(
            CounterData::default(),
            KernelContext::new("test", KernelScope::global()),
        );

        pipeline.run(&mut context).await.unwrap();

        assert_eq!(context.data::<CounterData>().unwrap().value, 2);
    }

    #[tokio::test]
    async fn missing_required_stage_fails() {
        let pipeline = Pipeline::new().require_stage("required");
        let mut context = PipelineContext::new(
            CounterData::default(),
            KernelContext::new("test", KernelScope::global()),
        );

        let err = pipeline.run(&mut context).await.unwrap_err();
        assert!(matches!(err, KernelError::RequiredStageMissing { .. }));
    }

    #[tokio::test]
    async fn cancelled_context_fails_before_stage() {
        let mut pipeline = Pipeline::new();
        pipeline.push(IncrementStage);
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let mut context = PipelineContext::new(
            CounterData::default(),
            KernelContext::new("test", KernelScope::global()),
        )
        .with_cancellation(cancellation);

        let err = pipeline.run(&mut context).await.unwrap_err();
        assert!(matches!(err, KernelError::Cancelled));
    }

    #[tokio::test]
    async fn observation_sink_receives_stage_delta() {
        struct DeltaStage;

        #[async_trait]
        impl Stage for DeltaStage {
            fn id(&self) -> &str {
                "delta"
            }

            async fn process(
                &self,
                context: &mut PipelineContext,
                next: Next<'_>,
            ) -> KernelResult<()> {
                context.observe_stage_delta(self.id(), "running", json!({ "pct": 0.5 }));
                next.call(context).await
            }
        }

        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let events_clone = events.clone();
        let mut pipeline = Pipeline::new();
        pipeline.push(DeltaStage);
        let mut context =
            PipelineContext::new((), KernelContext::new("test", KernelScope::global()))
                .with_observer_fn(move |event| {
                    events_clone.lock().unwrap().push(event.kind);
                });

        pipeline.run(&mut context).await.unwrap();
        let events = events.lock().unwrap();
        assert!(events.contains(&ExecutionEventKind::RunStarted));
        assert!(events.contains(&ExecutionEventKind::StageStarted));
        assert!(events.contains(&ExecutionEventKind::StageDelta));
        assert!(events.contains(&ExecutionEventKind::StageCompleted));
        assert!(events.contains(&ExecutionEventKind::RunCompleted));
    }

    #[test]
    fn run_blocking_runs_pipeline_without_external_executor() {
        let mut pipeline = Pipeline::new();
        pipeline.push(IncrementStage);
        pipeline.push(IncrementStage);
        let mut context = PipelineContext::new(
            CounterData::default(),
            KernelContext::new("test", KernelScope::global()),
        );

        pipeline.run_blocking(&mut context).unwrap();

        assert_eq!(context.data::<CounterData>().unwrap().value, 2);
        let stats = pipeline.stats();
        assert_eq!(stats.completed_runs, 1);
        assert_eq!(stats.failed_runs, 0);
    }

    #[test]
    fn pipeline_exports_stats_and_objects() {
        let mut pipeline = Pipeline::new();
        pipeline.push(IncrementStage);

        let stats = pipeline.stats();
        assert_eq!(stats.active_runs, 0);
        assert_eq!(stats.total_runs, 0);

        let objects = pipeline.objects();
        assert!(objects.iter().any(|object| {
            object.id == "pipeline"
                && object.kind == "pipeline"
                && object.state == KernelObjectState::Registered
        }));
        assert!(objects.iter().any(|object| {
            object.id == "increment"
                && object.kind == "pipeline.stage"
                && object.state == KernelObjectState::Registered
        }));
        assert_eq!(pipeline.active_leases(), 0);
        assert!(pipeline.shutdown(ShutdownMode::Graceful).is_ok());
    }

    #[test]
    fn observation_fn_adapter_is_invoked() {
        struct DeltaStage;

        #[async_trait]
        impl Stage for DeltaStage {
            fn id(&self) -> &str {
                "delta"
            }

            async fn process(
                &self,
                context: &mut PipelineContext,
                next: Next<'_>,
            ) -> KernelResult<()> {
                context.observe_stage_delta(self.id(), "running", Value::Null);
                next.call(context).await
            }
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        let mut pipeline = Pipeline::new();
        pipeline.push(DeltaStage);
        let mut context =
            PipelineContext::new((), KernelContext::new("test", KernelScope::global()))
                .with_observer_fn(move |event| {
                    if event.kind == ExecutionEventKind::StageDelta {
                        counter_clone.fetch_add(1, Ordering::SeqCst);
                    }
                });

        pipeline.run_blocking(&mut context).unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn failing_stage_emits_failure_observations() {
        struct FailingStage;

        #[async_trait]
        impl Stage for FailingStage {
            fn id(&self) -> &str {
                "failing"
            }

            async fn process(
                &self,
                _context: &mut PipelineContext,
                _next: Next<'_>,
            ) -> KernelResult<()> {
                Err(KernelError::transient("temporary failure"))
            }
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        let mut pipeline = Pipeline::new();
        pipeline.push(FailingStage);
        let mut context =
            PipelineContext::new((), KernelContext::new("test", KernelScope::global()))
                .with_observer_fn(move |event| {
                    if matches!(
                        event.kind,
                        ExecutionEventKind::StageFailed | ExecutionEventKind::RunFailed
                    ) {
                        counter_clone.fetch_add(1, Ordering::SeqCst);
                    }
                });

        let error = pipeline.run(&mut context).await.unwrap_err();
        assert!(matches!(error, KernelError::StageFailed { .. }));
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn retry_runs_stage_again_after_retryable_failure() {
        struct FailsOnceStage {
            attempts: AtomicUsize,
        }

        #[async_trait]
        impl Stage for FailsOnceStage {
            fn id(&self) -> &str {
                "fails-once"
            }

            async fn process(
                &self,
                context: &mut PipelineContext,
                next: Next<'_>,
            ) -> KernelResult<()> {
                if self.attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                    return Err(KernelError::transient("temporary failure"));
                }

                context.data_mut::<CounterData>()?.value += 1;
                next.call(context).await
            }
        }

        let mut pipeline = Pipeline::new();
        pipeline.push(FailsOnceStage {
            attempts: AtomicUsize::new(0),
        });
        let mut context = PipelineContext::new(
            CounterData::default(),
            KernelContext::new("test", KernelScope::global()),
        );

        pipeline
            .run_with_retry(
                &mut context,
                PipelineRetryPolicy::new(1)
                    .with_min_delay(Duration::ZERO)
                    .with_max_delay(Duration::ZERO)
                    .with_jitter(false),
            )
            .await
            .unwrap();

        assert_eq!(context.data::<CounterData>().unwrap().value, 1);
    }

    #[test]
    fn run_with_retry_blocking_supports_tokio_timer() {
        struct FailsOnceStage {
            attempts: AtomicUsize,
        }

        #[async_trait]
        impl Stage for FailsOnceStage {
            fn id(&self) -> &str {
                "fails-once-blocking"
            }

            async fn process(
                &self,
                context: &mut PipelineContext,
                next: Next<'_>,
            ) -> KernelResult<()> {
                if self.attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                    return Err(KernelError::transient("temporary failure"));
                }

                context.data_mut::<CounterData>()?.value += 1;
                next.call(context).await
            }
        }

        let mut pipeline = Pipeline::new();
        pipeline.push(FailsOnceStage {
            attempts: AtomicUsize::new(0),
        });
        let mut context = PipelineContext::new(
            CounterData::default(),
            KernelContext::new("test", KernelScope::global()),
        );

        pipeline
            .run_with_retry_blocking(
                &mut context,
                PipelineRetryPolicy::new(1)
                    .with_min_delay(Duration::from_millis(1))
                    .with_max_delay(Duration::from_millis(1))
                    .with_jitter(false),
            )
            .unwrap();

        assert_eq!(context.data::<CounterData>().unwrap().value, 1);
    }
}
