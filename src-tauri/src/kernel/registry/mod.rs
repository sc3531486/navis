use arc_swap::ArcSwap;
use async_trait::async_trait;
use chrono::Utc;
use im::HashMap;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Arc;
use triomphe::Arc as SharedArc;

use super::{
    CapabilityId, KernelContext, KernelError, KernelObjectInfo, KernelObjectState, KernelResource,
    KernelResult, SchemaVersion, ShutdownMode,
};

pub trait Capability: Send + Sync {
    fn id(&self) -> &str;
    fn kind(&self) -> &str;
    fn version(&self) -> SchemaVersion;
    fn metadata(&self) -> &Value;
}

impl<T> Capability for Arc<T>
where
    T: Capability + ?Sized,
{
    fn id(&self) -> &str {
        self.as_ref().id()
    }

    fn kind(&self) -> &str {
        self.as_ref().kind()
    }

    fn version(&self) -> SchemaVersion {
        self.as_ref().version()
    }

    fn metadata(&self) -> &Value {
        self.as_ref().metadata()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleState {
    Registered,
    Enabled,
    Running,
    Removed,
}

impl LifecycleState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Registered => "Registered",
            Self::Enabled => "Enabled",
            Self::Running => "Running",
            Self::Removed => "Removed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleAction {
    Enable,
    Disable,
    Start,
    Stop,
    Remove,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityInfo {
    pub id: String,
    pub kind: String,
    pub version: SchemaVersion,
    pub state: LifecycleState,
    pub metadata: SharedArc<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryStats {
    pub entry_count: usize,
    pub available_count: usize,
    pub by_kind: BTreeMap<String, usize>,
    pub by_state: BTreeMap<String, usize>,
}

impl CapabilityInfo {
    pub fn object_info(&self) -> KernelObjectInfo {
        KernelObjectInfo {
            id: self.id.clone(),
            kind: format!("registry.{}", self.kind),
            state: self.state.into(),
            scope: "global".to_string(),
            owner: None,
            created_at: Utc::now(),
            updated_at: None,
            metadata: self.metadata.clone(),
        }
    }
}

pub struct RegistryEntry<T: Capability + ?Sized> {
    item: Arc<T>,
    state: LifecycleState,
    cached_id: String,
    cached_kind: String,
    cached_object_kind: String,
    cached_state: &'static str,
    cached_version: SchemaVersion,
    cached_metadata: SharedArc<Value>,
}

impl<T: Capability + ?Sized> Clone for RegistryEntry<T> {
    fn clone(&self) -> Self {
        Self {
            item: self.item.clone(),
            state: self.state,
            cached_id: self.cached_id.clone(),
            cached_kind: self.cached_kind.clone(),
            cached_object_kind: self.cached_object_kind.clone(),
            cached_state: self.cached_state,
            cached_version: self.cached_version,
            cached_metadata: self.cached_metadata.clone(),
        }
    }
}

impl<T: Capability + ?Sized> RegistryEntry<T> {
    pub fn new(item: Arc<T>) -> Self {
        Self::with_state(item, LifecycleState::Registered)
    }

    pub fn with_state(item: Arc<T>, state: LifecycleState) -> Self {
        let cached_id = item.id().to_string();
        let cached_kind = item.kind().to_string();
        let cached_object_kind = format!("registry.{cached_kind}");
        let cached_version = item.version();
        let cached_metadata = SharedArc::new(item.metadata().clone());
        Self {
            item,
            state,
            cached_id,
            cached_kind,
            cached_object_kind,
            cached_state: state.as_str(),
            cached_version,
            cached_metadata,
        }
    }

    pub fn info(&self) -> CapabilityInfo {
        CapabilityInfo {
            id: self.cached_id.clone(),
            kind: self.cached_kind.clone(),
            version: self.cached_version,
            state: self.state,
            metadata: self.cached_metadata.clone(),
        }
    }

    pub fn object_info(&self) -> KernelObjectInfo {
        KernelObjectInfo {
            id: self.cached_id.clone(),
            kind: self.cached_object_kind.clone(),
            state: self.state.into(),
            scope: "global".to_string(),
            owner: None,
            created_at: Utc::now(),
            updated_at: None,
            metadata: self.cached_metadata.clone(),
        }
    }

    pub fn item(&self) -> Arc<T> {
        self.item.clone()
    }

    fn set_state(&mut self, state: LifecycleState) {
        self.state = state;
        self.cached_state = state.as_str();
    }

    fn is_available(&self) -> bool {
        matches!(
            self.state,
            LifecycleState::Enabled | LifecycleState::Running
        )
    }
}

pub trait Registry<T: Capability + ?Sized>: Send + Sync {
    fn register_arc(&self, item: Arc<T>) -> KernelResult<CapabilityId>;
    fn replace_arc(&self, item: Arc<T>) -> KernelResult<CapabilityId>;
    fn replace_arc_reset(&self, item: Arc<T>) -> KernelResult<CapabilityId>;
    fn unregister(&self, id: &str) -> KernelResult<()>;
    fn get(&self, id: &str) -> Option<Arc<T>>;
    fn get_registered(&self, id: &str) -> Option<Arc<T>>;
    fn info(&self, id: &str) -> Option<CapabilityInfo>;
    fn list(&self) -> Vec<CapabilityInfo>;
    fn objects(&self) -> Vec<KernelObjectInfo>;
    fn stats(&self) -> RegistryStats;
    fn is_registered(&self, id: &str) -> bool;
    fn is_available(&self, id: &str) -> bool;
    fn find(
        &self,
        predicate: &(dyn Fn(&CapabilityInfo) -> bool + Send + Sync),
    ) -> Vec<CapabilityInfo>;
    fn lifecycle(&self, id: &str, action: LifecycleAction) -> KernelResult<LifecycleState>;
}

#[async_trait]
pub trait CapabilityLifecycle: Capability {
    async fn before_lifecycle(
        &self,
        _action: LifecycleAction,
        _context: &KernelContext,
    ) -> KernelResult<()> {
        Ok(())
    }

    async fn after_lifecycle(
        &self,
        _action: LifecycleAction,
        _state: LifecycleState,
        _context: &KernelContext,
    ) -> KernelResult<()> {
        Ok(())
    }
}

#[async_trait]
pub trait AsyncRegistry<T: CapabilityLifecycle + ?Sized>: Registry<T> {
    async fn lifecycle_async(
        &self,
        id: &str,
        action: LifecycleAction,
        context: &KernelContext,
    ) -> KernelResult<LifecycleState>;
}

pub struct InMemoryRegistry<T: Capability + ?Sized> {
    entries: ArcSwap<HashMap<String, RegistryEntry<T>>>,
    write_lock: Mutex<()>,
}

impl<T: Capability + ?Sized> Default for InMemoryRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Capability + ?Sized> InMemoryRegistry<T> {
    pub fn new() -> Self {
        Self {
            entries: ArcSwap::from_pointee(HashMap::new()),
            write_lock: Mutex::new(()),
        }
    }

    fn next_state(
        current: LifecycleState,
        action: LifecycleAction,
    ) -> KernelResult<LifecycleState> {
        match (current, action) {
            (LifecycleState::Registered, LifecycleAction::Enable) => Ok(LifecycleState::Enabled),
            (LifecycleState::Enabled, LifecycleAction::Disable) => Ok(LifecycleState::Registered),
            (LifecycleState::Enabled, LifecycleAction::Start) => Ok(LifecycleState::Running),
            (LifecycleState::Running, LifecycleAction::Stop) => Ok(LifecycleState::Enabled),
            (_, LifecycleAction::Remove) => Ok(LifecycleState::Removed),
            (state, action) => Err(KernelError::invalid_input(format!(
                "invalid lifecycle transition from {state:?} via {action:?}"
            ))),
        }
    }

    fn store_entries(&self, mutate: impl FnOnce(&mut HashMap<String, RegistryEntry<T>>)) {
        let mut updated = (*self.entries.load_full()).clone();
        mutate(&mut updated);
        self.entries.store(Arc::new(updated));
    }
}

impl<T> InMemoryRegistry<T>
where
    T: Capability + Sized + 'static,
{
    pub fn register(&self, item: T) -> KernelResult<CapabilityId> {
        self.register_arc(Arc::new(item))
    }

    pub fn replace(&self, item: T) -> KernelResult<CapabilityId> {
        self.replace_arc(Arc::new(item))
    }

    pub fn replace_reset(&self, item: T) -> KernelResult<CapabilityId> {
        self.replace_arc_reset(Arc::new(item))
    }
}

impl<T: Capability + ?Sized> Registry<T> for InMemoryRegistry<T> {
    fn register_arc(&self, item: Arc<T>) -> KernelResult<CapabilityId> {
        let id = item.id().to_string();
        let kind = item.kind();
        let version = item.version();
        tracing::debug!(
            capability_id = %id,
            kind = %kind,
            version = %version,
            "registering capability"
        );

        let _guard = self.write_lock.lock();
        {
            let entries = self.entries.load();
            if entries.contains_key(&id) {
                tracing::warn!(capability_id = %id, "capability registration rejected");
                return Err(KernelError::CapabilityAlreadyRegistered { id });
            }
        }
        let capability_id = CapabilityId::new(id.clone());
        self.store_entries(|entries| {
            entries.insert(id.clone(), RegistryEntry::new(item.clone()));
        });
        Ok(capability_id)
    }

    fn replace_arc(&self, item: Arc<T>) -> KernelResult<CapabilityId> {
        let id = item.id().to_string();
        let kind = item.kind();
        let version = item.version();
        tracing::debug!(
            capability_id = %id,
            kind = %kind,
            version = %version,
            "replacing capability"
        );

        let _guard = self.write_lock.lock();
        let capability_id = CapabilityId::new(id.clone());
        self.store_entries(|entries| {
            let state = entries
                .get(&id)
                .map(|entry| entry.state)
                .unwrap_or(LifecycleState::Registered);
            entries.insert(id.clone(), RegistryEntry::with_state(item.clone(), state));
        });
        Ok(capability_id)
    }

    fn replace_arc_reset(&self, item: Arc<T>) -> KernelResult<CapabilityId> {
        let id = item.id().to_string();
        let kind = item.kind();
        let version = item.version();
        tracing::debug!(
            capability_id = %id,
            kind = %kind,
            version = %version,
            "replacing capability and resetting lifecycle"
        );

        let _guard = self.write_lock.lock();
        let capability_id = CapabilityId::new(id.clone());
        self.store_entries(|entries| {
            entries.insert(id.clone(), RegistryEntry::new(item.clone()));
        });
        Ok(capability_id)
    }

    fn unregister(&self, id: &str) -> KernelResult<()> {
        tracing::debug!(capability_id = %id, "unregistering capability");
        let _guard = self.write_lock.lock();
        self.entries
            .load()
            .get(id)
            .ok_or_else(|| KernelError::CapabilityNotFound { id: id.into() })?;
        self.store_entries(|entries| {
            entries.remove(id);
        });
        Ok(())
    }

    fn get(&self, id: &str) -> Option<Arc<T>> {
        let entries = self.entries.load();
        let entry = entries.get(id)?;
        if entry.is_available() {
            Some(entry.item())
        } else {
            None
        }
    }

    fn get_registered(&self, id: &str) -> Option<Arc<T>> {
        self.entries.load().get(id).map(RegistryEntry::item)
    }

    fn info(&self, id: &str) -> Option<CapabilityInfo> {
        self.entries.load().get(id).map(RegistryEntry::info)
    }

    fn list(&self) -> Vec<CapabilityInfo> {
        let entries = self.entries.load();
        let mut items = Vec::with_capacity(entries.len());
        items.extend(entries.values().map(RegistryEntry::info));
        items
    }

    fn objects(&self) -> Vec<KernelObjectInfo> {
        let entries = self.entries.load();
        let mut objects = Vec::with_capacity(entries.len());
        objects.extend(entries.values().map(RegistryEntry::object_info));
        objects
    }

    fn stats(&self) -> RegistryStats {
        let entries = self.entries.load();
        let mut by_kind = BTreeMap::new();
        let mut by_state = BTreeMap::new();
        let mut available_count = 0;

        for entry in entries.values() {
            if let Some(count) = by_kind.get_mut(entry.cached_kind.as_str()) {
                *count += 1;
            } else {
                by_kind.insert(entry.cached_kind.clone(), 1);
            }
            if let Some(count) = by_state.get_mut(entry.cached_state) {
                *count += 1;
            } else {
                by_state.insert(entry.cached_state.to_string(), 1);
            }
            if entry.is_available() {
                available_count += 1;
            }
        }

        RegistryStats {
            entry_count: entries.len(),
            available_count,
            by_kind,
            by_state,
        }
    }

    fn is_registered(&self, id: &str) -> bool {
        self.entries.load().contains_key(id)
    }

    fn is_available(&self, id: &str) -> bool {
        self.get(id).is_some()
    }

    fn find(
        &self,
        predicate: &(dyn Fn(&CapabilityInfo) -> bool + Send + Sync),
    ) -> Vec<CapabilityInfo> {
        let entries = self.entries.load();
        let mut matches = Vec::new();
        for entry in entries.values() {
            let info = entry.info();
            if predicate(&info) {
                matches.push(info);
            }
        }
        matches
    }

    fn lifecycle(&self, id: &str, action: LifecycleAction) -> KernelResult<LifecycleState> {
        tracing::debug!(
            capability_id = %id,
            action = ?action,
            "changing capability lifecycle"
        );
        let _guard = self.write_lock.lock();
        let current_state = self
            .entries
            .load()
            .get(id)
            .map(|entry| entry.state)
            .ok_or_else(|| KernelError::CapabilityNotFound { id: id.into() })?;
        let state = Self::next_state(current_state, action)?;
        if matches!(action, LifecycleAction::Remove) {
            self.store_entries(|entries| {
                entries.remove(id);
            });
            tracing::debug!(
                capability_id = %id,
                state = ?state,
                "capability lifecycle removed"
            );
            return Ok(state);
        }
        self.store_entries(|entries| {
            if let Some(entry) = entries.get_mut(id) {
                entry.set_state(state);
            }
        });
        tracing::debug!(
            capability_id = %id,
            state = ?state,
            "capability lifecycle changed"
        );
        Ok(state)
    }
}

impl<T: Capability + ?Sized> KernelResource for InMemoryRegistry<T> {
    fn object_info(&self) -> KernelObjectInfo {
        let stats = self.stats();
        KernelObjectInfo::new(
            "registry",
            "registry",
            if stats.entry_count > 0 {
                KernelObjectState::Enabled
            } else {
                KernelObjectState::Registered
            },
            "global",
        )
        .with_metadata(serde_json::json!({
            "entryCount": stats.entry_count,
            "availableCount": stats.available_count,
            "byKind": stats.by_kind,
            "byState": stats.by_state,
        }))
    }

    fn active_leases(&self) -> usize {
        self.stats().entry_count
    }

    fn shutdown(&self, mode: ShutdownMode) -> KernelResult<()> {
        match mode {
            ShutdownMode::Graceful | ShutdownMode::Deadline(_) if self.active_leases() == 0 => {
                Ok(())
            }
            ShutdownMode::Graceful | ShutdownMode::Deadline(_) => Err(KernelError::invalid_input(
                "registry has active resource leases",
            )),
            ShutdownMode::Immediate => {
                let _guard = self.write_lock.lock();
                self.entries.store(Arc::new(HashMap::new()));
                Ok(())
            }
        }
    }
}

impl From<LifecycleState> for KernelObjectState {
    fn from(state: LifecycleState) -> Self {
        match state {
            LifecycleState::Registered => Self::Registered,
            LifecycleState::Enabled => Self::Enabled,
            LifecycleState::Running => Self::Running,
            LifecycleState::Removed => Self::Removed,
        }
    }
}

#[async_trait]
impl<T> AsyncRegistry<T> for InMemoryRegistry<T>
where
    T: CapabilityLifecycle + ?Sized + 'static,
{
    async fn lifecycle_async(
        &self,
        id: &str,
        action: LifecycleAction,
        context: &KernelContext,
    ) -> KernelResult<LifecycleState> {
        let item = self
            .entries
            .load()
            .get(id)
            .map(RegistryEntry::item)
            .ok_or_else(|| KernelError::CapabilityNotFound { id: id.into() })?;

        item.before_lifecycle(action, context).await?;
        let state = self.lifecycle(id, action)?;
        item.after_lifecycle(action, state, context).await?;
        Ok(state)
    }
}

#[async_trait]
pub trait RegistryLoader<T: Capability + ?Sized>: Registry<T> {
    async fn load(&self, source: &str, context: &KernelContext) -> KernelResult<Vec<Arc<T>>>;
    async fn unload(&self, id: &str, context: &KernelContext) -> KernelResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Barrier;

    struct DemoCapability {
        id: String,
        metadata: Value,
    }

    impl Capability for DemoCapability {
        fn id(&self) -> &str {
            &self.id
        }

        fn kind(&self) -> &str {
            "demo"
        }

        fn version(&self) -> SchemaVersion {
            SchemaVersion::default()
        }

        fn metadata(&self) -> &Value {
            &self.metadata
        }
    }

    struct LifecycleCapability {
        id: String,
        metadata: Value,
        before_count: Arc<AtomicUsize>,
        after_count: Arc<AtomicUsize>,
    }

    impl Capability for LifecycleCapability {
        fn id(&self) -> &str {
            &self.id
        }

        fn kind(&self) -> &str {
            "lifecycle"
        }

        fn version(&self) -> SchemaVersion {
            SchemaVersion::default()
        }

        fn metadata(&self) -> &Value {
            &self.metadata
        }
    }

    #[async_trait]
    impl CapabilityLifecycle for LifecycleCapability {
        async fn before_lifecycle(
            &self,
            _action: LifecycleAction,
            _context: &KernelContext,
        ) -> KernelResult<()> {
            self.before_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn after_lifecycle(
            &self,
            _action: LifecycleAction,
            _state: LifecycleState,
            _context: &KernelContext,
        ) -> KernelResult<()> {
            self.after_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn only_enabled_items_are_returned() {
        let registry = InMemoryRegistry::<DemoCapability>::new();
        registry
            .register(DemoCapability {
                id: "capability.a".into(),
                metadata: json!({}),
            })
            .unwrap();

        assert!(registry.get("capability.a").is_none());
        registry
            .lifecycle("capability.a", LifecycleAction::Enable)
            .unwrap();
        assert!(registry.get("capability.a").is_some());
    }

    #[test]
    fn registered_item_can_be_read_before_enable() {
        let registry = InMemoryRegistry::<DemoCapability>::new();
        registry
            .register(DemoCapability {
                id: "capability.a".into(),
                metadata: json!({ "label": "A" }),
            })
            .unwrap();

        assert!(registry.get("capability.a").is_none());
        assert!(registry.get_registered("capability.a").is_some());
        let info = registry.info("capability.a").unwrap();
        assert_eq!(info.state, LifecycleState::Registered);
        assert_eq!(
            info.metadata.get("label").and_then(Value::as_str),
            Some("A")
        );
    }

    #[test]
    fn registry_exports_kernel_object_info() {
        let registry = InMemoryRegistry::<DemoCapability>::new();
        registry
            .register(DemoCapability {
                id: "capability.a".into(),
                metadata: json!({ "label": "A" }),
            })
            .unwrap();

        let objects = registry.objects();
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].id, "capability.a");
        assert_eq!(objects[0].kind, "registry.demo");
        assert_eq!(objects[0].state, KernelObjectState::Registered);
        assert_eq!(
            objects[0].metadata.get("label").and_then(Value::as_str),
            Some("A")
        );
    }

    #[test]
    fn registry_stats_counts_entries_by_kind_and_state() {
        let registry = InMemoryRegistry::<DemoCapability>::new();
        registry
            .register(DemoCapability {
                id: "capability.a".into(),
                metadata: json!({}),
            })
            .unwrap();
        registry
            .register(DemoCapability {
                id: "capability.b".into(),
                metadata: json!({}),
            })
            .unwrap();
        registry
            .lifecycle("capability.b", LifecycleAction::Enable)
            .unwrap();

        let stats = registry.stats();
        assert_eq!(stats.entry_count, 2);
        assert_eq!(stats.available_count, 1);
        assert_eq!(stats.by_kind.get("demo"), Some(&2));
        assert_eq!(stats.by_state.get("Registered"), Some(&1));
        assert_eq!(stats.by_state.get("Enabled"), Some(&1));
    }

    #[test]
    fn registry_resource_shutdown_is_explicit() {
        let registry = InMemoryRegistry::<DemoCapability>::new();
        registry
            .register(DemoCapability {
                id: "capability.a".into(),
                metadata: json!({}),
            })
            .unwrap();

        assert_eq!(registry.active_leases(), 1);
        assert!(registry.shutdown(ShutdownMode::Graceful).is_err());
        registry.shutdown(ShutdownMode::Immediate).unwrap();
        assert_eq!(registry.active_leases(), 0);
        assert_eq!(registry.stats().entry_count, 0);
    }

    #[test]
    fn replace_updates_item_and_preserves_lifecycle_state() {
        let registry = InMemoryRegistry::<DemoCapability>::new();
        registry
            .register(DemoCapability {
                id: "capability.a".into(),
                metadata: json!({ "generation": 1 }),
            })
            .unwrap();
        registry
            .lifecycle("capability.a", LifecycleAction::Enable)
            .unwrap();

        registry
            .replace(DemoCapability {
                id: "capability.a".into(),
                metadata: json!({ "generation": 2 }),
            })
            .unwrap();

        assert!(registry.get("capability.a").is_some());
        let info = registry.info("capability.a").unwrap();
        assert_eq!(info.state, LifecycleState::Enabled);
        assert_eq!(
            info.metadata.get("generation").and_then(Value::as_i64),
            Some(2)
        );
    }

    #[test]
    fn replace_reset_updates_item_and_resets_lifecycle_state() {
        let registry = InMemoryRegistry::<DemoCapability>::new();
        registry
            .register(DemoCapability {
                id: "capability.a".into(),
                metadata: json!({ "generation": 1 }),
            })
            .unwrap();
        registry
            .lifecycle("capability.a", LifecycleAction::Enable)
            .unwrap();

        registry
            .replace_reset(DemoCapability {
                id: "capability.a".into(),
                metadata: json!({ "generation": 2 }),
            })
            .unwrap();

        assert!(registry.get("capability.a").is_none());
        let info = registry.info("capability.a").unwrap();
        assert_eq!(info.state, LifecycleState::Registered);
        assert_eq!(
            info.metadata.get("generation").and_then(Value::as_i64),
            Some(2)
        );
    }

    #[test]
    fn duplicate_registration_fails() {
        let registry = InMemoryRegistry::<DemoCapability>::new();
        registry
            .register(DemoCapability {
                id: "same".into(),
                metadata: json!({}),
            })
            .unwrap();
        let err = registry
            .register(DemoCapability {
                id: "same".into(),
                metadata: json!({}),
            })
            .unwrap_err();
        assert!(matches!(
            err,
            KernelError::CapabilityAlreadyRegistered { .. }
        ));
    }

    #[test]
    fn concurrent_duplicate_registration_fails_closed() {
        let registry = Arc::new(InMemoryRegistry::<DemoCapability>::new());
        let start = Arc::new(Barrier::new(9));
        let mut handles = Vec::new();

        for index in 0..8 {
            let registry = registry.clone();
            let start = start.clone();
            handles.push(std::thread::spawn(move || {
                start.wait();
                registry.register(DemoCapability {
                    id: "same.concurrent".into(),
                    metadata: json!({ "attempt": index }),
                })
            }));
        }

        start.wait();
        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        let success_count = results.iter().filter(|result| result.is_ok()).count();
        let duplicate_count = results
            .iter()
            .filter(|result| matches!(result, Err(KernelError::CapabilityAlreadyRegistered { .. })))
            .count();

        assert_eq!(success_count, 1);
        assert_eq!(duplicate_count, 7);
        assert_eq!(registry.stats().entry_count, 1);
    }

    #[tokio::test]
    async fn async_lifecycle_runs_capability_hooks() {
        let registry = InMemoryRegistry::<LifecycleCapability>::new();
        let before_count = Arc::new(AtomicUsize::new(0));
        let after_count = Arc::new(AtomicUsize::new(0));
        registry
            .register(LifecycleCapability {
                id: "capability.lifecycle".into(),
                metadata: json!({}),
                before_count: before_count.clone(),
                after_count: after_count.clone(),
            })
            .unwrap();

        let state = registry
            .lifecycle_async(
                "capability.lifecycle",
                LifecycleAction::Enable,
                &KernelContext::new("test", super::super::KernelScope::global()),
            )
            .await
            .unwrap();

        assert_eq!(state, LifecycleState::Enabled);
        assert_eq!(before_count.load(Ordering::SeqCst), 1);
        assert_eq!(after_count.load(Ordering::SeqCst), 1);
        assert!(registry.get("capability.lifecycle").is_some());
    }

    #[test]
    fn register_returns_id_and_unregister_removes_entry() {
        let registry = InMemoryRegistry::<DemoCapability>::new();
        let id = registry
            .register(DemoCapability {
                id: "capability.a".into(),
                metadata: json!({}),
            })
            .unwrap();

        assert_eq!(id.as_str(), "capability.a");
        assert!(registry.is_registered("capability.a"));

        registry.unregister("capability.a").unwrap();

        assert!(!registry.is_registered("capability.a"));
        assert!(registry.get("capability.a").is_none());
    }

    #[test]
    fn unregistered_id_can_be_registered_again() {
        let registry = InMemoryRegistry::<DemoCapability>::new();
        registry
            .register(DemoCapability {
                id: "capability.a".into(),
                metadata: json!({}),
            })
            .unwrap();
        registry.unregister("capability.a").unwrap();

        registry
            .register(DemoCapability {
                id: "capability.a".into(),
                metadata: json!({ "generation": 2 }),
            })
            .unwrap();

        assert!(registry.is_registered("capability.a"));
    }

    #[test]
    fn removing_lifecycle_unregisters_entry() {
        let registry = InMemoryRegistry::<DemoCapability>::new();
        registry
            .register(DemoCapability {
                id: "capability.a".into(),
                metadata: json!({}),
            })
            .unwrap();

        let state = registry
            .lifecycle("capability.a", LifecycleAction::Remove)
            .unwrap();

        assert_eq!(state, LifecycleState::Removed);
        assert!(!registry.is_registered("capability.a"));
    }
}
