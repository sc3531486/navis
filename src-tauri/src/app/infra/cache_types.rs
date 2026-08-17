//! 缓存分类定义模块
//!
//! 基于设计文档 §四 定义缓存类型、配置和相关数据结构。
//! 每种缓存类型有独立的 TTL、最大大小和清理策略。

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 缓存类型枚举
///
/// 对应设计文档 §四 的缓存分类表
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CacheType {
    /// Worktree 缓存（DB 存储，24h TTL，500MB 上限，LRU+TTL 清理）
    WorktreeCache,
    /// Diff 缓存（内存存储，1h TTL，50MB 上限，会话切换清理）
    DiffCache,
    /// 索引缓存（DB+文件存储，7d TTL，1GB 上限，源文件变更清理）
    IndexCache,
    /// 模型缓存（文件存储，30d TTL，2GB 上限，手动清理）
    ModelCache,
    /// 会话缓存（内存存储，会话生命周期 TTL，100MB 上限，会话关闭清理）
    SessionCache,
    /// 工具缓存（DB 存储，10min TTL，50MB 上限，LRU 清理）
    ToolCache,
    /// RAG 缓存（DB 存储，10min TTL，100MB 上限，LRU+知识源更新清理）
    RagCache,
}

impl CacheType {
    /// 获取缓存类型的字符串标识符（用于数据库存储）
    pub fn as_str(&self) -> &'static str {
        match self {
            CacheType::WorktreeCache => "worktree_cache",
            CacheType::DiffCache => "diff_cache",
            CacheType::IndexCache => "index_cache",
            CacheType::ModelCache => "model_cache",
            CacheType::SessionCache => "session_cache",
            CacheType::ToolCache => "tool_cache",
            CacheType::RagCache => "rag_cache",
        }
    }

    /// 从字符串解析缓存类型
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "worktree_cache" => Some(CacheType::WorktreeCache),
            "diff_cache" => Some(CacheType::DiffCache),
            "index_cache" => Some(CacheType::IndexCache),
            "model_cache" => Some(CacheType::ModelCache),
            "session_cache" => Some(CacheType::SessionCache),
            "tool_cache" => Some(CacheType::ToolCache),
            "rag_cache" => Some(CacheType::RagCache),
            _ => None,
        }
    }

    /// 获取默认 TTL
    pub fn default_ttl(&self) -> Option<Duration> {
        match self {
            CacheType::WorktreeCache => Some(Duration::from_secs(24 * 3600)), // 24 小时
            CacheType::DiffCache => Some(Duration::from_secs(3600)),          // 1 小时
            CacheType::IndexCache => Some(Duration::from_secs(7 * 24 * 3600)), // 7 天
            CacheType::ModelCache => Some(Duration::from_secs(30 * 24 * 3600)), // 30 天
            CacheType::SessionCache => None,                                  // 会话生命周期
            CacheType::ToolCache => Some(Duration::from_secs(10 * 60)),       // 10 分钟
            CacheType::RagCache => Some(Duration::from_secs(10 * 60)),        // 10 分钟
        }
    }

    /// 获取最大缓存大小（字节）
    pub fn max_size_bytes(&self) -> u64 {
        match self {
            CacheType::WorktreeCache => 500 * 1024 * 1024, // 500MB
            CacheType::DiffCache => 50 * 1024 * 1024,      // 50MB
            CacheType::IndexCache => 1024 * 1024 * 1024,   // 1GB
            CacheType::ModelCache => 2 * 1024 * 1024 * 1024, // 2GB
            CacheType::SessionCache => 100 * 1024 * 1024,  // 100MB
            CacheType::ToolCache => 50 * 1024 * 1024,      // 50MB
            CacheType::RagCache => 100 * 1024 * 1024,      // 100MB
        }
    }

    /// 获取所有缓存类型
    pub fn all() -> &'static [CacheType] {
        &[
            CacheType::WorktreeCache,
            CacheType::DiffCache,
            CacheType::IndexCache,
            CacheType::ModelCache,
            CacheType::SessionCache,
            CacheType::ToolCache,
            CacheType::RagCache,
        ]
    }
}

impl std::fmt::Display for CacheType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 缓存清理结果
///
/// 记录一次缓存清理操作的结果统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheCleanupResult {
    /// 清理的缓存条目数
    pub entries_removed: usize,
    /// 释放的空间（字节）
    pub freed_bytes: u64,
    /// 清理耗时（毫秒）
    pub duration_ms: u64,
}

/// 缓存统计信息
///
/// 单个缓存类型的统计数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// 缓存类型标识
    pub cache_type: String,
    /// 条目数量
    pub entry_count: usize,
    /// 总大小（字节）
    pub total_size_bytes: u64,
    /// 最近一次访问时间
    pub last_accessed: Option<String>,
}

/// 缓存条目（内存层）
///
/// 用于内存 LRU 缓存中的条目
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// 缓存数据
    pub data: Vec<u8>,
    /// 过期时间戳（纳秒）
    pub expires_at: Option<i64>,
    /// 最后访问时间戳（纳秒，使用纳秒级精度确保 LRU 淘汰的精确性）
    pub last_accessed: i64,
    /// 访问次数
    pub access_count: u64,
}

