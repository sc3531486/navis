//! worktree 信任管理
//!
//! 基于设计文档 §6 实现，管理worktree 的信任级别。
//!
//! # 信任级别
//! - Trusted: 完全信任，全部放行（Level 0-3 按规则）
//! - Untrusted: 不信任，只允许 Level 0（只读）
//! - AskEachTime: 每次询问
//! - SessionScoped: 仅本次会话信任（不持久化，会话结束自动清除）

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

// ============================================================================
// 信任级别
// ============================================================================

/// worktree 信任级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrustLevel {
    /// 完全信任
    Trusted,
    /// 不信任（受限访问）
    Untrusted,
    /// 每次询问
    AskEachTime,
    /// 仅本次会话信任（不持久化）
    SessionScoped,
}

impl TrustLevel {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "trusted" => Some(TrustLevel::Trusted),
            "untrusted" => Some(TrustLevel::Untrusted),
            "ask" | "askeachtime" | "ask_each_time" => Some(TrustLevel::AskEachTime),
            "session" | "sessionscoped" | "session_scoped" => Some(TrustLevel::SessionScoped),
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            TrustLevel::Trusted => "trusted",
            TrustLevel::Untrusted => "untrusted",
            TrustLevel::AskEachTime => "ask",
            TrustLevel::SessionScoped => "session",
        }
    }

    /// 是否允许写操作
    pub fn allows_write(&self) -> bool {
        matches!(self, TrustLevel::Trusted | TrustLevel::SessionScoped)
    }
}

impl std::fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// WorktreeTrust 管理器
// ============================================================================

/// worktree 信任管理器
///
/// 管理worktree 的信任级别记录，支持持久化和会话级别信任。
#[derive(Debug)]
pub struct WorktreeTrustManager {
    /// 持久化信任记录（路径 -> 信任级别）
    trust_map: HashMap<String, TrustLevel>,
    /// 会话级信任记录（不持久化，会话结束清除）
    session_trust: HashMap<String, TrustLevel>,
}

impl WorktreeTrustManager {
    /// 创建新的worktree 信任管理器
    pub fn new() -> Self {
        tracing::debug!("Creating new WorktreeTrustManager");
        Self {
            trust_map: HashMap::new(),
            session_trust: HashMap::new(),
        }
    }

    /// 获取worktree 的信任级别
    ///
    /// 查询顺序：
    /// 1. 会话级信任记录（SessionScoped）
    /// 2. 持久化信任记录
    /// 3. 返回 None（表示未记录，需弹出确认对话框）
    ///
    /// # Arguments
    /// * `worktree` - worktree 路径
    pub fn get_trust(&self, worktree: &Path) -> Option<TrustLevel> {
        let key = normalize_worktree_path(worktree);

        // 会话级信任优先
        if let Some(trust) = self.session_trust.get(&key) {
            tracing::debug!(worktree = %key, trust = %trust, "Found session-scoped trust");
            return Some(*trust);
        }

        // 持久化信任
        if let Some(trust) = self.trust_map.get(&key) {
            tracing::debug!(worktree = %key, trust = %trust, "Found persistent trust");
            return Some(*trust);
        }

        tracing::debug!(worktree = %key, "No trust record found");
        None
    }

    /// 设置worktree 的信任级别
    ///
    /// # Arguments
    /// * `worktree` - worktree 路径
    /// * `trust` - 信任级别
    pub fn set_trust(&mut self, worktree: &Path, trust: TrustLevel) {
        let key = normalize_worktree_path(worktree);

        tracing::info!(
            worktree = %key,
            trust = %trust,
            "Setting worktree trust"
        );

        match trust {
            TrustLevel::SessionScoped => {
                // 会话级信任不持久化
                self.session_trust.insert(key.clone(), trust);
            }
            _ => {
                // 从会话记录中移除（如果有）
                self.session_trust.remove(&key);
                // 写入持久化记录
                self.trust_map.insert(key, trust);
            }
        }
    }

    /// 清除worktree 的信任记录
    ///
    /// # Arguments
    /// * `worktree` - worktree 路径
    pub fn clear_trust(&mut self, worktree: &Path) {
        let key = normalize_worktree_path(worktree);
        tracing::info!(worktree = %key, "Clearing worktree trust");
        self.trust_map.remove(&key);
        self.session_trust.remove(&key);
    }

    /// 清除所有会话级信任记录
    ///
    /// 会话结束时调用。
    pub fn clear_session_trust(&mut self) {
        let count = self.session_trust.len();
        tracing::info!(count = count, "Clearing all session-scoped trust records");
        self.session_trust.clear();
    }

