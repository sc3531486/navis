use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use super::{
    KernelError, KernelObjectInfo, KernelObjectState, KernelResource, KernelResult, ShutdownMode,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyInput {
    pub subject: String,
    pub action: String,
    pub target: String,
    pub scope: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCheckpoint {
    pub id: String,
    pub input: PolicyInput,
}

impl PolicyCheckpoint {
    pub fn new(
        id: impl Into<String>,
        subject: impl Into<String>,
        action: impl Into<String>,
        target: impl Into<String>,
        scope: impl Into<String>,
        metadata: Value,
    ) -> Self {
        Self {
            id: id.into(),
            input: PolicyInput {
                subject: subject.into(),
                action: action.into(),
                target: target.into(),
                scope: scope.into(),
                metadata,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PolicyDecision {
    Allow { reason: String },
    Ask { prompt: String, grant_spec: Value },
    Deny { reason: String },
}

pub trait Constraint: Send + Sync {
    fn id(&self) -> &str;
    fn priority(&self) -> i32;
    fn evaluate(&self, input: &PolicyInput) -> Option<PolicyDecision>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstraintInfo {
    pub id: String,
    pub priority: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyStats {
    pub constraint_count: usize,
    pub min_priority: Option<i32>,
    pub max_priority: Option<i32>,
}

impl ConstraintInfo {
    pub fn object_info(&self) -> KernelObjectInfo {
        KernelObjectInfo::new(
            self.id.clone(),
            "policy.constraint",
            KernelObjectState::Enabled,
            "global",
        )
        .with_metadata(json!({ "priority": self.priority }))
    }
}

#[derive(Clone)]
struct ConstraintEntry {
    constraint: Arc<dyn Constraint>,
    info: ConstraintInfo,
}

impl ConstraintEntry {
    fn new(constraint: Arc<dyn Constraint>) -> Self {
        Self {
            info: ConstraintInfo {
                id: constraint.id().to_string(),
                priority: constraint.priority(),
            },
            constraint,
        }
    }
}

pub struct PolicyEngine {
    constraints: RwLock<Vec<ConstraintEntry>>,
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self {
            constraints: RwLock::new(Vec::new()),
        }
    }

    /// Creates an isolated policy view from the current shared constraints.
    ///
    /// The returned engine owns its own ordered constraint list, while the
    /// individual constraints remain cheaply shared through `Arc`. This is
    /// intended for request-scoped policy additions that must not mutate the
    /// application-wide policy engine.
    pub fn overlay<I>(&self, constraints: I) -> KernelResult<Self>
    where
        I: IntoIterator<Item = Arc<dyn Constraint>>,
    {
        let overlay = Self {
            constraints: RwLock::new(self.constraints.read().clone()),
        };

        for constraint in constraints {
            overlay.add_arc(constraint)?;
        }

        Ok(overlay)
    }

    pub fn add(&self, constraint: impl Constraint + 'static) -> KernelResult<()> {
        self.add_arc(Arc::new(constraint))
    }

    pub fn add_arc(&self, constraint: Arc<dyn Constraint>) -> KernelResult<()> {
        tracing::debug!(
            policy_id = %constraint.id(),
            priority = constraint.priority(),
            "adding policy constraint"
        );
        let mut constraints = self.constraints.write();
        let entry = ConstraintEntry::new(constraint);
        if constraints
            .iter()
            .any(|existing| existing.info.id == entry.info.id)
        {
            return Err(KernelError::policy_constraint_already_registered(
                entry.info.id,
            ));
        }
        constraints.push(entry);
        constraints.sort_by_key(|entry| entry.info.priority);
        Ok(())
    }

    pub fn replace(&self, constraint: impl Constraint + 'static) -> KernelResult<()> {
        self.replace_arc(Arc::new(constraint))
    }

    pub fn replace_arc(&self, constraint: Arc<dyn Constraint>) -> KernelResult<()> {
        tracing::debug!(
            policy_id = %constraint.id(),
            priority = constraint.priority(),
            "replacing policy constraint"
        );
        let mut constraints = self.constraints.write();
        let entry = ConstraintEntry::new(constraint);
        constraints.retain(|existing| existing.info.id != entry.info.id);
        constraints.push(entry);
        constraints.sort_by_key(|entry| entry.info.priority);
        Ok(())
    }

    pub fn remove(&self, constraint_id: &str) -> KernelResult<()> {
        tracing::debug!(policy_id = %constraint_id, "removing policy constraint");
        let mut constraints = self.constraints.write();
        let before = constraints.len();
        constraints.retain(|entry| entry.info.id != constraint_id);
        if constraints.len() == before {
            return Err(KernelError::policy_constraint_not_found(constraint_id));
        }
        Ok(())
    }

    pub fn contains(&self, constraint_id: &str) -> bool {
        self.constraints
            .read()
            .iter()
            .any(|entry| entry.info.id == constraint_id)
    }

    pub fn list(&self) -> Vec<ConstraintInfo> {
        self.constraints
            .read()
            .iter()
            .map(|entry| entry.info.clone())
            .collect()
    }

    pub fn objects(&self) -> Vec<KernelObjectInfo> {
        self.list()
            .into_iter()
            .map(|info| info.object_info())
            .collect()
    }

    pub fn stats(&self) -> PolicyStats {
        let constraints = self.constraints.read();
        PolicyStats {
            constraint_count: constraints.len(),
            min_priority: constraints.first().map(|entry| entry.info.priority),
            max_priority: constraints.last().map(|entry| entry.info.priority),
        }
    }

    pub fn evaluate(&self, input: &PolicyInput) -> PolicyDecision {
        let span = tracing::debug_span!(
            "kernel.policy.evaluate",
            subject = %input.subject,
            action = %input.action,
            target = %input.target,
            scope = %input.scope
        );
        let _entered = span.enter();

        let constraints = self.constraints.read();

        for entry in constraints.iter() {
            if let Some(decision) = entry.constraint.evaluate(input) {
                tracing::debug!(
                    policy_id = %entry.info.id,
                    decision = ?decision,
                    "policy constraint matched"
                );
                return decision;
            }
        }

        tracing::warn!("no policy constraint matched");
        PolicyDecision::Deny {
            reason: "no policy constraint matched".into(),
        }
    }

    pub fn evaluate_checkpoint(&self, checkpoint: &PolicyCheckpoint) -> PolicyDecision {
        let span = tracing::debug_span!(
            "kernel.policy.checkpoint",
            checkpoint_id = %checkpoint.id,
            action = %checkpoint.input.action,
            scope = %checkpoint.input.scope
        );
        let _entered = span.enter();
        self.evaluate(&checkpoint.input)
    }

    pub fn ensure_allowed(&self, input: &PolicyInput) -> KernelResult<()> {
        match self.evaluate(input) {
            PolicyDecision::Allow { .. } => Ok(()),
            PolicyDecision::Ask { prompt, grant_spec } => Err(
                KernelError::policy_requires_approval_with_grant(prompt, grant_spec),
            ),
            PolicyDecision::Deny { reason } => Err(KernelError::policy_denied(reason)),
        }
    }

    pub fn ensure_checkpoint_allowed(&self, checkpoint: &PolicyCheckpoint) -> KernelResult<()> {
        match self.evaluate_checkpoint(checkpoint) {
            PolicyDecision::Allow { .. } => Ok(()),
            PolicyDecision::Ask { prompt, grant_spec } => Err(
                KernelError::policy_requires_approval_with_grant(prompt, grant_spec),
            ),
            PolicyDecision::Deny { reason } => Err(KernelError::policy_denied(reason)),
        }
    }

    pub fn len(&self) -> usize {
        self.constraints.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl KernelResource for PolicyEngine {
    fn object_info(&self) -> KernelObjectInfo {
        let stats = self.stats();
        KernelObjectInfo::new(
            "policy.engine",
            "policy.engine",
            KernelObjectState::Enabled,
            "global",
        )
        .with_metadata(json!({
            "constraintCount": stats.constraint_count,
            "minPriority": stats.min_priority,
            "maxPriority": stats.max_priority,
        }))
    }

    fn active_leases(&self) -> usize {
        self.stats().constraint_count
    }

    fn shutdown(&self, _mode: ShutdownMode) -> KernelResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::PolicyErrorKind;
    use serde_json::json;

    struct MatchAction {
        id: &'static str,
        action: &'static str,
        decision: PolicyDecision,
        priority: i32,
    }

    impl Constraint for MatchAction {
        fn id(&self) -> &str {
            self.id
        }

        fn priority(&self) -> i32 {
            self.priority
        }

        fn evaluate(&self, input: &PolicyInput) -> Option<PolicyDecision> {
            (input.action == self.action).then(|| self.decision.clone())
        }
    }

    fn input(action: &str) -> PolicyInput {
        PolicyInput {
            subject: "subject".into(),
            action: action.into(),
            target: "target".into(),
            scope: "global".into(),
            metadata: json!({}),
        }
    }

    #[test]
    fn no_match_denies() {
        let policy = PolicyEngine::new();
        let decision = policy.evaluate(&input("unknown"));
        assert!(matches!(decision, PolicyDecision::Deny { .. }));
    }

    #[test]
    fn lower_priority_number_wins() {
        let policy = PolicyEngine::new();
        policy
            .add(MatchAction {
                id: "late-write",
                action: "write",
                decision: PolicyDecision::Allow {
                    reason: "late".into(),
                },
                priority: 20,
            })
            .unwrap();
        policy
            .add(MatchAction {
                id: "early-write",
                action: "write",
                decision: PolicyDecision::Deny {
                    reason: "early".into(),
                },
                priority: 1,
            })
            .unwrap();

        assert_eq!(
            policy.evaluate(&input("write")),
            PolicyDecision::Deny {
                reason: "early".into()
            }
        );
    }

    #[test]
    fn remove_constraint_stops_matching_it() {
        let policy = PolicyEngine::new();
        policy
            .add(MatchAction {
                id: "read",
                action: "read",
                decision: PolicyDecision::Allow {
                    reason: "ok".into(),
                },
                priority: 1,
            })
            .unwrap();

        assert!(matches!(
            policy.evaluate(&input("read")),
            PolicyDecision::Allow { .. }
        ));
        policy.remove("read").unwrap();
        assert!(matches!(
            policy.evaluate(&input("read")),
            PolicyDecision::Deny { .. }
        ));
    }

    #[test]
    fn list_and_contains_expose_registered_constraints() {
        let policy = PolicyEngine::new();
        policy
            .add(MatchAction {
                id: "read",
                action: "read",
                decision: PolicyDecision::Allow {
                    reason: "ok".into(),
                },
                priority: 10,
            })
            .unwrap();

        assert!(policy.contains("read"));
        assert_eq!(
            policy.list(),
            vec![ConstraintInfo {
                id: "read".into(),
                priority: 10
            }]
        );
    }

    #[test]
    fn policy_exports_kernel_object_info() {
        let policy = PolicyEngine::new();
        policy
            .add(MatchAction {
                id: "read",
                action: "read",
                decision: PolicyDecision::Allow {
                    reason: "ok".into(),
                },
                priority: 10,
            })
            .unwrap();

        let objects = policy.objects();
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].id, "read");
        assert_eq!(objects[0].kind, "policy.constraint");
        assert_eq!(objects[0].state, KernelObjectState::Enabled);
        assert_eq!(
            objects[0].metadata.get("priority").and_then(Value::as_i64),
            Some(10)
        );
    }

    #[test]
    fn policy_stats_counts_constraints_and_priority_range() {
        let policy = PolicyEngine::new();
        policy
            .add(MatchAction {
                id: "read",
                action: "read",
                decision: PolicyDecision::Allow {
                    reason: "ok".into(),
                },
                priority: 10,
            })
            .unwrap();
        policy
            .add(MatchAction {
                id: "write",
                action: "write",
                decision: PolicyDecision::Deny {
                    reason: "no".into(),
                },
                priority: 30,
            })
            .unwrap();

        let stats = policy.stats();
        assert_eq!(stats.constraint_count, 2);
        assert_eq!(stats.min_priority, Some(10));
        assert_eq!(stats.max_priority, Some(30));
    }

    #[test]
    fn evaluates_policy_checkpoint() {
        let policy = PolicyEngine::new();
        policy
            .add(MatchAction {
                id: "checkpoint",
                action: "checkpoint.run",
                decision: PolicyDecision::Allow {
                    reason: "ok".into(),
                },
                priority: 1,
            })
            .unwrap();
        let checkpoint = PolicyCheckpoint::new(
            "checkpoint.a",
            "subject",
            "checkpoint.run",
            "target",
            "global",
            json!({}),
        );

        assert_eq!(
            policy.evaluate_checkpoint(&checkpoint),
            PolicyDecision::Allow {
                reason: "ok".into()
            }
        );
        assert!(policy.ensure_checkpoint_allowed(&checkpoint).is_ok());
    }

    #[test]
    fn ask_decision_preserves_grant_spec_in_error() {
        let policy = PolicyEngine::new();
        policy
            .add(MatchAction {
                id: "ask-write",
                action: "write",
                decision: PolicyDecision::Ask {
                    prompt: "confirm write".into(),
                    grant_spec: json!({ "permission": "write", "target": "file.rs" }),
                },
                priority: 1,
            })
            .unwrap();

        let error = policy.ensure_allowed(&input("write")).unwrap_err();

        assert!(matches!(
            error,
            KernelError::Policy {
                kind: PolicyErrorKind::RequiresApproval,
                ..
            }
        ));
        assert_eq!(
            error.policy_grant_spec(),
            Some(&json!({ "permission": "write", "target": "file.rs" }))
        );
    }

    #[test]
    fn replace_updates_constraint_without_external_index() {
        let policy = PolicyEngine::new();
        policy
            .add(MatchAction {
                id: "write",
                action: "write",
                decision: PolicyDecision::Allow {
                    reason: "old".into(),
                },
                priority: 20,
            })
            .unwrap();
        policy
            .replace(MatchAction {
                id: "write",
                action: "write",
                decision: PolicyDecision::Deny {
                    reason: "new".into(),
                },
                priority: 5,
            })
            .unwrap();

        assert_eq!(policy.len(), 1);
        assert_eq!(policy.list()[0].priority, 5);
        assert_eq!(
            policy.evaluate(&input("write")),
            PolicyDecision::Deny {
                reason: "new".into()
            }
        );
    }

    #[test]
    fn overlay_inherits_a_point_in_time_policy_without_mutating_shared_engine() {
        let policy = PolicyEngine::new();
        policy
            .add(MatchAction {
                id: "base",
                action: "read",
                decision: PolicyDecision::Allow {
                    reason: "base allow".into(),
                },
                priority: 10,
            })
            .unwrap();

        let overlay = policy
            .overlay([Arc::new(MatchAction {
                id: "request",
                action: "write",
                decision: PolicyDecision::Deny {
                    reason: "request deny".into(),
                },
                priority: 5,
            }) as Arc<dyn Constraint>])
            .unwrap();

        assert_eq!(policy.len(), 1);
        assert!(!policy.contains("request"));
        assert_eq!(overlay.len(), 2);
        assert_eq!(
            overlay.evaluate(&input("write")),
            PolicyDecision::Deny {
                reason: "request deny".into()
            }
        );
        assert_eq!(
            overlay.evaluate(&input("read")),
            PolicyDecision::Allow {
                reason: "base allow".into()
            }
        );
    }

    #[test]
    fn overlays_are_isolated_when_created_concurrently() {
        let policy = Arc::new(PolicyEngine::new());
        policy
            .add(MatchAction {
                id: "base",
                action: "read",
                decision: PolicyDecision::Allow {
                    reason: "base allow".into(),
                },
                priority: 10,
            })
            .unwrap();

        std::thread::scope(|scope| {
            for _ in 0..16 {
                let policy = Arc::clone(&policy);
                scope.spawn(move || {
                    let overlay = policy
                        .overlay([Arc::new(MatchAction {
                            id: "request",
                            action: "write",
                            decision: PolicyDecision::Deny {
                                reason: "request deny".into(),
                            },
                            priority: 5,
                        }) as Arc<dyn Constraint>])
                        .unwrap();

                    assert_eq!(overlay.len(), 2);
                    assert_eq!(
                        overlay.evaluate(&input("write")),
                        PolicyDecision::Deny {
                            reason: "request deny".into()
                        }
                    );
                });
            }
        });

        assert_eq!(policy.len(), 1);
        assert!(!policy.contains("request"));
    }

    #[test]
    fn overlay_keeps_the_shared_policy_snapshot_after_later_replacement() {
        let policy = PolicyEngine::new();
        policy
            .add(MatchAction {
                id: "read-policy",
                action: "read",
                decision: PolicyDecision::Allow {
                    reason: "before replacement".into(),
                },
                priority: 10,
            })
            .unwrap();

        let overlay = policy.overlay(std::iter::empty()).unwrap();
        policy
            .replace(MatchAction {
                id: "read-policy",
                action: "read",
                decision: PolicyDecision::Deny {
                    reason: "after replacement".into(),
                },
                priority: 10,
            })
            .unwrap();

        assert_eq!(
            overlay.evaluate(&input("read")),
            PolicyDecision::Allow {
                reason: "before replacement".into()
            }
        );
        assert_eq!(
            policy.evaluate(&input("read")),
            PolicyDecision::Deny {
                reason: "after replacement".into()
            }
        );
    }

    #[test]
    fn duplicate_constraint_id_is_rejected() {
        let policy = PolicyEngine::new();
        policy
            .add(MatchAction {
                id: "duplicate",
                action: "read",
                decision: PolicyDecision::Allow {
                    reason: "first".into(),
                },
                priority: 1,
            })
            .unwrap();

        let err = policy
            .add(MatchAction {
                id: "duplicate",
                action: "write",
                decision: PolicyDecision::Deny {
                    reason: "second".into(),
                },
                priority: 2,
            })
            .unwrap_err();

        assert!(matches!(
            err,
            KernelError::Policy {
                kind: PolicyErrorKind::ConstraintAlreadyRegistered,
                ..
            }
        ));
    }
}