impl CacheEntry {
    /// 检查缓存条目是否已过期
    ///
    /// # Arguments
    /// * `now` - 当前时间戳（纳秒）
    pub fn is_expired(&self, now: i64) -> bool {
        if let Some(expires_at) = self.expires_at {
            now >= expires_at
        } else {
            false
        }
    }
}

/// 缓存类型配置
///
/// 描述单个缓存类型的完整配置信息
#[derive(Debug, Clone)]
pub struct CacheTypeConfig {
    /// 缓存类型
    pub cache_type: CacheType,
    /// TTL
    pub ttl: Option<Duration>,
    /// 最大大小（字节）
    pub max_size: u64,
    /// 是否存储在 DB 中
    pub persistent: bool,
    /// 清理策略描述
    pub cleanup_policy: &'static str,
}

impl CacheTypeConfig {
    /// 获取指定缓存类型的配置
    pub fn for_type(cache_type: &CacheType) -> Self {
        match cache_type {
            CacheType::WorktreeCache => Self {
                cache_type: CacheType::WorktreeCache,
                ttl: Some(Duration::from_secs(24 * 3600)),
                max_size: 500 * 1024 * 1024,
                persistent: true,
                cleanup_policy: "LRU + TTL",
            },
            CacheType::DiffCache => Self {
                cache_type: CacheType::DiffCache,
                ttl: Some(Duration::from_secs(3600)),
                max_size: 50 * 1024 * 1024,
                persistent: false,
                cleanup_policy: "会话切换清理",
            },
            CacheType::IndexCache => Self {
                cache_type: CacheType::IndexCache,
                ttl: Some(Duration::from_secs(7 * 24 * 3600)),
                max_size: 1024 * 1024 * 1024,
                persistent: true,
                cleanup_policy: "源文件变更清理",
            },
            CacheType::ModelCache => Self {
                cache_type: CacheType::ModelCache,
                ttl: Some(Duration::from_secs(30 * 24 * 3600)),
                max_size: 2 * 1024 * 1024 * 1024,
                persistent: false,
                cleanup_policy: "手动清理",
            },
            CacheType::SessionCache => Self {
                cache_type: CacheType::SessionCache,
                ttl: None,
                max_size: 100 * 1024 * 1024,
                persistent: false,
                cleanup_policy: "会话关闭清理",
            },
            CacheType::ToolCache => Self {
                cache_type: CacheType::ToolCache,
                ttl: Some(Duration::from_secs(10 * 60)),
                max_size: 50 * 1024 * 1024,
                persistent: true,
                cleanup_policy: "LRU",
            },
            CacheType::RagCache => Self {
                cache_type: CacheType::RagCache,
                ttl: Some(Duration::from_secs(10 * 60)),
                max_size: 100 * 1024 * 1024,
                persistent: true,
                cleanup_policy: "LRU + 知识源更新清理",
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_type_as_str_roundtrip() {
        for ct in CacheType::all() {
            let s = ct.as_str();
            let parsed = CacheType::from_str(s);
            assert_eq!(
                parsed.as_ref(),
                Some(ct),
                "CacheType roundtrip failed for {}",
                s
            );
        }
    }

    #[test]
    fn test_cache_type_from_str_invalid() {
        assert!(CacheType::from_str("invalid_cache").is_none());
        assert!(CacheType::from_str("").is_none());
    }

    #[test]
    fn test_cache_type_ttl() {
        assert_eq!(
            CacheType::WorktreeCache.default_ttl(),
            Some(Duration::from_secs(24 * 3600))
        );
        assert_eq!(CacheType::SessionCache.default_ttl(), None);
        assert_eq!(
            CacheType::ToolCache.default_ttl(),
            Some(Duration::from_secs(600))
        );
    }

    #[test]
    fn test_cache_type_max_size() {
        assert_eq!(CacheType::WorktreeCache.max_size_bytes(), 500 * 1024 * 1024);
        assert_eq!(CacheType::DiffCache.max_size_bytes(), 50 * 1024 * 1024);
        assert_eq!(CacheType::RagCache.max_size_bytes(), 100 * 1024 * 1024);
    }

    #[test]
    fn test_cache_entry_expiration() {
        let entry = CacheEntry {
            data: vec![],
            expires_at: Some(1000),
            last_accessed: 0,
            access_count: 0,
        };
        assert!(!entry.is_expired(999));
        assert!(entry.is_expired(1000));
        assert!(entry.is_expired(1001));
    }

    #[test]
    fn test_cache_entry_no_expiration() {
        let entry = CacheEntry {
            data: vec![],
            expires_at: None,
            last_accessed: 0,
            access_count: 0,
        };
        assert!(!entry.is_expired(i64::MAX));
    }

    #[test]
    fn test_cache_type_config() {
        let config = CacheTypeConfig::for_type(&CacheType::WorktreeCache);
        assert!(config.persistent);
        assert_eq!(config.cleanup_policy, "LRU + TTL");

        let config = CacheTypeConfig::for_type(&CacheType::DiffCache);
        assert!(!config.persistent);
    }
}