    /// 获取所有持久化信任记录
    pub fn list_trusted(&self) -> Vec<(String, TrustLevel)> {
        self.trust_map
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    /// 判断worktree是否为完全信任
    pub fn is_trusted(&self, worktree: &Path) -> bool {
        matches!(
            self.get_trust(worktree),
            Some(TrustLevel::Trusted) | Some(TrustLevel::SessionScoped)
        )
    }

    /// 判断worktree是否为不信任
    pub fn is_untrusted(&self, worktree: &Path) -> bool {
        self.get_trust(worktree) == Some(TrustLevel::Untrusted)
    }

    /// 从持久化数据恢复信任记录
    ///
    /// # Arguments
    /// * `records` - 信任记录列表
    pub fn load_records(&mut self, records: Vec<(String, TrustLevel)>) {
        let count = records.len();
        tracing::info!(count = count, "Loading worktree trust records");
        for (worktree, trust) in records {
            self.trust_map.insert(worktree, trust);
        }
    }
}

impl Default for WorktreeTrustManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 规范化worktree 路径（作为 HashMap 的 key）
fn normalize_worktree_path(path: &Path) -> String {
    let s = path.to_string_lossy().to_string();
    let normalized = s.replace('\\', "/");

    // 去除尾部的 /
    if normalized.len() > 1 && normalized.ends_with('/') {
        normalized[..normalized.len() - 1].to_string()
    } else {
        normalized
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_trust_level_parse() {
        assert_eq!(TrustLevel::from_str("trusted"), Some(TrustLevel::Trusted));
        assert_eq!(
            TrustLevel::from_str("untrusted"),
            Some(TrustLevel::Untrusted)
        );
        assert_eq!(TrustLevel::from_str("ask"), Some(TrustLevel::AskEachTime));
        assert_eq!(
            TrustLevel::from_str("AskEachTime"),
            Some(TrustLevel::AskEachTime)
        );
        assert_eq!(
            TrustLevel::from_str("ask_each_time"),
            Some(TrustLevel::AskEachTime)
        );
        assert_eq!(
            TrustLevel::from_str("session"),
            Some(TrustLevel::SessionScoped)
        );
        assert_eq!(
            TrustLevel::from_str("SessionScoped"),
            Some(TrustLevel::SessionScoped)
        );
        assert_eq!(
            TrustLevel::from_str("session_scoped"),
            Some(TrustLevel::SessionScoped)
        );
        assert!(TrustLevel::from_str("unknown").is_none());
    }

    #[test]
    fn test_trust_level_display() {
        assert_eq!(TrustLevel::Trusted.to_string(), "trusted");
        assert_eq!(TrustLevel::Untrusted.to_string(), "untrusted");
        assert_eq!(TrustLevel::AskEachTime.to_string(), "ask");
        assert_eq!(TrustLevel::SessionScoped.to_string(), "session");
    }

    #[test]
    fn test_trust_level_allows_write() {
        assert!(TrustLevel::Trusted.allows_write());
        assert!(!TrustLevel::Untrusted.allows_write());
        assert!(!TrustLevel::AskEachTime.allows_write());
        assert!(TrustLevel::SessionScoped.allows_write());
    }

    #[test]
    fn test_worktree_trust_new() {
        let wt = WorktreeTrustManager::new();
        assert!(wt.list_trusted().is_empty());
    }

    #[test]
    fn test_set_and_get_trust() {
        let mut wt = WorktreeTrustManager::new();
        let ws = PathBuf::from("/home/user/project");

        wt.set_trust(&ws, TrustLevel::Trusted);
        assert_eq!(wt.get_trust(&ws), Some(TrustLevel::Trusted));
    }

    #[test]
    fn test_get_trust_unknown_worktree() {
        let wt = WorktreeTrustManager::new();
        let ws = PathBuf::from("/home/user/unknown");
        assert!(wt.get_trust(&ws).is_none());
    }

    #[test]
    fn test_set_untrusted() {
        let mut wt = WorktreeTrustManager::new();
        let ws = PathBuf::from("/home/user/project");

        wt.set_trust(&ws, TrustLevel::Untrusted);
        assert_eq!(wt.get_trust(&ws), Some(TrustLevel::Untrusted));
        assert!(wt.is_untrusted(&ws));
        assert!(!wt.is_trusted(&ws));
    }

    #[test]
    fn test_session_scoped_trust() {
        let mut wt = WorktreeTrustManager::new();
        let ws = PathBuf::from("/home/user/project");

        // 设置会话级信任
        wt.set_trust(&ws, TrustLevel::SessionScoped);
        assert_eq!(wt.get_trust(&ws), Some(TrustLevel::SessionScoped));
        assert!(wt.is_trusted(&ws));

        // 会话级信任不会出现在持久化列表中
        let trusted_list = wt.list_trusted();
        assert!(trusted_list.is_empty());
    }

    #[test]
    fn test_clear_session_trust() {
        let mut wt = WorktreeTrustManager::new();
        let ws1 = PathBuf::from("/home/user/project1");
        let ws2 = PathBuf::from("/home/user/project2");

        wt.set_trust(&ws1, TrustLevel::SessionScoped);
        wt.set_trust(&ws2, TrustLevel::Trusted);

        // 清除会话信任
        wt.clear_session_trust();

        // session_scoped 信任被清除
        assert!(wt.get_trust(&ws1).is_none());
        // 持久化信任保留
        assert_eq!(wt.get_trust(&ws2), Some(TrustLevel::Trusted));
    }

    #[test]
    fn test_clear_trust() {
        let mut wt = WorktreeTrustManager::new();
        let ws = PathBuf::from("/home/user/project");

        wt.set_trust(&ws, TrustLevel::Trusted);
        assert_eq!(wt.get_trust(&ws), Some(TrustLevel::Trusted));

        wt.clear_trust(&ws);
        assert!(wt.get_trust(&ws).is_none());
    }

    #[test]
    fn test_list_trusted() {
        let mut wt = WorktreeTrustManager::new();

        wt.set_trust(&PathBuf::from("/project/a"), TrustLevel::Trusted);
        wt.set_trust(&PathBuf::from("/project/b"), TrustLevel::Untrusted);
        wt.set_trust(&PathBuf::from("/project/c"), TrustLevel::AskEachTime);

        let list = wt.list_trusted();
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn test_is_trusted_and_is_untrusted() {
        let mut wt = WorktreeTrustManager::new();
        let ws = PathBuf::from("/home/user/project");

        // 未设置
        assert!(!wt.is_trusted(&ws));
        assert!(!wt.is_untrusted(&ws));

        // 设置为 Trusted
        wt.set_trust(&ws, TrustLevel::Trusted);
        assert!(wt.is_trusted(&ws));
        assert!(!wt.is_untrusted(&ws));

        // 更改为 Untrusted
        wt.set_trust(&ws, TrustLevel::Untrusted);
        assert!(!wt.is_trusted(&ws));
        assert!(wt.is_untrusted(&ws));
    }

    #[test]
    fn test_load_records() {
        let mut wt = WorktreeTrustManager::new();

        let records = vec![
            ("/project/a".to_string(), TrustLevel::Trusted),
            ("/project/b".to_string(), TrustLevel::Untrusted),
        ];

        wt.load_records(records);

        assert_eq!(
            wt.get_trust(&PathBuf::from("/project/a")),
            Some(TrustLevel::Trusted)
        );
        assert_eq!(
            wt.get_trust(&PathBuf::from("/project/b")),
            Some(TrustLevel::Untrusted)
        );
    }

    #[test]
    fn test_normalize_worktree_path() {
        assert_eq!(
            normalize_worktree_path(&PathBuf::from("/home/user/project")),
            "/home/user/project"
        );
        assert_eq!(
            normalize_worktree_path(&PathBuf::from("C:\\Users\\admin\\project")),
            "C:/Users/admin/project"
        );
    }

    #[test]
    fn test_trust_level_serialization() {
        let trust = TrustLevel::Trusted;
        let json = serde_json::to_string(&trust).unwrap();
        let deserialized: TrustLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(trust, deserialized);

        let trust = TrustLevel::SessionScoped;
        let json = serde_json::to_string(&trust).unwrap();
        let deserialized: TrustLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(trust, deserialized);
    }

    #[test]
    fn test_multiple_worktrees() {
        let mut wt = WorktreeTrustManager::new();

        let ws1 = PathBuf::from("/project/a");
        let ws2 = PathBuf::from("/project/b");

        wt.set_trust(&ws1, TrustLevel::Trusted);
        wt.set_trust(&ws2, TrustLevel::Untrusted);

        assert!(wt.is_trusted(&ws1));
        assert!(wt.is_untrusted(&ws2));

        // 清除一个不影响另一个
        wt.clear_trust(&ws1);
        assert!(wt.get_trust(&ws1).is_none());
        assert!(wt.is_untrusted(&ws2));
    }
}
