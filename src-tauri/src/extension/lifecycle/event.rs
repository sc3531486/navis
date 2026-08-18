//! Extension 事件订阅能力。
//!
//! 本模块只定义 Extension lifecycle 需要的事件订阅 port、Kernel EventBus
//! 适配器和 Extension-owned subscription ledger。Extension lifecycle 不直接
//! 依赖具体 EventBus 实现；所有订阅句柄都由 Extension ledger 记录并负责
//! 在 disable/uninstall 前撤销。

use std::collections::HashMap;
use std::sync::Arc;

use futures_util::future::BoxFuture;
use parking_lot::Mutex;

use crate::extension::models::{EventSubscriptionRegistration, ExtensionEventDto};
use crate::kernel::{EventBus, EventEnvelope, EventHandler, SharedEventEnvelope, SubscriptionId, Topic};

/// 将 Kernel event 转换为 Extension runtime 的稳定 DTO。
impl From<&EventEnvelope> for ExtensionEventDto {
    fn from(event: &EventEnvelope) -> Self {
        Self {
            id: event.id.clone(),
            topic: event.topic.as_str().to_string(),
            version: event.version,
            scope_key: event.context.scope_key(),
            source: event.context.source.clone(),
            payload: event.payload.as_ref().map(|payload| (**payload).clone()),
            created_at: event.created_at,
        }
    }
}

/// Navis 瀑布事件域（设计 38 §2.2）。
///
/// 这些是事件模式常量，供 Agent 编排与工具流水线的扩展订阅使用。本模块
/// 只定义模式契约，不负责接线到 Agent 编排（C4 范围）。注意 `agent/*`
/// 前缀属于高频模式，扩展订阅必须走 stream 通道（见 `is_high_frequency_pattern`）；
/// `tools/*` 与 `llm/*` 是低频域，可经 Kernel EventBus 低延迟订阅。
///
/// 等待 Agent 扩展订阅接线（C4 范围），当前仅作模式契约。
#[allow(dead_code)]
pub mod waterfall {
    pub const AGENT_PRE_STEP: &str = "agent/pre-step";
    pub const LLM_STREAM: &str = "llm/stream";
    pub const TOOLS_PRE_EXECUTE: &str = "tools/pre-execute";
    pub const TOOLS_EXECUTE: &str = "tools/execute";
    pub const TOOLS_POST_EXECUTE: &str = "tools/post-execute";
    pub const AGENT_TURN_STOPPING: &str = "agent/turn-stopping";

    /// 全部瀑布事件模式；供验证与文档引用。
    pub const ALL: [&str; 6] = [
        AGENT_PRE_STEP,
        LLM_STREAM,
        TOOLS_PRE_EXECUTE,
        TOOLS_EXECUTE,
        TOOLS_POST_EXECUTE,
        AGENT_TURN_STOPPING,
    ];
}

/// 声明式事件订阅的投影输入（38 §2.2 升级后）。
///
/// 由 manifest `contributes.eventSubscriptions`（`EventSubscriptionRegistration`）
/// 投影而来；`handler` 引用在 Extension runtime 落地前不参与本层注册，只把
/// topic/scope 注册到 Kernel EventBus。
///
/// enable 接线由 Cordis 迁移任务在 state.rs 完成，当前仅提供能力。
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredEventSubscription {
    /// Extension 内唯一订阅 ID（仅用于诊断）。
    pub id: String,
    /// Kernel EventBus topic，支持精确或 glob 通配（如 `session.*`）。
    pub topic: String,
    /// 可选的 Kernel scope key。
    pub scope_key: Option<String>,
}

#[allow(dead_code)]
impl From<&EventSubscriptionRegistration> for DeclaredEventSubscription {
    fn from(registration: &EventSubscriptionRegistration) -> Self {
        Self {
            id: registration.id.clone(),
            topic: registration.topic.clone(),
            scope_key: registration.scope_key.clone(),
        }
    }
}

/// Extension runtime 可使用的事件处理器。
///
/// 该类型只在真正的 Extension runtime handler 入口落地后创建。manifest
/// 中的 handler DTO 不能直接转换为此类型。
pub type ExtensionSyncEventHandler = Arc<dyn Fn(ExtensionEventDto) + Send + Sync>;
pub type ExtensionAsyncEventHandler =
    Arc<dyn Fn(ExtensionEventDto) -> BoxFuture<'static, ()> + Send + Sync>;

