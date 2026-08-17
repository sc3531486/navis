//! Auth schema checks.
//!
//! Auth tables are part of the database root schema. This module only verifies
//! that the root schema contains the expected auth tables; table creation stays
//! in the database root schema.

use anyhow::Result;
use rusqlite::Connection;

/// Verify that Auth database tables exist in the root schema.
pub fn verify_auth_tables(conn: &Connection) -> Result<bool> {
    let required_tables = ["api_keys", "git_credentials", "provider_active_keys"];

    for table in &required_tables {
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                rusqlite::params![table],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;

        if !exists {
            tracing::warn!(table = table, "Auth table missing from root schema");
            return Ok(false);
        }
    }

    tracing::debug!("All auth tables verified");
    Ok(true)
}

/// Get auth table row counts for diagnostics and tests.
pub fn get_table_counts(conn: &Connection) -> Result<(i64, i64, i64)> {
    let api_keys_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM api_keys", [], |row| row.get(0))
        .unwrap_or(0);

    let git_creds_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM git_credentials", [], |row| row.get(0))
        .unwrap_or(0);

    let active_keys_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM provider_active_keys", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    Ok((api_keys_count, git_creds_count, active_keys_count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::infra::db::Database;

    #[test]
    fn test_verify_auth_tables_exist() {
        let conn = Database::open_memory().unwrap();
        let result = verify_auth_tables(&conn).unwrap();
        assert!(
            result,
            "All auth tables should exist after Database::open_memory"
        );
    }

    #[test]
    fn test_get_table_counts_empty() {
        let conn = Database::open_memory().unwrap();
        let (api_keys, git_creds, active_keys) = get_table_counts(&conn).unwrap();
        assert_eq!(api_keys, 0);
        assert_eq!(git_creds, 0);
        assert_eq!(active_keys, 0);
    }

    #[test]
    fn test_auth_tables_are_created_by_root_schema() {
        let conn = Database::open_memory().unwrap();
        assert!(verify_auth_tables(&conn).unwrap());
    }
}
