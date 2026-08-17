use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub type KernelResult<T> = Result<T, KernelError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyErrorKind {
    Denied,
    RequiresApproval,
    Undecidable,
    ConstraintNotFound,
    ConstraintAlreadyRegistered,
}

impl std::fmt::Display for PolicyErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Denied => write!(f, "denied"),
            Self::RequiresApproval => write!(f, "requires_approval"),
            Self::Undecidable => write!(f, "undecidable"),
            Self::ConstraintNotFound => write!(f, "constraint_not_found"),
            Self::ConstraintAlreadyRegistered => write!(f, "constraint_already_registered"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KernelErrorKind {
    NotFound,
    AlreadyExists,
    NotEnabled,
    RequiredMissing,
    TypeMismatch,
    Cancelled,
    Deadline,
    Policy,
    RequiresApproval,
    Version,
    Payload,
    Resource,
    Transient,
    Invariant,
    Internal,
}

impl std::fmt::Display for KernelErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "not_found"),
            Self::AlreadyExists => write!(f, "already_exists"),
            Self::NotEnabled => write!(f, "not_enabled"),
            Self::RequiredMissing => write!(f, "required_missing"),
            Self::TypeMismatch => write!(f, "type_mismatch"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Deadline => write!(f, "deadline"),
            Self::Policy => write!(f, "policy"),
            Self::RequiresApproval => write!(f, "requires_approval"),
            Self::Version => write!(f, "version"),
            Self::Payload => write!(f, "payload"),
            Self::Resource => write!(f, "resource"),
            Self::Transient => write!(f, "transient"),
            Self::Invariant => write!(f, "invariant"),
            Self::Internal => write!(f, "internal"),
        }
    }
}

impl KernelErrorKind {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            KernelErrorKind::Transient | KernelErrorKind::Resource | KernelErrorKind::Deadline
        )
    }

    pub fn is_policy_error(&self) -> bool {
        matches!(
            self,
            KernelErrorKind::Policy | KernelErrorKind::RequiresApproval
        )
    }

    pub fn is_invariant_violation(&self) -> bool {
        matches!(
            self,
            KernelErrorKind::AlreadyExists
                | KernelErrorKind::RequiredMissing
                | KernelErrorKind::TypeMismatch
                | KernelErrorKind::Version
                | KernelErrorKind::Payload
                | KernelErrorKind::Invariant
        )
    }
}

#[derive(Debug, Error)]
pub enum KernelError {
    #[error("capability not found: {id}")]
    CapabilityNotFound { id: String },

    #[error("capability already registered: {id}")]
    CapabilityAlreadyRegistered { id: String },

    #[error("capability is not enabled: {id}")]
    CapabilityNotEnabled { id: String },

    #[error("required stage is missing: {id}")]
    RequiredStageMissing { id: String },

    #[error("stage failed: {id} ({kind}): {message}")]
    StageFailed {
        id: String,
        kind: KernelErrorKind,
        message: String,
    },

    #[error("payload type mismatch: expected {expected}, actual {actual}")]
    PayloadTypeMismatch {
        expected: &'static str,
        actual: &'static str,
    },

    #[error("operation cancelled")]
    Cancelled,

    #[error("deadline exceeded")]
    DeadlineExceeded,

    #[error("policy {kind:?}: {detail}")]
    Policy {
        kind: PolicyErrorKind,
        detail: String,
        grant_spec: Option<Value>,
    },

    #[error("event subscription not found: {id}")]
    EventSubscriptionNotFound { id: String },

    #[error("version mismatch: expected {expected}, actual {actual}")]
    VersionMismatch { expected: String, actual: String },

    #[error("event payload is unsupported: topic={topic}, version={version}")]
    UnsupportedEventPayload { topic: String, version: String },

    #[error("audit sink failed: {message}")]
    AuditSinkFailed { message: String },

    #[error("transient failure: {message}")]
    TransientFailure { message: String },

    #[error("{message}")]
    InvalidInput { message: String },
}

impl KernelError {
    pub fn cancelled() -> Self {
        Self::Cancelled
    }

    pub fn deadline_exceeded() -> Self {
        Self::DeadlineExceeded
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput {
            message: message.into(),
        }
    }

    pub fn transient(message: impl Into<String>) -> Self {
        Self::TransientFailure {
            message: message.into(),
        }
    }

