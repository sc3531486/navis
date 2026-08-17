use chrono::{DateTime, Utc};
use flume::{Receiver, Sender};
use futures_util::future::{BoxFuture, FutureExt};
use glob::Pattern;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use tokio::runtime::Handle;
use triomphe::Arc as SharedArc;

use super::{
    KernelContext, KernelObjectInfo, KernelObjectState, KernelResource, KernelResult,
    ResourceLease, SchemaVersion, ShutdownMode, SubscriptionId, Topic,
};

pub type SharedEventEnvelope = SharedArc<EventEnvelope>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub id: String,
    pub topic: Topic,
    pub version: SchemaVersion,
    pub context: KernelContext,
    pub payload: Option<SharedArc<Value>>,
    pub created_at: DateTime<Utc>,
}

impl EventEnvelope {
    pub fn new(
        topic: impl Into<Topic>,
        context: KernelContext,
        payload: Option<SharedArc<Value>>,
    ) -> Self {
        Self {
            id: uuid::Uuid::now_v7().to_string(),
            topic: topic.into(),
            version: SchemaVersion::default(),
            context,
            payload,
            created_at: Utc::now(),
        }
    }

    pub fn scope_key(&self) -> String {
        self.context.scope_key()
    }

    pub fn scope_key_ref(&self) -> &str {
        self.context.scope_key_ref()
    }
}

pub type EventHandler = Arc<dyn Fn(&EventEnvelope) + Send + Sync>;
pub type AsyncEventHandler =
    Arc<dyn Fn(SharedEventEnvelope) -> BoxFuture<'static, ()> + Send + Sync>;
type EventSubscriptionSnapshot = Arc<Vec<EventSubscription>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventBusStats {
    pub subscription_count: usize,
    pub history_len: usize,
    pub history_capacity: usize,
    pub queue_len: usize,
    pub queue_capacity: usize,
    pub dispatcher_running: bool,
    pub overflow_count: u64,
}

impl EventBusStats {
    /// Returns `true` if any events were dropped from the dispatcher queue.
    pub fn has_overflow(&self) -> bool {
        self.overflow_count > 0
    }
}

#[derive(Clone)]
enum EventHandlerKind {
    Sync(EventHandler),
    Async(AsyncEventHandler),
}

#[derive(Clone)]
struct EventSubscriptionWorker {
    tx: Sender<SharedEventEnvelope>,
}

#[derive(Clone)]
pub struct EventSubscription {
    pub id: SubscriptionId,
    pub topic: Option<Topic>,
    pub scope_key: Option<String>,
    pub lease: ResourceLease,
    worker: EventSubscriptionWorker,
    topic_pattern: Option<Pattern>,
}

struct DispatcherState {
    tx: Option<Sender<SharedEventEnvelope>>,
    handle: Option<JoinHandle<()>>,
    closed: bool,
}

/// Kernel event bus abstraction.
///
/// Implementations dispatch only events emitted after a subscription is
/// registered. Subscriptions do not replay historical events. Callers that
/// need a catch-up snapshot must query [`EventBus::recent`] explicitly and
/// reconcile it with live events at the application boundary.
pub trait EventBus: Send + Sync {
    fn emit(&self, envelope: EventEnvelope) -> KernelResult<()>;
    /// Subscribe to matching future events with a synchronous handler.
    ///
    /// A single subscription must observe matching events in emit order.
    /// Different subscriptions may be dispatched concurrently.
    fn subscribe(
        &self,
        topic: Option<Topic>,
        scope_key: Option<String>,
        handler: EventHandler,
    ) -> KernelResult<SubscriptionId>;
    /// Subscribe to matching future events with an async handler.
    ///
    /// A single subscription must observe matching events in emit order.
    /// Different subscriptions may be dispatched concurrently.
    fn subscribe_async(
        &self,
        topic: Option<Topic>,
        scope_key: Option<String>,
        handler: AsyncEventHandler,
    ) -> KernelResult<SubscriptionId>;
    fn unsubscribe(&self, id: &SubscriptionId) -> KernelResult<()>;
    fn recent(&self, limit: usize) -> Vec<SharedEventEnvelope>;
    fn stats(&self) -> EventBusStats;
    fn objects(&self) -> Vec<KernelObjectInfo>;
}

