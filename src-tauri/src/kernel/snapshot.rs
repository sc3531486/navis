use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{
    AuditRecorder, AuditStats, Capability, EventBus, EventBusStats, KernelObjectInfo, Pipeline,
    PipelineStats, PolicyEngine, PolicyStats, Registry, RegistryStats,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelSnapshot {
    pub captured_at: DateTime<Utc>,
    pub objects: Vec<KernelObjectInfo>,
    pub registries: Vec<RegistryStats>,
    pub pipelines: Vec<PipelineStats>,
    pub event_buses: Vec<EventBusStats>,
    pub policies: Vec<PolicyStats>,
    pub audits: Vec<AuditStats>,
}

impl Default for KernelSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

impl KernelSnapshot {
    pub fn new() -> Self {
        Self {
            captured_at: Utc::now(),
            objects: Vec::new(),
            registries: Vec::new(),
            pipelines: Vec::new(),
            event_buses: Vec::new(),
            policies: Vec::new(),
            audits: Vec::new(),
        }
    }

    pub fn with_registry<T: Capability + ?Sized>(mut self, registry: &impl Registry<T>) -> Self {
        self.objects.extend(registry.objects());
        self.registries.push(registry.stats());
        self
    }

    pub fn with_pipeline(mut self, pipeline: &Pipeline) -> Self {
        self.objects.extend(pipeline.objects());
        self.pipelines.push(pipeline.stats());
        self
    }

    pub fn with_event_bus(mut self, event_bus: &dyn EventBus) -> Self {
        self.objects.extend(event_bus.objects());
        self.event_buses.push(event_bus.stats());
        self
    }

    pub fn with_policy(mut self, policy: &PolicyEngine) -> Self {
        self.objects.extend(policy.objects());
        self.policies.push(policy.stats());
        self
    }

    pub fn with_audit(mut self, audit: &AuditRecorder) -> Self {
        self.audits.push(audit.stats());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::{InMemoryRegistry, LifecycleAction, Registry, SchemaVersion};
    use serde_json::{json, Value};

    struct SnapshotCapability {
        id: String,
        metadata: Value,
    }

    impl Capability for SnapshotCapability {
        fn id(&self) -> &str {
            &self.id
        }

        fn kind(&self) -> &str {
            "snapshot"
        }

        fn version(&self) -> SchemaVersion {
            SchemaVersion::default()
        }

        fn metadata(&self) -> &Value {
            &self.metadata
        }
    }

    #[test]
    fn snapshot_collects_registry_pipeline_policy_and_audit_stats() {
        let registry = InMemoryRegistry::<SnapshotCapability>::new();
        registry
            .register(SnapshotCapability {
                id: "capability.snapshot".into(),
                metadata: json!({}),
            })
            .unwrap();
        registry
            .lifecycle("capability.snapshot", LifecycleAction::Enable)
            .unwrap();
        let pipeline = Pipeline::new();
        let policy = PolicyEngine::new();
        let audit = AuditRecorder::disabled();

        let snapshot = KernelSnapshot::new()
            .with_registry(&registry)
            .with_pipeline(&pipeline)
            .with_policy(&policy)
            .with_audit(&audit);

        assert_eq!(snapshot.registries.len(), 1);
        assert_eq!(snapshot.registries[0].entry_count, 1);
        assert_eq!(snapshot.pipelines.len(), 1);
        assert_eq!(snapshot.policies.len(), 1);
        assert_eq!(snapshot.audits.len(), 1);
        assert!(snapshot
            .objects
            .iter()
            .any(|object| object.id == "capability.snapshot"));
    }
}