    pub fn kind(&self) -> KernelErrorKind {
        match self {
            Self::CapabilityNotFound { .. }
            | Self::Policy {
                kind: PolicyErrorKind::ConstraintNotFound,
                ..
            }
            | Self::EventSubscriptionNotFound { .. } => KernelErrorKind::NotFound,
            Self::CapabilityAlreadyRegistered { .. }
            | Self::Policy {
                kind: PolicyErrorKind::ConstraintAlreadyRegistered,
                ..
            } => KernelErrorKind::AlreadyExists,
            Self::CapabilityNotEnabled { .. } => KernelErrorKind::NotEnabled,
            Self::RequiredStageMissing { .. } => KernelErrorKind::RequiredMissing,
            Self::StageFailed { kind, .. } => *kind,
            Self::PayloadTypeMismatch { .. } => KernelErrorKind::TypeMismatch,
            Self::Cancelled => KernelErrorKind::Cancelled,
            Self::DeadlineExceeded => KernelErrorKind::Deadline,
            Self::Policy {
                kind: PolicyErrorKind::Denied,
                ..
            }
            | Self::Policy {
                kind: PolicyErrorKind::Undecidable,
                ..
            } => KernelErrorKind::Policy,
            Self::Policy {
                kind: PolicyErrorKind::RequiresApproval,
                ..
            } => KernelErrorKind::RequiresApproval,
            Self::VersionMismatch { .. } => KernelErrorKind::Version,
            Self::UnsupportedEventPayload { .. } => KernelErrorKind::Payload,
            Self::AuditSinkFailed { .. } => KernelErrorKind::Resource,
            Self::TransientFailure { .. } => KernelErrorKind::Transient,
            Self::InvalidInput { .. } => KernelErrorKind::Invariant,
        }
    }

    pub fn is_retryable(&self) -> bool {
        self.kind().is_retryable()
    }

    pub fn is_policy_error(&self) -> bool {
        self.kind().is_policy_error()
    }

    pub fn is_invariant_violation(&self) -> bool {
        self.kind().is_invariant_violation()
    }

    pub fn policy_denied(reason: impl Into<String>) -> Self {
        Self::Policy {
            kind: PolicyErrorKind::Denied,
            detail: reason.into(),
            grant_spec: None,
        }
    }

    pub fn policy_requires_approval(reason: impl Into<String>) -> Self {
        Self::Policy {
            kind: PolicyErrorKind::RequiresApproval,
            detail: reason.into(),
            grant_spec: None,
        }
    }

    pub fn policy_requires_approval_with_grant(
        reason: impl Into<String>,
        grant_spec: Value,
    ) -> Self {
        Self::Policy {
            kind: PolicyErrorKind::RequiresApproval,
            detail: reason.into(),
            grant_spec: Some(grant_spec),
        }
    }

    pub fn policy_undecidable(reason: impl Into<String>) -> Self {
        Self::Policy {
            kind: PolicyErrorKind::Undecidable,
            detail: reason.into(),
            grant_spec: None,
        }
    }

    pub fn policy_constraint_not_found(id: impl Into<String>) -> Self {
        Self::Policy {
            kind: PolicyErrorKind::ConstraintNotFound,
            detail: id.into(),
            grant_spec: None,
        }
    }

    pub fn policy_constraint_already_registered(id: impl Into<String>) -> Self {
        Self::Policy {
            kind: PolicyErrorKind::ConstraintAlreadyRegistered,
            detail: id.into(),
            grant_spec: None,
        }
    }

    pub fn policy_grant_spec(&self) -> Option<&Value> {
        match self {
            Self::Policy { grant_spec, .. } => grant_spec.as_ref(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_lookup_and_duplicate_errors() {
        assert_eq!(
            KernelError::CapabilityNotFound { id: "a".into() }.kind(),
            KernelErrorKind::NotFound
        );
        assert_eq!(
            KernelError::policy_constraint_already_registered("p").kind(),
            KernelErrorKind::AlreadyExists
        );
        assert!(KernelError::policy_constraint_already_registered("p").is_invariant_violation());
    }

    #[test]
    fn classifies_policy_errors() {
        let denied = KernelError::policy_denied("no");
        let approval = KernelError::policy_requires_approval("ask");

        assert!(denied.is_policy_error());
        assert!(approval.is_policy_error());
        assert!(!denied.is_retryable());
        assert!(!approval.is_retryable());
    }

    #[test]
    fn classifies_retryable_errors() {
        assert!(KernelError::StageFailed {
            id: "stage".into(),
            kind: KernelErrorKind::Transient,
            message: "temporary".into()
        }
        .is_retryable());

        assert!(!KernelError::StageFailed {
            id: "policy".into(),
            kind: KernelErrorKind::Policy,
            message: "denied".into()
        }
        .is_retryable());
        assert!(KernelError::AuditSinkFailed {
            message: "queue full".into()
        }
        .is_retryable());
        assert!(KernelError::transient("temporary").is_retryable());
        assert!(KernelError::DeadlineExceeded.is_retryable());
    }
}