/// In-memory event bus with ordered async handler dispatch.
///
/// # Construction
///
/// `new()` requires an explicit [`tokio::runtime::Handle`].  The handle is used
/// by `subscribe()` to run sync handlers on the Tokio blocking thread pool
/// via [`Handle::spawn_blocking`], and by `subscribe_async()` to run async
/// handlers via [`Handle::spawn`]. Each subscription owns a serial worker
/// queue, so one subscriber observes matching events in emit order. Different
/// subscriptions may still run concurrently. The caller must ensure the
/// runtime behind the handle remains alive for the bus's lifetime.
///
/// # Dispatcher lifecycle
///
/// The dispatcher OS thread starts lazily on the first `subscribe()` call.
/// `emit()` always writes to the history ring-buffer; it only enqueues to the
/// dispatcher channel once the dispatcher is running. Subscriptions do not
/// replay history. Callers that need past events must query [`EventBus::recent`]
/// explicitly.
///
/// # Shutdown
///
/// `shutdown()` drops the internal channel and joins the dispatcher thread.
/// **It does not wait for in-flight subscription worker tasks to complete.**
/// Those tasks run to completion (or panic) independently. Callers that need
/// handler completion guarantees should implement their own synchronisation
/// inside the handler.
pub struct InMemoryEventBus {
    subscriptions: Arc<RwLock<HashMap<SubscriptionId, EventSubscription>>>,
    subscription_snapshot: Arc<RwLock<EventSubscriptionSnapshot>>,
    runtime: Handle,
    dispatcher: Mutex<DispatcherState>,
    history: Mutex<VecDeque<SharedEventEnvelope>>,
    history_capacity: usize,
    overflow_count: AtomicU64,
}

impl InMemoryEventBus {
    const EVENT_QUEUE_CAPACITY: usize = 4096;