/// Extension runtime handler；只接收稳定的 ExtensionEventDto。
///
/// Kernel EventEnvelope 到该 DTO 的转换只发生在 composition adapter 内部，
/// 不泄漏到 Extension lifecycle 或未来的 runtime。
pub enum ExtensionEventHandler {
    Sync(ExtensionSyncEventHandler),
    Async(ExtensionAsyncEventHandler),
}

/// Extension 事件订阅能力端口。
///
/// lifecycle 只依赖这个业务端口，不依赖 Kernel EventBus。具体的 runtime
/// handler 解析、权限检查和 handler 生命周期由上层 runtime 在调用前完成。
pub trait EventSubscriptionPort: Send + Sync {
    fn subscribe_extension(
        &self,
        extension_id: &str,
        topic: String,
        scope_key: Option<String>,
        handler: ExtensionEventHandler,
    ) -> anyhow::Result<SubscriptionId>;

    fn unsubscribe_extension(
        &self,
        extension_id: &str,
        subscription_id: &SubscriptionId,
    ) -> anyhow::Result<()>;
}

/// Extension-owned 的订阅事实。
///
/// `subscription_id` 是 Kernel EventBus 返回的 opaque 句柄；topic 和 scope
/// 只用于诊断与审计，不用于 disable 时重新推导订阅。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionSubscriptionRecord {
    pub subscription_id: SubscriptionId,
    pub topic: Topic,
    pub scope_key: Option<String>,
}

/// Extension-owned subscription ledger。
///
/// 这是订阅资源的唯一事实源。它只记录已经由 EventSubscriptionPort 成功
/// 注册的句柄，不记录仅存在于 manifest 中的声明。
#[derive(Debug, Default)]
pub struct ExtensionSubscriptionLedger {
    subscriptions: HashMap<String, Vec<ExtensionSubscriptionRecord>>,
}

impl ExtensionSubscriptionLedger {
    #[cfg(test)]
    pub fn record_many(
        &mut self,
        extension_id: &str,
        records: &[ExtensionSubscriptionRecord],
    ) -> anyhow::Result<()> {
        if extension_id.trim().is_empty() {
            return Err(anyhow::anyhow!("Extension ID must not be empty"));
        }
        if records.is_empty() {
            return Ok(());
        }
        let mut ids = std::collections::HashSet::new();
        for record in records {
            if !ids.insert(record.subscription_id.clone()) {
                return Err(anyhow::anyhow!(
                    "Event subscription '{}' is duplicated in the registration batch",
                    record.subscription_id
                ));
            }
            if self
                .subscriptions
                .values()
                .flatten()
                .any(|item| item.subscription_id == record.subscription_id)
            {
                return Err(anyhow::anyhow!(
                    "Event subscription '{}' is already owned",
                    record.subscription_id
                ));
            }
        }
        self.subscriptions
            .entry(extension_id.to_string())
            .or_default()
            .extend(records.iter().cloned());
        Ok(())
    }

