//! 缓存管理模块
//!
//! 基于设计文档 §四 实现，提供内存 LRU 缓存，支持 TTL 过期和自动清理。
//!
//! # 设计要点
//! - 内存层使用 HashMap + 访问时间戳实现近似 LRU
//! - 支持 TTL 过期自动淘汰
//! - 支持按缓存类型分组管理
//! - 数据库层的缓存元数据在 Storage 主模块中管理

use std::collections::HashMap;

use super::cache_types::CacheEntry;

/// 内存缓存管理器
///
/// 使用 HashMap 实现的内存缓存，支持 LRU 淘汰和 TTL 过期。
/// 按缓存类型（CacheType）分组管理缓存条目。
pub struct MemoryCache {
    /// 缓存数据存储，按 (cache_type, key) 索引
    entries: HashMap<(String, String), CacheEntry>,
    /// 缓存容量上限（条目数）
    capacity: usize,
}

impl MemoryCache {
    /// 创建新的内存缓存管理器
    ///
    /// # Arguments
    /// * `capacity` - 最大缓存条目数
    pub fn new(capacity: usize) -> Self {
        tracing::debug!(capacity = capacity, "Creating memory cache");
        Self {
            entries: HashMap::new(),
            capacity,
        }
    }

    /// 获取缓存数据
    ///
    /// # Arguments
    /// * `cache_type` - 缓存类型标识
    /// * `key` - 缓存键
    ///
    /// # Returns
    /// 缓存的数据（如果存在且未过期）
    pub fn get(&mut self, cache_type: &str, key: &str) -> Option<Vec<u8>> {
        let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let cache_key = (cache_type.to_string(), key.to_string());

        if let Some(entry) = self.entries.get_mut(&cache_key) {
            // 检查是否过期
            if entry.is_expired(now) {
                tracing::debug!(
                    cache_type = cache_type,
                    key = key,
                    "Cache entry expired, removing"
                );
                self.entries.remove(&cache_key);
                return None;
            }

            // 更新访问信息
            entry.last_accessed = now;
            entry.access_count += 1;
            Some(entry.data.clone())
        } else {
            None
        }
    }

    /// 设置缓存数据
    ///
    /// # Arguments
    /// * `cache_type` - 缓存类型标识
    /// * `key` - 缓存键
    /// * `data` - 缓存数据
    /// * `ttl_secs` - TTL（秒），None 表示永不过期
    pub fn set(&mut self, cache_type: &str, key: &str, data: Vec<u8>, ttl_secs: Option<u64>) {
        let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        // 如果缓存已满，执行 LRU 淘汰
        if self.entries.len() >= self.capacity {
            self.evict_lru();
        }

        let expires_at = ttl_secs.map(|ttl| now + (ttl as i64) * 1_000_000_000);

        let entry = CacheEntry {
            data,
            expires_at,
            last_accessed: now,
            access_count: 0,
        };

        let cache_key = (cache_type.to_string(), key.to_string());
        self.entries.insert(cache_key, entry);

        tracing::debug!(
            cache_type = cache_type,
            key = key,
            ttl = ?ttl_secs,
            "Cache entry set"
        );
    }

    /// 删除缓存条目
    ///
    /// # Arguments
    /// * `cache_type` - 缓存类型标识
    /// * `key` - 缓存键
    ///
    /// # Returns
    /// 是否成功删除
    pub fn delete(&mut self, cache_type: &str, key: &str) -> bool {
        let cache_key = (cache_type.to_string(), key.to_string());
        let removed = self.entries.remove(&cache_key).is_some();

        if removed {
            tracing::debug!(cache_type = cache_type, key = key, "Cache entry deleted");
        }

        removed
    }

    /// 清除指定缓存类型的所有条目
    ///
    /// # Arguments
    /// * `cache_type` - 缓存类型标识
    pub fn clear_type(&mut self, cache_type: &str) {
        let keys_to_remove: Vec<_> = self
            .entries
            .keys()
            .filter(|(ct, _)| ct == cache_type)
            .cloned()
            .collect();

        let count = keys_to_remove.len();
        for key in keys_to_remove {
            self.entries.remove(&key);
        }

        tracing::debug!(
            cache_type = cache_type,
            removed = count,
            "Cache type cleared"
        );
    }

    /// 清除所有缓存
    pub fn clear_all(&mut self) {
        let count = self.entries.len();
        self.entries.clear();
        tracing::debug!(removed = count, "All cache cleared");
    }