    /// Create a new event bus backed by the given Tokio runtime handle.
    ///
    /// The dispatcher thread is **not** started yet — it starts lazily on the
    /// first `subscribe()` call.
    pub fn new(history_capacity: usize, runtime: Handle) -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            subscription_snapshot: Arc::new(RwLock::new(Arc::new(Vec::new()))),
            runtime,
            dispatcher: Mutex::new(DispatcherState {
                tx: None,
                handle: None,
                closed: false,
            }),
            history: Mutex::new(VecDeque::with_capacity(history_capacity)),
            history_capacity,
            overflow_count: AtomicU64::new(0),
        }
    }

    /// Start the dispatcher thread if it hasn't been started yet.
    fn ensure_dispatcher(
        dispatcher: &mut DispatcherState,
        snapshot: Arc<RwLock<EventSubscriptionSnapshot>>,
        runtime: Handle,
    ) {
        if dispatcher.closed {
            return;
        }
        if dispatcher.tx.is_some() {
            return;
        }

        let (tx, rx) = flume::bounded(Self::EVENT_QUEUE_CAPACITY);
        let handle = Self::spawn_dispatcher(snapshot, rx, runtime);
        dispatcher.tx = Some(tx);
        dispatcher.handle = Some(handle);

        tracing::debug!("event dispatcher started");
    }

    fn spawn_dispatcher(
        subscription_snapshot: Arc<RwLock<EventSubscriptionSnapshot>>,
        rx: Receiver<SharedEventEnvelope>,
        _runtime: Handle,
    ) -> JoinHandle<()> {
        std::thread::Builder::new()
            .name("kernel-event-dispatcher".into())
            .spawn(move || {
                while let Ok(envelope) = rx.recv() {
                    let subscriptions = subscription_snapshot.read().clone();

                    for subscription in subscriptions.iter() {
                        if Self::matches(subscription, &envelope) {
                            if let Err(error) = subscription.worker.tx.send(envelope.clone()) {
                                tracing::warn!(
                                    subscription_id = %subscription.id,
                                    event_id = %envelope.id,
                                    error = %error,
                                    "event subscription worker is closed"
                                );
                            }
                        }
                    }
                }
            })
            .expect("failed to spawn kernel-event-dispatcher thread")
    }

    /// Shut down the dispatcher thread.
    ///
    /// Drops the internal channel and joins the dispatcher OS thread so that no
    /// new `spawn_blocking` tasks will be submitted after this call returns.
    ///
    /// **Note:** handler tasks already submitted to `spawn_blocking` are *not*
    /// cancelled — they run to completion (or panic) on the Tokio blocking
    /// pool independently.
    pub fn shutdown(&self) {
        let mut dispatcher = self.dispatcher.lock();
        dispatcher.closed = true;
        dispatcher.tx = None;
        if let Some(handle) = dispatcher.handle.take() {
            let _ = handle.join();
        }
    }

    fn refresh_subscription_snapshot(
        &self,
        subscriptions: &HashMap<SubscriptionId, EventSubscription>,
    ) {
        *self.subscription_snapshot.write() =
            Arc::new(subscriptions.values().cloned().collect::<Vec<_>>());
    }

    fn matches(subscription: &EventSubscription, envelope: &EventEnvelope) -> bool {
        if let Some(pattern) = &subscription.topic_pattern {
            if !pattern.matches(envelope.topic.as_str()) {
                return false;
            }
        }

        if let Some(scope_key) = &subscription.scope_key {
            if scope_key != envelope.scope_key_ref() {
                return false;
            }
        }

        true
    }

    fn subscribe_with_handler(
        &self,
        topic: Option<Topic>,
        scope_key: Option<String>,
        handler: EventHandlerKind,
    ) -> KernelResult<SubscriptionId> {
        let id = SubscriptionId::generate();
        tracing::debug!(
            subscription_id = %id,
            topic = topic.as_ref().map(|topic| topic.as_str()),
            scope = scope_key.as_deref(),
            "creating event subscription"
        );
        let topic_pattern = topic
            .as_ref()
            .map(|topic| {
                Pattern::new(topic.as_str()).map_err(|error| {
                    super::KernelError::invalid_input(format!(
                        "invalid event topic pattern '{}': {error}",
                        topic.as_str()
                    ))
                })
            })
            .transpose()?;
        let worker = Self::spawn_subscription_worker(id.clone(), handler, self.runtime.clone());
        let subscription = EventSubscription {
            id: id.clone(),
            topic,
            scope_key,
            lease: ResourceLease::new(id.to_string()).with_owner("event.subscription"),
            worker,
            topic_pattern,
        };
        let mut dispatcher = self.dispatcher.lock();
        Self::ensure_dispatcher(
            &mut dispatcher,
            Arc::clone(&self.subscription_snapshot),
            self.runtime.clone(),
        );
        if dispatcher.closed {
            return Err(super::KernelError::invalid_input(
                "event bus is already shut down",
            ));
        }
        {
            let mut subscriptions = self.subscriptions.write();
            subscriptions.insert(id.clone(), subscription);
            self.refresh_subscription_snapshot(&subscriptions);
        }
        Ok(id)
    }

    fn spawn_subscription_worker(
        subscription_id: SubscriptionId,
        handler: EventHandlerKind,
        runtime: Handle,
    ) -> EventSubscriptionWorker {
        let (tx, rx) = flume::unbounded::<SharedEventEnvelope>();
        match handler {
            EventHandlerKind::Sync(handler) => {
                runtime.spawn(async move {
                    while let Ok(envelope) = rx.recv_async().await {
                        tracing::trace!(
                            subscription_id = %subscription_id,
                            event_id = %envelope.id,
                            "running sync event handler"
                        );
                        let event_id = envelope.id.clone();
                        let subscription_id_for_task = subscription_id.clone();
                        let handler = handler.clone();
                        let result = tokio::task::spawn_blocking(move || {
                            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                handler(&envelope)
                            }))
                        })
                        .await;

                        if !matches!(result, Ok(Ok(()))) {
                            tracing::error!(
                                subscription_id = %subscription_id_for_task,
                                event_id = %event_id,
                                "sync event handler panicked"
                            );
                        }
                    }
                });
            }
            EventHandlerKind::Async(handler) => {
                runtime.spawn(async move {
                    while let Ok(envelope) = rx.recv_async().await {
                        tracing::trace!(
                            subscription_id = %subscription_id,
                            event_id = %envelope.id,
                            "running async event handler"
                        );
                        let result = std::panic::AssertUnwindSafe(handler(envelope.clone()))
                            .catch_unwind()
                            .await;

                        if result.is_err() {
                            tracing::error!(
                                subscription_id = %subscription_id,
                                event_id = %envelope.id,
                                "async event handler panicked"
                            );
                        }
                    }
                });
            }
        }
        EventSubscriptionWorker { tx }
    }
}