    pub fn records(&self, extension_id: &str) -> Vec<ExtensionSubscriptionRecord> {
        self.subscriptions
            .get(extension_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn remove(&mut self, extension_id: &str, subscription_id: &SubscriptionId) -> bool {
        let Some(records) = self.subscriptions.get_mut(extension_id) else {
            return false;
        };
        let before = records.len();
        records.retain(|record| &record.subscription_id != subscription_id);
        let removed = records.len() != before;
        let empty = records.is_empty();
        if empty {
            self.subscriptions.remove(extension_id);
        }
        removed
    }

    pub fn contains_extension(&self, extension_id: &str) -> bool {
        self.subscriptions.contains_key(extension_id)
    }
}

/// 将公共 Kernel EventBus 适配为 Extension lifecycle port。
pub struct KernelEventSubscriptionAdapter {
    event_bus: Arc<dyn EventBus>,
    owners: Mutex<HashMap<SubscriptionId, String>>,
    /// 声明式订阅的事件投递目标；None 时只把事件记录到 tracing（供
    /// Extension runtime 落地前占位，事件到达仍被 Kernel 确认）。
    /// enable 接线由 Cordis 迁移任务装配，当前仅提供能力。
    #[allow(dead_code)]
    declared_sink: Option<Arc<dyn Fn(ExtensionEventDto) + Send + Sync>>,
}

/// 声明式订阅能力（38 §2.2）：真实注册/注销。enable 接线由 Cordis 迁移
/// 任务在 state.rs 完成，当前仅提供能力。
#[allow(dead_code)]
impl KernelEventSubscriptionAdapter {
    pub fn new(event_bus: Arc<dyn EventBus>) -> Self {
        Self {
            event_bus,
            owners: Mutex::new(HashMap::new()),
            declared_sink: None,
        }
    }

    /// 设置声明式订阅的事件投递目标。运行时 handler 入口落地后，Cordis
    /// 迁移任务可在装配时注入真正的桥接 sink。
    pub fn with_declared_sink(
        mut self,
        sink: Arc<dyn Fn(ExtensionEventDto) + Send + Sync>,
    ) -> Self {
        self.declared_sink = Some(sink);
        self
    }

    /// 把声明式 `event_subscriptions` 真实注册到 Kernel EventBus（38 §2.2）。
    ///
    /// 精确 topic 与通配 pattern（如 `session.*`）按 Kernel 的 glob 语义注册，
    /// 返回已成功注册的 opaque 句柄记录。调用方（enable 事务）只应把这批记录
    /// 写入 `ExtensionSubscriptionLedger`——ledger 只记录"已成功注册的 opaque 句柄"
    /// 的语义由此保持。
    ///
    /// fail-closed 约束：
    /// - 高频 pattern（`agent/*`、`terminal/*`、`task/*`）按 extension_bridge.rs
    ///   现有裁决强制走 stream，这里在注册前整体拒绝（不产生部分注册）。
    /// - 单个注册失败会逆序撤销本批已注册句柄，保证不泄漏无 ledger 记录的订阅。
    pub fn subscribe_declared(
        &self,
        extension_id: &str,
        declarations: &[DeclaredEventSubscription],
    ) -> anyhow::Result<Vec<ExtensionSubscriptionRecord>> {
        if extension_id.trim().is_empty() {
            return Err(anyhow::anyhow!("Extension ID must not be empty"));
        }
        if declarations.is_empty() {
            return Ok(Vec::new());
        }

        for declaration in declarations {
            if is_high_frequency_pattern(&declaration.topic) {
                return Err(anyhow::anyhow!(
                    "Event topic '{}' declared by Extension '{}' is a high-frequency stream \
                     pattern (agent/terminal/task) and must use the stream channel",
                    declaration.topic,
                    extension_id
                ));
            }
        }

        let handler = self.declared_handler(extension_id);
        let mut registered: Vec<ExtensionSubscriptionRecord> = Vec::new();
        for declaration in declarations {
            let topic = Topic::new(&declaration.topic);
            let subscription_id = match self.event_bus.subscribe(
                Some(topic.clone()),
                declaration.scope_key.clone(),
                handler.clone(),
            ) {
                Ok(subscription_id) => subscription_id,
                Err(error) => {
                    self.rollback_registered(extension_id, &registered);
                    return Err(anyhow::anyhow!(
                        "Extension '{}' failed to subscribe to event topic '{}': {}",
                        extension_id,
                        declaration.topic,
                        error
                    ));
                }
            };
            self.owners
                .lock()
                .insert(subscription_id.clone(), extension_id.to_string());
            registered.push(ExtensionSubscriptionRecord {
                subscription_id,
                topic,
                scope_key: declaration.scope_key.clone(),
            });
        }

        tracing::debug!(
            extension_id,
            count = registered.len(),
            "Registered declared Extension event subscriptions on Kernel EventBus"
        );
        Ok(registered)
    }

    /// 注销某扩展的全部已注册订阅（声明式与显式共用）。
    ///
    /// 无任何句柄时返回 Ok（幂等）。单个句柄注销失败会尽力继续注销其余句柄，
    /// 失败句柄保留在 owners 映射中以便重试，并返回首个错误。disable 路径的
    /// 逆序注销由调用方按 ledger 记录顺序 reverse 后逐个执行。
    pub fn unsubscribe_all(&self, extension_id: &str) -> anyhow::Result<()> {
        let ids = {
            let owners = self.owners.lock();
            owners
                .iter()
                .filter(|(_, owner)| owner.as_str() == extension_id)
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>()
        };
        let mut first_error = None;
        for id in ids {
            match self.event_bus.unsubscribe(&id) {
                Ok(()) => {
                    self.owners.lock().remove(&id);
                    tracing::debug!(
                        extension_id,
                        subscription_id = %id,
                        "Unregistered Extension event subscription"
                    );
                }
                Err(error) => {
                    tracing::warn!(
                        extension_id,
                        subscription_id = %id,
                        error = %error,
                        "Failed to unsubscribe Extension event subscription"
                    );
                    if first_error.is_none() {
                        first_error = Some(anyhow::anyhow!(
                            "Extension '{}' failed to unsubscribe event subscription '{}': {}",
                            extension_id,
                            id,
                            error
                        ));
                    }
                }
            }
        }
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    /// 声明式订阅的 Kernel handler：把事件转换为稳定 DTO 并交给投递 sink。
    fn declared_handler(&self, extension_id: &str) -> EventHandler {
        let sink = self.declared_sink.clone();
        let extension_id = extension_id.to_string();
        Arc::new(move |envelope: &EventEnvelope| {
            let dto = ExtensionEventDto::from(envelope);
            tracing::trace!(
                extension_id = %extension_id,
                topic = %dto.topic,
                event_id = %dto.id,
                "Extension declared subscription received Kernel event"
            );
            if let Some(sink) = &sink {
                sink(dto);
            }
        })
    }

    /// 逆序撤销一批已注册句柄（enable 中途失败时保证无泄漏）。
    fn rollback_registered(&self, extension_id: &str, records: &[ExtensionSubscriptionRecord]) {
        for record in records.iter().rev() {
            if let Err(error) = self.event_bus.unsubscribe(&record.subscription_id) {
                tracing::warn!(
                    extension_id,
                    subscription_id = %record.subscription_id,
                    error = %error,
                    "Failed to roll back partially registered Extension event subscription"
                );
                continue;
            }
            self.owners.lock().remove(&record.subscription_id);
        }
    }
}

/// 高频事件模式（`agent/*`、`terminal/*`、`task/*`）必须走 stream 通道，
/// 不允许经低延迟 Kernel EventBus 订阅。
///
/// 与 `ui/extension_bridge.rs::is_high_frequency_event_pattern` 的裁决一致；
/// 此处额外覆盖 `/` 分隔形式（38 §2.2 瀑布事件，如 `agent/pre-step`）。
#[allow(dead_code)]
fn is_high_frequency_pattern(pattern: &str) -> bool {
    let pattern = pattern.trim();
    ["agent", "terminal", "task"].iter().any(|kind| {
        pattern == *kind
            || pattern.starts_with(&format!("{kind}."))
            || pattern.starts_with(&format!("{kind}:"))
            || pattern.starts_with(&format!("{kind}/"))
    })
}

impl EventSubscriptionPort for KernelEventSubscriptionAdapter {
    fn subscribe_extension(
        &self,
        extension_id: &str,
        topic: String,
        scope_key: Option<String>,
        handler: ExtensionEventHandler,
    ) -> anyhow::Result<SubscriptionId> {
        if extension_id.trim().is_empty() {
            return Err(anyhow::anyhow!("Extension ID must not be empty"));
        }

        let kernel_topic = Topic::new(topic);
        let subscription_id = match handler {
            ExtensionEventHandler::Sync(handler) => {
                let adapter = Arc::new(move |event: &EventEnvelope| {
                    handler(ExtensionEventDto::from(event));
                });
                self.event_bus
                    .subscribe(Some(kernel_topic), scope_key, adapter)?
            }
            ExtensionEventHandler::Async(handler) => {
                let adapter = Arc::new(move |event: SharedEventEnvelope| {
                    let dto = ExtensionEventDto::from(event.as_ref());
                    handler(dto)
                });
                self.event_bus
                    .subscribe_async(Some(kernel_topic), scope_key, adapter)?
            }
        };

        self.owners
            .lock()
            .insert(subscription_id.clone(), extension_id.to_string());
        tracing::debug!(
            extension_id,
            subscription_id = %subscription_id,
            "Registered Extension event subscription"
        );
        Ok(subscription_id)
    }

    fn unsubscribe_extension(
        &self,
        extension_id: &str,
        subscription_id: &SubscriptionId,
    ) -> anyhow::Result<()> {
        let mut owners = self.owners.lock();
        let owner = owners.get(subscription_id).ok_or_else(|| {
            anyhow::anyhow!(
                "Event subscription '{}' is not owned by an Extension",
                subscription_id
            )
        })?;
        if owner != extension_id {
            return Err(anyhow::anyhow!(
                "Event subscription '{}' belongs to Extension '{}', not '{}'",
                subscription_id,
                owner,
                extension_id
            ));
        }

        // Keep ownership until the kernel confirms removal so a failed cleanup
        // remains retryable by the owning Extension.
        self.event_bus.unsubscribe(subscription_id)?;
        owners.remove(subscription_id);
        tracing::debug!(
            extension_id,
            subscription_id = %subscription_id,
            "Unregistered Extension event subscription"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::{
        AsyncEventHandler, EventBusStats, EventEnvelope, EventHandler, InMemoryEventBus,
        KernelContext, KernelObjectInfo, KernelResult, SharedEventEnvelope,
    };
    use futures_util::FutureExt;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::{Duration, Instant};
    use tokio::runtime::Handle;

    struct FailOnceUnsubscribeBus {
        inner: InMemoryEventBus,
        fail_next_unsubscribe: AtomicBool,
    }

    impl FailOnceUnsubscribeBus {
        fn new() -> Self {
            Self {
                inner: InMemoryEventBus::new(100, Handle::current()),
                fail_next_unsubscribe: AtomicBool::new(true),
            }
        }
    }

    impl EventBus for FailOnceUnsubscribeBus {
        fn emit(&self, envelope: EventEnvelope) -> KernelResult<()> {
            self.inner.emit(envelope)
        }

        fn subscribe(
            &self,
            topic: Option<Topic>,
            scope_key: Option<String>,
            handler: EventHandler,
        ) -> KernelResult<SubscriptionId> {
            self.inner.subscribe(topic, scope_key, handler)
        }

        fn subscribe_async(
            &self,
            topic: Option<Topic>,
            scope_key: Option<String>,
            handler: AsyncEventHandler,
        ) -> KernelResult<SubscriptionId> {
            self.inner.subscribe_async(topic, scope_key, handler)
        }

        fn unsubscribe(&self, id: &SubscriptionId) -> KernelResult<()> {
            if self.fail_next_unsubscribe.swap(false, Ordering::SeqCst) {
                return Err(crate::kernel::KernelError::invalid_input(
                    "injected unsubscribe failure",
                ));
            }
            self.inner.unsubscribe(id)
        }

        fn recent(&self, limit: usize) -> Vec<SharedEventEnvelope> {
            self.inner.recent(limit)
        }

        fn stats(&self) -> EventBusStats {
            self.inner.stats()
        }

        fn objects(&self) -> Vec<KernelObjectInfo> {
            self.inner.objects()
        }
    }

    /// 第二次 subscribe 调用时注入失败的测试总线：验证 subscribe_declared
    /// 中途失败会逆序撤销已注册句柄。
    struct FailOnSecondSubscribeBus {
        inner: InMemoryEventBus,
        subscribe_calls: AtomicUsize,
    }

    impl FailOnSecondSubscribeBus {
        fn new() -> Self {
            Self {
                inner: InMemoryEventBus::new(100, Handle::current()),
                subscribe_calls: AtomicUsize::new(0),
            }
        }
    }

    impl EventBus for FailOnSecondSubscribeBus {
        fn emit(&self, envelope: EventEnvelope) -> KernelResult<()> {
            self.inner.emit(envelope)
        }

        fn subscribe(
            &self,
            topic: Option<Topic>,
            scope_key: Option<String>,
            handler: EventHandler,
        ) -> KernelResult<SubscriptionId> {
            if self.subscribe_calls.fetch_add(1, Ordering::SeqCst) == 1 {
                return Err(crate::kernel::KernelError::invalid_input(
                    "injected subscribe failure",
                ));
            }
            self.inner.subscribe(topic, scope_key, handler)
        }

        fn subscribe_async(
            &self,
            topic: Option<Topic>,
            scope_key: Option<String>,
            handler: AsyncEventHandler,
        ) -> KernelResult<SubscriptionId> {
            self.inner.subscribe_async(topic, scope_key, handler)
        }

        fn unsubscribe(&self, id: &SubscriptionId) -> KernelResult<()> {
            self.inner.unsubscribe(id)
        }

        fn recent(&self, limit: usize) -> Vec<SharedEventEnvelope> {
            self.inner.recent(limit)
        }

        fn stats(&self) -> EventBusStats {
            self.inner.stats()
        }

        fn objects(&self) -> Vec<KernelObjectInfo> {
            self.inner.objects()
        }
    }

    fn declared(id: &str, topic: &str) -> DeclaredEventSubscription {
        DeclaredEventSubscription {
            id: id.to_string(),
            topic: topic.to_string(),
            scope_key: None,
        }
    }

    async fn wait_for_count(counter: &AtomicUsize, expected: usize) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if counter.load(Ordering::SeqCst) == expected {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    fn event(topic: &str) -> EventEnvelope {
        EventEnvelope::new(
            topic,
            KernelContext::new("test", crate::kernel::KernelScope::global()),
            None,
        )
    }

    #[tokio::test]
    async fn adapter_subscribe_and_unsubscribe_controls_delivery() {
        let bus = Arc::new(InMemoryEventBus::new(100, Handle::current()));
        let adapter = KernelEventSubscriptionAdapter::new(bus.clone());
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_clone = hits.clone();

        let subscription_id = adapter
            .subscribe_extension(
                "extension.one",
                "extension.changed".to_string(),
                None,
                ExtensionEventHandler::Sync(Arc::new(move |_| {
                    hits_clone.fetch_add(1, Ordering::SeqCst);
                })),
            )
            .unwrap();
        bus.emit(event("extension.changed")).unwrap();
        wait_for_count(&hits, 1).await;
        assert_eq!(hits.load(Ordering::SeqCst), 1);

        adapter
            .unsubscribe_extension("extension.one", &subscription_id)
            .unwrap();
        bus.emit(event("extension.changed")).unwrap();
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        assert_eq!(bus.stats().subscription_count, 0);
    }

    #[tokio::test]
    async fn adapter_rejects_cross_extension_unsubscribe() {
        let bus = Arc::new(InMemoryEventBus::new(100, Handle::current()));
        let adapter = KernelEventSubscriptionAdapter::new(bus.clone());
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_clone = hits.clone();
        let subscription_id = adapter
            .subscribe_extension(
                "extension.owner",
                "extension.changed".to_string(),
                None,
                ExtensionEventHandler::Sync(Arc::new(move |_| {
                    hits_clone.fetch_add(1, Ordering::SeqCst);
                })),
            )
            .unwrap();

        let error = adapter
            .unsubscribe_extension("extension.other", &subscription_id)
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("belongs to Extension 'extension.owner'"));
        assert_eq!(bus.stats().subscription_count, 1);

        bus.emit(event("extension.changed")).unwrap();
        wait_for_count(&hits, 1).await;
        assert_eq!(hits.load(Ordering::SeqCst), 1);

        adapter
            .unsubscribe_extension("extension.owner", &subscription_id)
            .unwrap();
    }

    #[tokio::test]
    async fn adapter_retains_ownership_when_unsubscribe_fails() {
        let bus = Arc::new(FailOnceUnsubscribeBus::new());
        let adapter = KernelEventSubscriptionAdapter::new(bus.clone());
        let subscription_id = adapter
            .subscribe_extension(
                "extension.owner",
                "extension.changed".to_string(),
                None,
                ExtensionEventHandler::Async(Arc::new(|_| async {}.boxed())),
            )
            .unwrap();

        let error = adapter
            .unsubscribe_extension("extension.owner", &subscription_id)
            .unwrap_err();
        assert!(error.to_string().contains("injected unsubscribe failure"));
        assert_eq!(bus.stats().subscription_count, 1);

        adapter
            .unsubscribe_extension("extension.owner", &subscription_id)
            .unwrap();
        assert_eq!(bus.stats().subscription_count, 0);
    }

    #[tokio::test]
    async fn declared_subscriptions_register_deliver_and_cleanup_on_kernel_bus() {
        let bus = Arc::new(InMemoryEventBus::new(100, Handle::current()));
        let received = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let received_clone = received.clone();
        let adapter = KernelEventSubscriptionAdapter::new(bus.clone()).with_declared_sink(Arc::new(
            move |dto| received_clone.lock().push(dto.topic),
        ));

        let records = adapter
            .subscribe_declared(
                "extension.demo",
                &[
                    declared("session-completed", "session.completed"),
                    declared("project-changed", "project.changed"),
                ],
            )
            .unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(bus.stats().subscription_count, 2);

        // 返回的 opaque 句柄可写入 SubscriptionLedger，符合"只记录已注册句柄"语义。
        let mut ledger = ExtensionSubscriptionLedger::default();
        ledger.record_many("extension.demo", &records).unwrap();
        assert_eq!(ledger.records("extension.demo").len(), 2);

        bus.emit(event("session.completed")).unwrap();
        bus.emit(event("project.changed")).unwrap();
        bus.emit(event("agent.completed")).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(
            received.lock().as_slice(),
            &["session.completed", "project.changed"]
        );

        // unsubscribe_all 逆序注销；随后事件不再投递。
        adapter.unsubscribe_all("extension.demo").unwrap();
        assert_eq!(bus.stats().subscription_count, 0);
        bus.emit(event("session.completed")).unwrap();
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert_eq!(received.lock().len(), 2);

        // 注销成功后 ledger 同步移除。
        for record in &records {
            assert!(ledger.remove("extension.demo", &record.subscription_id));
        }
        assert!(!ledger.contains_extension("extension.demo"));
    }

    #[tokio::test]
    async fn declared_wildcard_pattern_matches_all_topics_in_family() {
        let bus = Arc::new(InMemoryEventBus::new(100, Handle::current()));
        let received = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let received_clone = received.clone();
        let adapter = KernelEventSubscriptionAdapter::new(bus.clone()).with_declared_sink(Arc::new(
            move |dto| received_clone.lock().push(dto.topic),
        ));

        let records = adapter
            .subscribe_declared(
                "extension.demo",
                &[declared("session-any", "session.*")],
            )
            .unwrap();
        assert_eq!(records.len(), 1);

        bus.emit(event("session.completed")).unwrap();
        bus.emit(event("session.changed")).unwrap();
        bus.emit(event("project.changed")).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(
            received.lock().as_slice(),
            &["session.completed", "session.changed"]
        );

        adapter.unsubscribe_all("extension.demo").unwrap();
        assert_eq!(bus.stats().subscription_count, 0);
    }

    #[tokio::test]
    async fn declared_scope_key_restricts_delivery_to_matching_scope() {
        let bus = Arc::new(InMemoryEventBus::new(100, Handle::current()));
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_clone = hits.clone();
        let adapter = KernelEventSubscriptionAdapter::new(bus.clone()).with_declared_sink(Arc::new(
            move |_| {
                hits_clone.fetch_add(1, Ordering::SeqCst);
            },
        ));

        adapter
            .subscribe_declared(
                "extension.demo",
                &[DeclaredEventSubscription {
                    id: "scoped".to_string(),
                    topic: "state.changed".to_string(),
                    scope_key: Some("session:a".to_string()),
                }],
            )
            .unwrap();

        bus.emit(EventEnvelope::new(
            "state.changed",
            KernelContext::new("test", crate::kernel::KernelScope::scoped("session", "a")),
            None,
        ))
        .unwrap();
        bus.emit(EventEnvelope::new(
            "state.changed",
            KernelContext::new("test", crate::kernel::KernelScope::scoped("session", "b")),
            None,
        ))
        .unwrap();
        wait_for_count(&hits, 1).await;
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn declared_high_frequency_patterns_are_rejected_before_registration() {
        let bus = Arc::new(InMemoryEventBus::new(100, Handle::current()));
        let adapter = KernelEventSubscriptionAdapter::new(bus.clone());

        for pattern in ["agent.*", "agent/pre-step", "terminal.*", "task.finished"] {
            let error = adapter
                .subscribe_declared("extension.demo", &[declared("stream", pattern)])
                .unwrap_err();
            assert!(
                error.to_string().contains("high-frequency stream"),
                "pattern '{pattern}' should be rejected, got: {error}"
            );
        }
        // 批级 fail-closed：任一高频 pattern 拒绝整批，不产生部分注册。
        let error = adapter
            .subscribe_declared(
                "extension.demo",
                &[declared("ok", "session.completed"), declared("hot", "agent.*")],
            )
            .unwrap_err();
        assert!(error.to_string().contains("agent.*"));
        assert_eq!(bus.stats().subscription_count, 0);
    }

    #[tokio::test]
    async fn declared_subscribe_rolls_back_partial_registration_on_failure() {
        let bus = Arc::new(FailOnSecondSubscribeBus::new());
        let adapter = KernelEventSubscriptionAdapter::new(bus.clone());

        let error = adapter
            .subscribe_declared(
                "extension.demo",
                &[
                    declared("first", "session.completed"),
                    declared("second", "project.changed"),
                ],
            )
            .unwrap_err();
        assert!(error.to_string().contains("injected subscribe failure"));
        // 首个句柄已注册但批次失败，必须逆序撤销，不泄漏无 ledger 记录的订阅。
        assert_eq!(bus.stats().subscription_count, 0);
    }

    #[tokio::test]
    async fn unsubscribe_all_is_idempotent_and_clears_owners() {
        let bus = Arc::new(InMemoryEventBus::new(100, Handle::current()));
        let adapter = KernelEventSubscriptionAdapter::new(bus.clone());
        adapter
            .subscribe_declared(
                "extension.demo",
                &[declared("one", "session.completed"), declared("two", "project.changed")],
            )
            .unwrap();

        adapter.unsubscribe_all("extension.demo").unwrap();
        // 无句柄时再次调用返回 Ok（幂等），不影响其他扩展的句柄。
        adapter.unsubscribe_all("extension.demo").unwrap();
        adapter.unsubscribe_all("extension.other").unwrap();
        assert_eq!(bus.stats().subscription_count, 0);
    }

    #[test]
    fn waterfall_pattern_constants_match_design_contract() {
        assert_eq!(super::waterfall::AGENT_PRE_STEP, "agent/pre-step");
        assert_eq!(super::waterfall::LLM_STREAM, "llm/stream");
        assert_eq!(super::waterfall::TOOLS_PRE_EXECUTE, "tools/pre-execute");
        assert_eq!(super::waterfall::TOOLS_EXECUTE, "tools/execute");
        assert_eq!(super::waterfall::TOOLS_POST_EXECUTE, "tools/post-execute");
        assert_eq!(super::waterfall::AGENT_TURN_STOPPING, "agent/turn-stopping");
        // 模式常量互不重复。
        let mut unique = std::collections::HashSet::new();
        for pattern in super::waterfall::ALL {
            assert!(unique.insert(pattern));
        }
        assert_eq!(unique.len(), 6);
    }

    #[test]
    fn declared_event_subscription_projects_from_manifest_dto() {
        let registration = EventSubscriptionRegistration {
            id: "session-completed".into(),
            topic: "session.completed".into(),
            scope_key: Some("session:active".into()),
            handler: crate::extension::models::EventHandlerReference {
                module: "./runtime/events".into(),
                export: "onSessionCompleted".into(),
            },
        };
        let projected = DeclaredEventSubscription::from(&registration);
        assert_eq!(projected.id, "session-completed");
        assert_eq!(projected.topic, "session.completed");
        assert_eq!(projected.scope_key.as_deref(), Some("session:active"));
    }

    fn record(id: &str) -> ExtensionSubscriptionRecord {
        ExtensionSubscriptionRecord {
            subscription_id: SubscriptionId::new(id),
            topic: Topic::new("agent.completed"),
            scope_key: Some("session:1".to_string()),
        }
    }

    #[test]
    fn ledger_empty_batch_does_not_create_owner() {
        let mut ledger = ExtensionSubscriptionLedger::default();
        ledger.record_many("ext.demo", &[]).unwrap();
        assert!(!ledger.contains_extension("ext.demo"));
        assert!(ledger.records("ext.demo").is_empty());
    }

    #[test]
    fn ledger_records_and_removes_owned_subscription() {
        let mut ledger = ExtensionSubscriptionLedger::default();
        ledger.record_many("ext.demo", &[record("sub-1")]).unwrap();
        assert!(ledger.contains_extension("ext.demo"));
        assert_eq!(ledger.records("ext.demo").len(), 1);

        assert!(ledger.remove("ext.demo", &SubscriptionId::new("sub-1")));
        assert!(!ledger.contains_extension("ext.demo"));
        assert!(ledger.records("ext.demo").is_empty());
    }

    #[test]
    fn ledger_rejects_duplicate_opaque_handle() {
        let mut ledger = ExtensionSubscriptionLedger::default();
        ledger.record_many("ext.one", &[record("sub-1")]).unwrap();
        let error = ledger
            .record_many("ext.two", &[record("sub-1")])
            .unwrap_err();
        assert!(error.to_string().contains("already owned"));
    }

    #[test]
    fn ledger_does_not_remove_unknown_handle() {
        let mut ledger = ExtensionSubscriptionLedger::default();
        ledger.record_many("ext.demo", &[record("sub-1")]).unwrap();
        assert!(!ledger.remove("ext.demo", &SubscriptionId::new("missing")));
        assert_eq!(ledger.records("ext.demo").len(), 1);
    }

    #[test]
    fn ledger_batch_record_is_atomic_when_validation_fails() {
        let mut ledger = ExtensionSubscriptionLedger::default();
        let error = ledger
            .record_many("ext.demo", &[record("sub-1"), record("sub-1")])
            .unwrap_err();

        assert!(error.to_string().contains("duplicated"));
        assert!(ledger.records("ext.demo").is_empty());
    }
}
