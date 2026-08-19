// 通用沙箱与动态 ACL：将文件操作、Shell 执行、网络等封装为能力原语，
// 依据扩展声明与运行时授予的 Token 动态校验授权，不再编译期绑定静态权限。
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// 基础能力原语（可扩展）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    FsRead,
    FsWrite,
    ShellExec,
    Network,
    EventEmit,
}

impl Capability {
    /// 从清单 permissions 字段解析（兼容 snake_case / kebab-case）
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "fs.read" | "fs_read" => Some(Self::FsRead),
            "fs.write" | "fs_write" => Some(Self::FsWrite),
            "shell.exec" | "shell_exec" => Some(Self::ShellExec),
            "network" => Some(Self::Network),
            "event.emit" | "event_emit" => Some(Self::EventEmit),
            _ => None,
        }
    }
}

/// 扩展启动时提交的权限令牌：声明其插件身份与期望能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionToken {
    pub plugin_id: String,
    pub capabilities: Vec<String>,
}

/// 审计条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub ts: u128,
    pub plugin_id: String,
    pub capability: String,
    pub allowed: bool,
    pub detail: String,
}

/// 动态沙箱：维护每个插件的授权集合，并提供授权/校验/审计
#[derive(Clone)]
pub struct Sandbox {
    grants: Arc<RwLock<HashMap<String, HashSet<Capability>>>>,
    audit: Arc<RwLock<Vec<AuditEntry>>>,
}

impl Default for Sandbox {
    fn default() -> Self {
        Self::new()
    }
}

impl Sandbox {
    pub fn new() -> Self {
        Self {
            grants: Arc::new(RwLock::new(HashMap::new())),
            audit: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 授予插件一组能力（启动时根据 extension.json permissions 声明）
    pub fn grant(&self, plugin_id: &str, caps: &[String]) {
        let mut map = self.grants.write().unwrap();
        let set = map.entry(plugin_id.to_string()).or_default();
        for c in caps {
            if let Some(cap) = Capability::from_str(c) {
                set.insert(cap);
            }
        }
    }

    pub fn revoke(&self, plugin_id: &str) {
        self.grants.write().unwrap().remove(plugin_id);
    }

    /// 校验 Token 是否拥有某能力；记录审计日志
    pub fn authorize(
        &self,
        token: &PermissionToken,
        capability: Capability,
        detail: &str,
    ) -> Result<(), String> {
        let allowed = self
            .grants
            .read()
            .unwrap()
            .get(&token.plugin_id)
            .map(|set| set.contains(&capability))
            .unwrap_or(false);
        let cap_str = format!("{capability:?}");
        self.audit.write().unwrap().push(AuditEntry {
            ts: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
            plugin_id: token.plugin_id.clone(),
            capability: cap_str.clone(),
            allowed,
            detail: detail.to_string(),
        });
        if allowed {
            Ok(())
        } else {
            Err(format!(
                "[Navis Sandbox] plugin '{}' lacks capability '{}' ({detail})",
                token.plugin_id, cap_str
            ))
        }
    }

    /// 依据清单 permissions 字段为插件授权
    pub fn grant_from_manifest(&self, plugin_id: &str, permissions: &serde_json::Value) {
        if let Some(perms) = permissions.as_object() {
            let keys: Vec<String> = perms.keys().cloned().collect();
            self.grant(plugin_id, &keys);
        }
    }

    pub fn audit_log(&self) -> Vec<AuditEntry> {
        self.audit.read().unwrap().clone()
    }
}