impl EventBus for InMemoryEventBus {
    fn emit(&self, envelope: EventEnvelope) -> KernelResult<()> {
        let envelope = SharedArc::new(envelope);
        let tx = {
            let dispatcher = self.dispatcher.lock();
            if dispatcher.closed {
                return Err(super::KernelError::invalid_input(
                    "event bus is already shut down",
                ));
            }
            dispatcher.tx.clone()
        };
        tracing::debug!(
            event_id = %envelope.id,
            topic = %envelope.topic,
            scope = %envelope.scope_key_ref(),
            version = %envelope.version,
            "emitting event envelope"
        );

        {
            let mut history = self.history.lock();
            history.push_back(envelope.clone());
            if history.len() > self.history_capacity {
                let overflow = history.len() - self.history_capacity;
                history.drain(..overflow);
            }
        }

        if let Some(tx) = tx {
            match tx.send(envelope.clone()) {
                Ok(()) => {}
                Err(_) => {
                    return Err(super::KernelError::invalid_input(
                        "event dispatcher is disconnected",
                    ));
                }
            }
        }

        tracing::debug!(
            event_id = %envelope.id,
            "event envelope enqueued"
        );
        Ok(())
    }

    fn subscribe(
        &self,
        topic: Option<Topic>,
        scope_key: Option<String>,
        handler: EventHandler,
    ) -> KernelResult<SubscriptionId> {
        self.subscribe_with_handler(topic, scope_key, EventHandlerKind::Sync(handler))
    }

    fn subscribe_async(
        &self,
        topic: Option<Topic>,
        scope_key: Option<String>,
        handler: AsyncEventHandler,
    ) -> KernelResult<SubscriptionId> {
        self.subscribe_with_handler(topic, scope_key, EventHandlerKind::Async(handler))
    }

    fn unsubscribe(&self, id: &SubscriptionId) -> KernelResult<()> {
        tracing::debug!(subscription_id = %id, "removing event subscription");
        let mut subscriptions = self.subscriptions.write();
        subscriptions
            .remove(id)
            .map(|_| {
                self.refresh_subscription_snapshot(&subscriptions);
            })
            .ok_or_else(|| super::KernelError::EventSubscriptionNotFound { id: id.to_string() })
    }

    fn recent(&self, limit: usize) -> Vec<SharedEventEnvelope> {
        let history = self.history.lock();
        let start = history.len().saturating_sub(limit);
        history.iter().skip(start).cloned().collect()
    }

    fn stats(&self) -> EventBusStats {
        let dispatcher = self.dispatcher.lock();
        EventBusStats {
            subscription_count: self.subscriptions.read().len(),
            history_len: self.history.lock().len(),
            history_capacity: self.history_capacity,
            queue_len: dispatcher.tx.as_ref().map(Sender::len).unwrap_or(0),
            queue_capacity: dispatcher
                .tx
                .as_ref()
                .and_then(Sender::capacity)
                .unwrap_or(0),
            dispatcher_running: dispatcher.tx.is_some(),
            overflow_count: self.overflow_count.load(Ordering::Relaxed),
        }
    }

    fn objects(&self) -> Vec<KernelObjectInfo> {
        let mut objects = self
            .subscriptions
            .read()
            .values()
            .map(|subscription| {
                KernelObjectInfo::new(
                    subscription.id.to_string(),
                    "event.subscription",
                    KernelObjectState::Enabled,
                    subscription
                        .scope_key
                        .clone()
                        .unwrap_or_else(|| "global".to_string()),
                )
                .with_metadata(json!({
                    "topic": subscription.topic.as_ref().map(|topic| topic.as_str()),
                    "scope": subscription.scope_key,
                    "leaseId": subscription.lease.id.clone(),
                }))
            })
            .collect::<Vec<_>>();

        let dispatcher_running = self.dispatcher.lock().tx.is_some();
        objects.push(KernelObjectInfo::new(
            "event.dispatcher",
            "event.dispatcher",
            if dispatcher_running {
                KernelObjectState::Running
            } else {
                KernelObjectState::Registered
            },
            "global",
        ));
        objects
    }
}

