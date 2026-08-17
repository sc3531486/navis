//! Auth 存储能力。
//!
//! 该能力由 security 域独占，封装 Auth 所需的数据库访问与加密上下文，
//! 避免 app/业务层接触 Connection、锁或通用数据库闭包。

use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::credential;
use super::key_store;
use super::provider_keys;
use super::{ApiKey, GitCredential, ValidationStatus};
use crate::app::infra::{Database, Encryption};

#[derive(Clone)]
pub struct AuthStore {
    connection: Arc<Mutex<Connection>>,
    encryption: Option<Encryption>,
}

impl AuthStore {
    pub fn open(db_path: &Path, encryption: Option<Encryption>) -> Result<Self> {
        let connection = Arc::new(Mutex::new(Database::open(db_path)?));
        Ok(Self {
            connection,
            encryption,
        })
    }

    #[cfg(test)]
    pub(crate) fn from_connection(
        connection: Arc<Mutex<Connection>>,
        encryption: Option<Encryption>,
    ) -> Self {
        Self {
            connection,
            encryption,
        }
    }

    pub fn insert_key(
        &self,
        provider: &str,
        name: &str,
        key: &str,
        base_url: Option<&str>,
    ) -> Result<ApiKey> {
        key_store::insert_key(
            &self.connection,
            &self.encryption,
            provider,
            name,
            key,
            base_url,
        )
    }

    pub fn get_key(&self, id: &str) -> Result<Option<ApiKey>> {
        key_store::get_key_by_id(&self.connection, id)
    }

    pub fn get_decrypted_key(&self, id: &str) -> Result<Option<String>> {
        key_store::get_decrypted_key(&self.connection, &self.encryption, id)
    }

    pub fn get_active_decrypted_key(&self, provider: &str) -> Result<Option<String>> {
        provider_keys::get_active_decrypted_key(&self.connection, &self.encryption, provider)
    }

    pub fn get_keys_by_provider(&self, provider: &str) -> Result<Vec<ApiKey>> {
        key_store::get_keys_by_provider(&self.connection, provider)
    }

    pub fn list_keys(&self) -> Result<Vec<ApiKey>> {
        key_store::list_keys(&self.connection)
    }

    pub fn delete_key(&self, id: &str) -> Result<()> {
        key_store::delete_key(&self.connection, id)
    }

    pub fn set_active_key(&self, provider: &str, key_id: &str) -> Result<()> {
        provider_keys::set_active_key(&self.connection, provider, key_id)
    }

    pub fn update_validation_status(&self, id: &str, status: &ValidationStatus) -> Result<()> {
        key_store::update_validation_status(&self.connection, id, status)
    }

    pub fn rotate_key(&self, id: &str, new_key: &str) -> Result<()> {
        key_store::rotate_key(&self.connection, &self.encryption, id, new_key)
    }

    pub fn insert_credential(&self, pattern: &str, credential: &GitCredential) -> Result<()> {
        credential::insert_credential(&self.connection, &self.encryption, pattern, credential)
    }

    pub fn find_matching_credential(&self, repo_url: &str) -> Result<Option<GitCredential>> {
        credential::find_matching_credential(&self.connection, &self.encryption, repo_url)
    }

    pub fn list_credentials(&self) -> Result<Vec<GitCredential>> {
        credential::list_credentials(&self.connection)
    }

    pub fn delete_credential(&self, id: &str) -> Result<()> {
        credential::delete_credential(&self.connection, id)
    }
}