    /// 清理过期条目
    ///
    /// 遍历所有缓存条目，移除已过期的条目。
    pub fn cleanup_expired(&mut self) -> usize {
        let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        let expired_keys: Vec<_> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.is_expired(now))
            .map(|(key, _)| key.clone())
            .collect();

        let count = expired_keys.len();
        for key in expired_keys {
            self.entries.remove(&key);
        }

        if count > 0 {
            tracing::debug!(removed = count, "Expired cache entries cleaned up");
        }

        count
    }

    /// LRU 淘汰
    ///
    /// 移除最近最少访问的条目（按 last_accessed 时间排序）。
    fn evict_lru(&mut self) {
        if self.entries.is_empty() {
            return;
        }

        // 找到最久未访问的条目
        let lru_key = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(key, _)| key.clone());

        if let Some(key) = lru_key {
            self.entries.remove(&key);
            tracing::debug!(key = ?key, "LRU eviction: removed least recently used entry");
        }
    }

    /// 获取指定缓存类型的条目数
    pub fn count_by_type(&self, cache_type: &str) -> usize {
        self.entries
            .keys()
            .filter(|(ct, _)| ct == cache_type)
            .count()
    }

    /// 获取总条目数
    pub fn total_count(&self) -> usize {
        self.entries.len()
    }

    /// 获取指定缓存类型的总大小（字节）
    pub fn size_by_type(&self, cache_type: &str) -> u64 {
        self.entries
            .iter()
            .filter(|((ct, _), _)| ct == cache_type)
            .map(|(_, entry)| entry.data.len() as u64)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_set_get() {
        let mut cache = MemoryCache::new(100);

        cache.set("test", "key1", b"hello".to_vec(), None);
        let result = cache.get("test", "key1");
        assert_eq!(result, Some(b"hello".to_vec()));
    }

    #[test]
    fn test_cache_get_nonexistent() {
        let mut cache = MemoryCache::new(100);
        assert!(cache.get("test", "nonexistent").is_none());
    }

    #[test]
    fn test_cache_delete() {
        let mut cache = MemoryCache::new(100);

        cache.set("test", "key1", b"data".to_vec(), None);
        assert!(cache.delete("test", "key1"));
        assert!(cache.get("test", "key1").is_none());
    }

    #[test]
    fn test_cache_clear_type() {
        let mut cache = MemoryCache::new(100);

        cache.set("type1", "k1", b"a".to_vec(), None);
        cache.set("type1", "k2", b"b".to_vec(), None);
        cache.set("type2", "k3", b"c".to_vec(), None);

        cache.clear_type("type1");

        assert!(cache.get("type1", "k1").is_none());
        assert!(cache.get("type1", "k2").is_none());
        assert!(cache.get("type2", "k3").is_some());
    }

    #[test]
    fn test_cache_clear_all() {
        let mut cache = MemoryCache::new(100);

        cache.set("type1", "k1", b"a".to_vec(), None);
        cache.set("type2", "k2", b"b".to_vec(), None);

        cache.clear_all();
        assert_eq!(cache.total_count(), 0);
    }

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = MemoryCache::new(3);

        cache.set("t", "k1", b"a".to_vec(), None);
        std::thread::sleep(std::time::Duration::from_millis(10));
        cache.set("t", "k2", b"b".to_vec(), None);
        std::thread::sleep(std::time::Duration::from_millis(10));
        cache.set("t", "k3", b"c".to_vec(), None);

        // 访问 k1 更新其时间戳
        cache.get("t", "k1");

        // 添加第 4 个条目，触发 LRU 淘汰（k2 应该被淘汰）
        cache.set("t", "k4", b"d".to_vec(), None);

        assert_eq!(cache.total_count(), 3);
        assert!(cache.get("t", "k1").is_some()); // 已访问，不被淘汰
        assert!(cache.get("t", "k2").is_none()); // 最久未访问，被淘汰
    }

    #[test]
    fn test_cache_count_and_size() {
        let mut cache = MemoryCache::new(100);

        cache.set("type1", "k1", vec![0u8; 100], None);
        cache.set("type1", "k2", vec![0u8; 200], None);
        cache.set("type2", "k3", vec![0u8; 300], None);

        assert_eq!(cache.count_by_type("type1"), 2);
        assert_eq!(cache.count_by_type("type2"), 1);
        assert_eq!(cache.size_by_type("type1"), 300);
        assert_eq!(cache.size_by_type("type2"), 300);
    }
}