impl KernelResource for InMemoryEventBus {
    fn object_info(&self) -> KernelObjectInfo {
        let stats = self.stats();
        KernelObjectInfo::new(
            "event.bus",
            "event.bus",
            if stats.dispatcher_running {
                KernelObjectState::Running
            } else {
                KernelObjectState::Registered
            },
            "global",
        )
        .with_metadata(json!({
            "subscriptionCount": stats.subscription_count,
            "historyLen": stats.history_len,
            "queueLen": stats.queue_len,
        }))
    }

    fn active_leases(&self) -> usize {
        self.stats().subscription_count
    }

    fn shutdown(&self, _mode: ShutdownMode) -> KernelResult<()> {
        InMemoryEventBus::shutdown(self);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::KernelScope;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};
    use triomphe::Arc as SharedArc;

    async fn wait_for_count_async(counter: &AtomicUsize, expected: usize) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if counter.load(Ordering::SeqCst) == expected {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    fn test_bus() -> InMemoryEventBus {
        InMemoryEventBus::new(1000, Handle::current())
    }

    #[tokio::test]
    async fn scoped_subscription_only_receives_matching_scope() {
        let bus = test_bus();
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_clone = hits.clone();
        bus.subscribe(
            Some(Topic::from("state.changed")),
            Some("scope:a".to_string()),
            Arc::new(move |_| {
                hits_clone.fetch_add(1, Ordering::SeqCst);
            }),
        )
        .unwrap();

        bus.emit(EventEnvelope::new(
            "state.changed",
            KernelContext::new("test", KernelScope::scoped("scope", "a")),
            None,
        ))
        .unwrap();
        bus.emit(EventEnvelope::new(
            "state.changed",
            KernelContext::new("test", KernelScope::scoped("scope", "b")),
            None,
        ))
        .unwrap();

        wait_for_count_async(&hits, 1).await;
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn emit_without_subscribers_only_writes_history() {
        let bus = test_bus();
        bus.emit(EventEnvelope::new(
            "system.ready",
            KernelContext::new("test", KernelScope::global()),
            None,
        ))
        .unwrap();
        assert_eq!(bus.recent(10).len(), 1);
        let stats = bus.stats();
        assert_eq!(stats.history_len, 1);
        assert_eq!(stats.subscription_count, 0);
        assert!(!stats.dispatcher_running);
    }

    #[tokio::test]
    async fn recent_returns_oldest_to_newest_limited_tail() {
        let bus = InMemoryEventBus::new(4, Handle::current());
        for index in 0..6 {
            bus.emit(EventEnvelope::new(
                format!("event.{index}"),
                KernelContext::new("test", KernelScope::global()),
                Some(SharedArc::new(json!({ "index": index }))),
            ))
            .unwrap();
        }

        let recent = bus.recent(2);
        let topics = recent
            .iter()
            .map(|envelope| envelope.topic.as_str())
            .collect::<Vec<_>>();
        assert_eq!(topics, vec!["event.4", "event.5"]);

        let all_recent = bus.recent(10);
        let topics = all_recent
            .iter()
            .map(|envelope| envelope.topic.as_str())
            .collect::<Vec<_>>();
        assert_eq!(topics, vec!["event.2", "event.3", "event.4", "event.5"]);

        assert!(bus.recent(0).is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn subscription_handlers_observe_events_in_emit_order() {
        let bus = test_bus();
        let observed = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let observed_clone = observed.clone();

        bus.subscribe(
            Some(Topic::from("ordered.changed")),
            None,
            Arc::new(move |envelope| {
                let index = envelope
                    .payload
                    .as_ref()
                    .and_then(|payload| payload.get("index"))
                    .and_then(Value::as_u64)
                    .unwrap();
                if index == 0 {
                    std::thread::sleep(Duration::from_millis(50));
                }
                observed_clone.lock().push(index);
            }),
        )
        .unwrap();

        for index in 0..3 {
            bus.emit(EventEnvelope::new(
                "ordered.changed",
                KernelContext::new("test", KernelScope::global()),
                Some(SharedArc::new(json!({ "index": index }))),
            ))
            .unwrap();
        }

        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if observed.lock().len() == 3 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        assert_eq!(*observed.lock(), vec![0, 1, 2]);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn dispatcher_state_is_consistent_across_start_and_shutdown() {
        let bus = Arc::new(test_bus());
        let mut tasks = Vec::new();

        for _ in 0..32 {
            let bus = bus.clone();
            tasks.push(tokio::spawn(async move {
                for _ in 0..16 {
                    let _ = bus.subscribe(None, None, Arc::new(|_| {}));
                    let dispatcher = bus.dispatcher.lock();
                    assert_eq!(dispatcher.tx.is_some(), dispatcher.handle.is_some());
                }
            }));
        }

        for task in tasks {
            task.await.unwrap();
        }

        {
            let dispatcher = bus.dispatcher.lock();
            assert!(dispatcher.tx.is_some());
            assert!(dispatcher.handle.is_some());
        }

        bus.shutdown();

        {
            let dispatcher = bus.dispatcher.lock();
            assert!(dispatcher.tx.is_none());
            assert!(dispatcher.handle.is_none());
        }

        assert!(!bus.stats().dispatcher_running);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn async_subscription_receives_matching_event() {
        let bus = test_bus();
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_clone = hits.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let tx = Arc::new(parking_lot::Mutex::new(Some(tx)));
        bus.subscribe_async(
            Some(Topic::from("async.changed")),
            None,
            Arc::new(move |envelope| {
                let hits = hits_clone.clone();
                let tx = tx.clone();
                async move {
                    assert_eq!(envelope.topic.as_str(), "async.changed");
                    hits.fetch_add(1, Ordering::SeqCst);
                    if let Some(tx) = tx.lock().take() {
                        let _ = tx.send(());
                    }
                }
                .boxed()
            }),
        )
        .unwrap();

        bus.emit(EventEnvelope::new(
            "async.changed",
            KernelContext::new("test", KernelScope::global()),
            None,
        ))
        .unwrap();

        tokio::time::timeout(Duration::from_secs(2), rx)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn unsubscribe_stops_future_event_delivery_and_reports_missing_id() {
        let bus = test_bus();
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_clone = hits.clone();
        let id = bus
            .subscribe(
                Some(Topic::from("state.changed")),
                None,
                Arc::new(move |_| {
                    hits_clone.fetch_add(1, Ordering::SeqCst);
                }),
            )
            .unwrap();

        bus.emit(EventEnvelope::new(
            "state.changed",
            KernelContext::new("test", KernelScope::global()),
            None,
        ))
        .unwrap();
        wait_for_count_async(&hits, 1).await;
        assert_eq!(hits.load(Ordering::SeqCst), 1);

        assert_eq!(bus.stats().subscription_count, 1);
        bus.unsubscribe(&id).unwrap();
        assert_eq!(bus.stats().subscription_count, 0);

        bus.emit(EventEnvelope::new(
            "state.changed",
            KernelContext::new("test", KernelScope::global()),
            None,
        ))
        .unwrap();
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert_eq!(hits.load(Ordering::SeqCst), 1);

        let err = bus.unsubscribe(&id).unwrap_err();
        assert!(matches!(
            err,
            super::super::KernelError::EventSubscriptionNotFound { .. }
        ));
    }

    #[tokio::test]
    async fn event_bus_exports_kernel_object_info() {
        let bus = test_bus();
        let id = bus
            .subscribe(
                Some(Topic::from("state.changed")),
                Some("scope:a".to_string()),
                Arc::new(|_| {}),
            )
            .unwrap();

        let objects = bus.objects();
        assert!(objects.iter().any(|object| {
            object.id == id.as_str()
                && object.kind == "event.subscription"
                && object.state == KernelObjectState::Enabled
                && object.scope == "scope:a"
        }));
        assert!(objects.iter().any(|object| {
            object.id == "event.dispatcher"
                && object.kind == "event.dispatcher"
                && object.state == KernelObjectState::Running
        }));
        assert_eq!(bus.active_leases(), 1);
    }

    #[test]
    fn topic_wildcard_matches_expected_topics() {
        assert!(Pattern::new("action.*")
            .unwrap()
            .matches("action.completed"));
        assert!(Pattern::new("*.completed")
            .unwrap()
            .matches("action.completed"));
        assert!(Pattern::new("action.*.done")
            .unwrap()
            .matches("action.item.done"));
        assert!(!Pattern::new("action.*")
            .unwrap()
            .matches("reasoning.completed"));
        assert!(!Pattern::new("action.completed")
            .unwrap()
            .matches("action.failed"));
    }
}
