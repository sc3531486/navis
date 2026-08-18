//! Storage-backed Kernel audit sink.
//!
//! This module is not an application-side audit recorder or fact source. It is
//! the SQLite persistence adapter for records already produced by
//! `crate::kernel::AuditRecorder` and delivered through the Kernel `AuditSink`
//! contract.

use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::kernel::{AuditRecord, AuditSink, KernelError, KernelResult};

pub struct StorageAuditSink {
    connection: Arc<Mutex<Connection>>,
}

impl StorageAuditSink {
    pub fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self { connection }
    }
}

impl AuditSink for StorageAuditSink {
    fn record(&self, record: &AuditRecord) -> KernelResult<()> {
        tracing::trace!(
            audit_id = %record.id,
            trace_id = %record.trace_id,
            "writing audit entry to storage"
        );

        let connection = self
            .connection
            .lock()
            .map_err(|error| KernelError::AuditSinkFailed {
                message: format!("storage audit connection lock failed: {error}"),
            })?;

        connection
            .execute(
                "INSERT INTO audit_log (
                    id, schema_version, trace_id, span_id, parent_span_id,
                    scope, source, operation_id, action, policy_decision,
                    duration_ms, input_digest, output_digest, status, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                rusqlite::params![
                    record.id,
                    record.schema_version,
                    record.trace_id.as_str(),
                    record.span_id.as_str(),
                    record.parent_span_id.as_ref().map(|id| id.as_str()),
                    record.scope,
                    record.source,
                    record.operation_id,
                    record.action,
                    record
                        .policy_decision
                        .as_ref()
                        .map(|value| value.to_string()),
                    record.duration_ms,
                    serde_json::to_string(&record.input_digest).map_err(|error| {
                        KernelError::AuditSinkFailed {
                            message: error.to_string(),
                        }
                    })?,
                    serde_json::to_string(&record.output_digest).map_err(|error| {
                        KernelError::AuditSinkFailed {
                            message: error.to_string(),
                        }
                    })?,
                    record.status.to_string(),
                    record.created_at.to_rfc3339(),
                ],
            )
            .map_err(|error| KernelError::AuditSinkFailed {
                message: error.to_string(),
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::infra::Database;
    use crate::kernel::{AuditStatus, KernelContext, KernelScope};

    #[test]
    fn writes_audit_record() {
        let connection = Arc::new(Mutex::new(Database::open_memory().unwrap()));
        let sink = StorageAuditSink::new(connection.clone());
        let context = KernelContext::new("test", KernelScope::global());
        let record = AuditRecord::new(&context, "operation", "run", AuditStatus::Success);

        sink.record(&record).unwrap();

        let count: i64 = connection
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM audit_log", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
