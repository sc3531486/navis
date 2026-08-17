// ── 归属扩展：navis-agent-core ──
// 迁移目标：extensions/navis-agent-core/ExtensionBackend/src/

use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tokio::sync::oneshot;

use super::super::permissions::{normalize_permission_key, normalize_permission_pattern};

pub struct ToolApprovalStore {
    pending: Mutex<HashMap<String, oneshot::Sender<ToolApprovalDecision>>>,
    pending_sessions: Mutex<HashMap<String, String>>,
    session_allow: Mutex<HashSet<String>>,
    project_allow: Mutex<HashSet<String>>,
    project_deny: Mutex<HashSet<String>>,
    project_rules_path: Option<PathBuf>,
}

impl ToolApprovalStore {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            pending_sessions: Mutex::new(HashMap::new()),
            session_allow: Mutex::new(HashSet::new()),
            project_allow: Mutex::new(HashSet::new()),
            project_deny: Mutex::new(HashSet::new()),
            project_rules_path: None,
        }
    }

    pub fn with_project_rule_store(path: PathBuf) -> Self {
        let registry = Self {
            pending: Mutex::new(HashMap::new()),
            pending_sessions: Mutex::new(HashMap::new()),
            session_allow: Mutex::new(HashSet::new()),
            project_allow: Mutex::new(HashSet::new()),
            project_deny: Mutex::new(HashSet::new()),
            project_rules_path: Some(path),
        };
        if let Err(error) = registry.load_project_rules() {
            tracing::warn!(error = %error, "Failed to load project approval rules");
        }
        registry
    }

    pub(crate) fn register(
        &self,
        session_id: &str,
        request_id: &str,
    ) -> Result<oneshot::Receiver<ToolApprovalDecision>, String> {
        let (sender, receiver) = oneshot::channel();
        {
            let mut pending = self
                .pending
                .lock()
                .map_err(|_| "工具审批状态不可用".to_string())?;
            pending.insert(request_id.to_string(), sender);
        }
        {
            let mut pending_sessions = self
                .pending_sessions
                .lock()
                .map_err(|_| "工具审批状态不可用".to_string())?;
            pending_sessions.insert(request_id.to_string(), session_id.to_string());
        }
        Ok(receiver)
    }

    fn cache_key(scope: &str, permission: &str, pattern: &str) -> String {
        format!(
            "{}\u{001f}{}\u{001f}{}",
            scope.trim(),
            normalize_permission_key(permission),
            normalize_permission_pattern(pattern)
        )
    }

    fn session_permission_key(session_id: &str, permission: &str, pattern: &str) -> String {
        Self::cache_key(session_id, permission, pattern)
    }

    fn normalize_worktree_root(worktree_root: &str) -> String {
        let normalized = worktree_root.replace('\\', "/");
        if normalized.len() > 1 && normalized.ends_with('/') {
            normalized[..normalized.len() - 1].to_string()
        } else {
            normalized
        }
    }

    fn worktree_permission_key(worktree_root: &str, permission: &str, pattern: &str) -> String {
        Self::cache_key(
            &Self::normalize_worktree_root(worktree_root),
            permission,
            pattern,
        )
    }

    pub(crate) fn is_session_allowed(
        &self,
        session_id: &str,
        permission: &str,
        pattern: &str,
    ) -> Result<bool, String> {
        Ok(self
            .session_allow
            .lock()
            .map_err(|_| "工具审批状态不可用".to_string())?
            .contains(&Self::session_permission_key(
                session_id, permission, pattern,
            )))
    }

    pub(crate) fn remember_session_allow(
        &self,
        session_id: &str,
        permission: &str,
        pattern: &str,
    ) -> Result<(), String> {
        self.session_allow
            .lock()
            .map_err(|_| "工具审批状态不可用".to_string())?
            .insert(Self::session_permission_key(
                session_id, permission, pattern,
            ));
        Ok(())
    }

    pub(crate) fn is_project_allowed(
        &self,
        worktree_root: Option<&str>,
        permission: &str,
        pattern: &str,
    ) -> Result<bool, String> {
        let Some(worktree_root) = worktree_root else {
            return Ok(false);
        };
        Ok(self
            .project_allow
            .lock()
            .map_err(|_| "项目审批状态不可用".to_string())?
            .contains(&Self::worktree_permission_key(
                worktree_root,
                permission,
                pattern,
            )))
    }

    pub(crate) fn is_project_denied(
        &self,
        worktree_root: Option<&str>,
        permission: &str,
        pattern: &str,
    ) -> Result<bool, String> {
        let Some(worktree_root) = worktree_root else {
            return Ok(false);
        };
        Ok(self
            .project_deny
            .lock()
            .map_err(|_| "项目审批状态不可用".to_string())?
            .contains(&Self::worktree_permission_key(
                worktree_root,
                permission,
                pattern,
            )))
    }

    pub(crate) fn remember_project_allow(
        &self,
        worktree_root: Option<&str>,
        permission: &str,
        pattern: &str,
    ) -> Result<(), String> {
        let worktree_root = worktree_root
            .ok_or_else(|| "当前会话未绑定 worktree，无法保存项目级审批允许规则".to_string())?;
        let key = Self::worktree_permission_key(worktree_root, permission, pattern);
        {
            let mut allow = self
                .project_allow
                .lock()
                .map_err(|_| "项目审批状态不可用".to_string())?;
            allow.insert(key.clone());
        }
        {
            let mut deny = self
                .project_deny
                .lock()
                .map_err(|_| "项目审批状态不可用".to_string())?;
            deny.remove(&key);
        }
        self.save_project_rules()
    }

    pub(crate) fn remember_project_deny(
        &self,
        worktree_root: Option<&str>,
        permission: &str,
        pattern: &str,
    ) -> Result<(), String> {
        let worktree_root = worktree_root
            .ok_or_else(|| "当前会话未绑定 worktree，无法保存项目级审批拒绝规则".to_string())?;
        let key = Self::worktree_permission_key(worktree_root, permission, pattern);
        {
            let mut deny = self
                .project_deny
                .lock()
                .map_err(|_| "项目审批状态不可用".to_string())?;
            deny.insert(key.clone());
        }
        {
            let mut allow = self
                .project_allow
                .lock()
                .map_err(|_| "项目审批状态不可用".to_string())?;
            allow.remove(&key);
        }
        self.save_project_rules()
    }

    fn load_project_rules(&self) -> Result<(), String> {
        let Some(path) = &self.project_rules_path else {
            return Ok(());
        };
        if !path.exists() {
            return Ok(());
        }
        let content =
            fs::read_to_string(path).map_err(|error| format!("项目审批规则读取失败: {}", error))?;
        let rules: ProjectApprovalRules = serde_json::from_str(&content)
            .map_err(|error| format!("项目审批规则解析失败: {}", error))?;
        {
            let mut allow = self
                .project_allow
                .lock()
                .map_err(|_| "项目审批状态不可用".to_string())?;
            allow.clear();
            allow.extend(rules.allow);
        }
        {
            let mut deny = self
                .project_deny
                .lock()
                .map_err(|_| "项目审批状态不可用".to_string())?;
            deny.clear();
            deny.extend(rules.deny);
        }
        Ok(())
    }

    fn save_project_rules(&self) -> Result<(), String> {
        let Some(path) = &self.project_rules_path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("项目审批规则目录创建失败: {}", error))?;
        }
        let allow = self
            .project_allow
            .lock()
            .map_err(|_| "项目审批状态不可用".to_string())?
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let deny = self
            .project_deny
            .lock()
            .map_err(|_| "项目审批状态不可用".to_string())?
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let content = serde_json::to_string_pretty(&ProjectApprovalRules { allow, deny })
            .map_err(|error| format!("项目审批规则序列化失败: {}", error))?;
        fs::write(path, content).map_err(|error| format!("项目审批规则保存失败: {}", error))
    }

    pub(crate) fn respond(
        &self,
        request_id: &str,
        decision: ToolApprovalDecision,
    ) -> Result<bool, String> {
        let sender = self
            .pending
            .lock()
            .map_err(|_| "工具审批状态不可用".to_string())?
            .remove(request_id);
        self.pending_sessions
            .lock()
            .map_err(|_| "工具审批状态不可用".to_string())?
            .remove(request_id);
        if let Some(sender) = sender {
            let _ = sender.send(decision);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub(crate) fn remove_pending(&self, request_id: &str) -> Result<bool, String> {
        let removed = self
            .pending
            .lock()
            .map_err(|_| "工具审批状态不可用".to_string())?
            .remove(request_id)
            .is_some();
        self.pending_sessions
            .lock()
            .map_err(|_| "工具审批状态不可用".to_string())?
            .remove(request_id);
        Ok(removed)
    }

    pub(crate) fn abort_session_pending(&self, session_id: &str) -> Result<usize, String> {
        let request_ids = {
            let pending_sessions = self
                .pending_sessions
                .lock()
                .map_err(|_| "工具审批状态不可用".to_string())?;
            pending_sessions
                .iter()
                .filter_map(|(request_id, pending_session_id)| {
                    (pending_session_id == session_id).then(|| request_id.clone())
                })
                .collect::<Vec<_>>()
        };

        let mut aborted = 0;
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| "工具审批状态不可用".to_string())?;
        let mut pending_sessions = self
            .pending_sessions
            .lock()
            .map_err(|_| "工具审批状态不可用".to_string())?;
        for request_id in request_ids {
            if pending.remove(&request_id).is_some() {
                aborted += 1;
            }
            pending_sessions.remove(&request_id);
        }
        Ok(aborted)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ProjectApprovalRules {
    #[serde(default)]
    allow: BTreeSet<String>,
    #[serde(default)]
    deny: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolApprovalDecision {
    AllowOnce,
    AllowSession,
    AllowProject,
    DenyAlways,
}

impl ToolApprovalDecision {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::AllowOnce => "allow_once",
            Self::AllowSession => "allow_session",
            Self::AllowProject => "allow_project",
            Self::DenyAlways => "deny_always",
        }
    }

    pub(crate) fn is_allowed(self) -> bool {
        matches!(
            self,
            Self::AllowOnce | Self::AllowSession | Self::AllowProject
        )
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "allow_once" => Some(Self::AllowOnce),
            "allow_session" => Some(Self::AllowSession),
            "allow_project" => Some(Self::AllowProject),
            "deny" | "deny_always" => Some(Self::DenyAlways),
            _ => None,
        }
    }
}

impl Default for ToolApprovalStore {
    fn default() -> Self {
        Self::new()
    }
}
