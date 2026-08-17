use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use triomphe::Arc as SharedArc;

use super::id::TraceId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KernelScope {
    Global,
    Scoped { kind: String, id: String },
}

impl KernelScope {
    pub fn global() -> Self {
        Self::Global
    }

    pub fn scoped(kind: impl Into<String>, id: impl Into<String>) -> Self {
        Self::Scoped {
            kind: kind.into(),
            id: id.into(),
        }
    }

    pub fn key(&self) -> String {
        match self {
            Self::Global => "global".to_string(),
            Self::Scoped { kind, id } => format!("{kind}:{id}"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct KernelContext {
    pub trace_id: TraceId,
    pub scope: KernelScope,
    pub scope_key: String,
    pub source: String,
    pub started_at: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,
    pub owner: Option<String>,
    pub metadata: Option<SharedArc<Value>>,
}

impl<'de> Deserialize<'de> for KernelContext {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct KernelContextFields {
            trace_id: TraceId,
            scope: KernelScope,
            #[serde(default)]
            scope_key: String,
            source: String,
            started_at: DateTime<Utc>,
            deadline: Option<DateTime<Utc>>,
            owner: Option<String>,
            #[serde(default)]
            metadata: Option<Value>,
        }

        let mut fields = KernelContextFields::deserialize(deserializer)?;
        if fields.scope_key.is_empty() {
            fields.scope_key = fields.scope.key();
        }

        Ok(Self {
            trace_id: fields.trace_id,
            scope: fields.scope,
            scope_key: fields.scope_key,
            source: fields.source,
            started_at: fields.started_at,
            deadline: fields.deadline,
            owner: fields.owner,
            metadata: fields.metadata.map(SharedArc::new),
        })
    }
}

impl KernelContext {
    pub fn new(source: impl Into<String>, scope: KernelScope) -> Self {
        let scope_key = scope.key();
        Self {
            trace_id: TraceId::generate(),
            scope,
            scope_key,
            source: source.into(),
            started_at: Utc::now(),
            deadline: None,
            owner: None,
            metadata: None,
        }
    }

    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }

    pub fn with_owner(mut self, owner: impl Into<String>) -> Self {
        self.owner = Some(owner.into());
        self
    }

    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(SharedArc::new(metadata));
        self
    }

    pub fn scope_key(&self) -> String {
        if self.scope_key.is_empty() {
            self.scope.key()
        } else {
            self.scope_key.clone()
        }
    }

    pub fn scope_key_ref(&self) -> &str {
        debug_assert!(
            !self.scope_key.is_empty(),
            "scope_key should be cached in constructor"
        );
        &self.scope_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn scoped_key_is_stable() {
        let scope = KernelScope::scoped("worktree", "abc");
        assert_eq!(scope.key(), "worktree:abc");
    }

    #[test]
    fn context_caches_scope_key() {
        let context = KernelContext::new("test", KernelScope::scoped("worktree", "abc"));

        assert_eq!(context.scope_key(), "worktree:abc");
        assert_eq!(context.scope_key_ref(), "worktree:abc");
    }

    #[test]
    fn deserialized_context_rebuilds_missing_scope_key() {
        let value = json!({
            "trace_id": TraceId::generate(),
            "scope": { "Scoped": { "kind": "worktree", "id": "abc" } },
            "source": "test",
            "started_at": Utc::now(),
            "deadline": null,
            "owner": null,
            "metadata": {}
        });

        let context: KernelContext = serde_json::from_value(value).unwrap();

        assert_eq!(context.scope_key_ref(), "worktree:abc");
    }
}
