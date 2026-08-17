use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use triomphe::Arc as SharedArc;

use super::KernelResult;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KernelObjectState {
    Registered,
    Enabled,
    Running,
    Stopping,
    Removed,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelObjectInfo {
    pub id: String,
    pub kind: String,
    pub state: KernelObjectState,
    pub scope: String,
    pub owner: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub metadata: SharedArc<Value>,
}

impl KernelObjectInfo {
    pub fn new(
        id: impl Into<String>,
        kind: impl Into<String>,
        state: KernelObjectState,
        scope: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            state,
            scope: scope.into(),
            owner: None,
            created_at: Utc::now(),
            updated_at: None,
            metadata: SharedArc::new(Value::Null),
        }
    }

    pub fn with_owner(mut self, owner: impl Into<String>) -> Self {
        self.owner = Some(owner.into());
        self
    }

    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = SharedArc::new(metadata);
        self
    }

    pub fn with_updated_at_now(mut self) -> Self {
        self.updated_at = Some(Utc::now());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShutdownMode {
    Graceful,
    Deadline(DateTime<Utc>),
    Immediate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLease {
    pub id: String,
    pub object_id: String,
    pub owner: Option<String>,
    pub acquired_at: DateTime<Utc>,
}

impl ResourceLease {
    pub fn new(object_id: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::now_v7().to_string(),
            object_id: object_id.into(),
            owner: None,
            acquired_at: Utc::now(),
        }
    }

    pub fn with_owner(mut self, owner: impl Into<String>) -> Self {
        self.owner = Some(owner.into());
        self
    }
}

pub trait KernelResource: Send + Sync {
    fn object_info(&self) -> KernelObjectInfo;
    fn active_leases(&self) -> usize;
    fn shutdown(&self, mode: ShutdownMode) -> KernelResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn object_info_carries_identity_state_and_metadata() {
        let object = KernelObjectInfo::new(
            "object.a",
            "registry.entry",
            KernelObjectState::Registered,
            "global",
        )
        .with_owner("owner.a")
        .with_metadata(json!({ "kind": "demo" }))
        .with_updated_at_now();

        assert_eq!(object.id, "object.a");
        assert_eq!(object.kind, "registry.entry");
        assert_eq!(object.state, KernelObjectState::Registered);
        assert_eq!(object.scope, "global");
        assert_eq!(object.owner.as_deref(), Some("owner.a"));
        assert_eq!(
            object.metadata.get("kind").and_then(Value::as_str),
            Some("demo")
        );
        assert!(object.updated_at.is_some());
    }

    #[test]
    fn resource_lease_uses_stable_generated_id() {
        let lease = ResourceLease::new("object.a").with_owner("owner.a");

        assert!(!lease.id.is_empty());
        assert_eq!(lease.object_id, "object.a");
        assert_eq!(lease.owner.as_deref(), Some("owner.a"));
    }
}
