//! 资源限制
//!
//! 基于设计文档 §3 实现，提供 CPU/内存/时间等资源限制和用量监控。
//!
//! # 设计思路
//! - 资源类型：CPU、内存、磁盘、执行时间
//! - 每种资源类型可配置上限值
//! - 跟踪当前用量
//! - 超限时发出警告/拒绝

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ============================================================================
// 资源类型
// ============================================================================

/// 资源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    /// CPU 使用率（百分比，0-100）
    Cpu,
    /// 内存使用量（字节）
    Memory,
    /// 磁盘使用量（字节）
    Disk,
    /// 执行时间（秒）
    ExecutionTime,
    /// 网络连接数
    NetworkConnections,
}

impl ResourceType {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "cpu" => Some(ResourceType::Cpu),
            "memory" | "mem" => Some(ResourceType::Memory),
            "disk" => Some(ResourceType::Disk),
            "execution_time" | "executiontime" | "time" => Some(ResourceType::ExecutionTime),
            "network_connections" | "networkconnections" | "connections" => {
                Some(ResourceType::NetworkConnections)
            }
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceType::Cpu => "cpu",
            ResourceType::Memory => "memory",
            ResourceType::Disk => "disk",
            ResourceType::ExecutionTime => "execution_time",
            ResourceType::NetworkConnections => "network_connections",
        }
    }

    /// 获取默认限制值
    pub fn default_limit_value(&self) -> f64 {
        match self {
            ResourceType::Cpu => 80.0,                       // 80%
            ResourceType::Memory => 512.0 * 1024.0 * 1024.0, // 512 MB
            ResourceType::Disk => 1024.0 * 1024.0 * 1024.0,  // 1 GB
            ResourceType::ExecutionTime => 300.0,            // 5 分钟
            ResourceType::NetworkConnections => 100.0,       // 100 个连接
        }
    }

    /// 获取资源单位名称
    pub fn unit_name(&self) -> &'static str {
        match self {
            ResourceType::Cpu => "%",
            ResourceType::Memory => "bytes",
            ResourceType::Disk => "bytes",
            ResourceType::ExecutionTime => "seconds",
            ResourceType::NetworkConnections => "connections",
        }
    }
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// 资源限制
// ============================================================================

/// 资源限制配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimit {
    /// 资源类型
    pub resource_type: ResourceType,
    /// 上限值
    pub max_value: f64,
    /// 警告阈值（百分比，0.0-1.0，相对于 max_value）
    pub warn_threshold: f64,
    /// 是否启用
    pub enabled: bool,
}

impl ResourceLimit {
    /// 创建新的资源限制
    pub fn new(resource_type: ResourceType, max_value: f64) -> Self {
        Self {
            resource_type,
            max_value,
            warn_threshold: 0.8, // 默认 80% 时警告
            enabled: true,
        }
    }

    /// 设置警告阈值
    pub fn with_warn_threshold(mut self, threshold: f64) -> Self {
        self.warn_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// 设置为禁用
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// 检查用量是否超过限制
    pub fn check_usage(&self, usage: f64) -> ResourceCheckResult {
        if !self.enabled {
            return ResourceCheckResult::Ok;
        }

        if usage >= self.max_value {
            ResourceCheckResult::Exceeded
        } else if usage >= self.max_value * self.warn_threshold {
            ResourceCheckResult::Warning
        } else {
            ResourceCheckResult::Ok
        }
    }
}

/// 资源检查结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceCheckResult {
    /// 正常
    Ok,
    /// 警告（接近上限）
    Warning,
    /// 超限
    Exceeded,
}

// ============================================================================
// 资源用量
// ============================================================================

/// 资源用量信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// 资源类型
    pub resource_type: ResourceType,
    /// 当前用量
    pub current_value: f64,
    /// 上限值
    pub max_value: f64,
    /// 使用率（百分比）
    pub usage_percent: f64,
}

impl ResourceUsage {
    /// 创建新的资源用量信息
    pub fn new(resource_type: ResourceType, current: f64, max: f64) -> Self {
        let percent = if max > 0.0 {
            (current / max * 100.0).min(100.0)
        } else {
            0.0
        };
        Self {
            resource_type,
            current_value: current,
            max_value: max,
            usage_percent: percent,
        }
    }
}

// ============================================================================
// ResourceLimitManager
// ============================================================================

/// 资源限制管理器
///
/// 管理所有资源类型的限制配置和当前用量。
#[derive(Debug)]
pub struct ResourceLimitManager {
    /// 资源限制配置
    limits: HashMap<ResourceType, ResourceLimit>,
    /// 当前用量
    usage: HashMap<ResourceType, f64>,
}

impl ResourceLimitManager {
    /// 创建新的资源限制管理器
    pub fn new() -> Self {
        tracing::debug!("Creating new ResourceLimitManager");
        let mut manager = Self {
            limits: HashMap::new(),
            usage: HashMap::new(),
        };

        // 注册默认资源限制
        manager.init_defaults();
        manager
    }

    /// 初始化默认资源限制
    fn init_defaults(&mut self) {
        let resource_types = [
            ResourceType::Cpu,
            ResourceType::Memory,
            ResourceType::Disk,
            ResourceType::ExecutionTime,
            ResourceType::NetworkConnections,
        ];

        for rt in &resource_types {
            self.limits
                .insert(*rt, ResourceLimit::new(*rt, rt.default_limit_value()));
            self.usage.insert(*rt, 0.0);
        }
    }

    /// 设置资源限制
    ///
    /// # Arguments
    /// * `resource` - 资源类型
    /// * `limit` - 资源限制配置
    pub fn set_limit(&mut self, resource: ResourceType, limit: ResourceLimit) {
        tracing::info!(
            resource = %resource,
            max_value = limit.max_value,
            warn_threshold = limit.warn_threshold,
            enabled = limit.enabled,
            "Setting resource limit"
        );
        self.limits.insert(resource, limit);
    }

    /// 获取资源限制
    pub fn get_limit(&self, resource: &ResourceType) -> Option<&ResourceLimit> {
        self.limits.get(resource)
    }

    /// 获取资源用量
    pub fn get_usage(&self, resource: &ResourceType) -> ResourceUsage {
        let current = self.usage.get(resource).copied().unwrap_or(0.0);
        let max = self
            .limits
            .get(resource)
            .map(|l| l.max_value)
            .unwrap_or(0.0);

        ResourceUsage::new(*resource, current, max)
    }

    /// 更新资源用量
    ///
    /// # Arguments
    /// * `resource` - 资源类型
    /// * `current` - 当前用量
    ///
    /// # Returns
    /// 检查结果
    pub fn update_usage(&mut self, resource: ResourceType, current: f64) -> ResourceCheckResult {
        self.usage.insert(resource, current);

        let limit = self.limits.get(&resource);
        match limit {
            Some(limit) => {
                let result = limit.check_usage(current);

                match result {
                    ResourceCheckResult::Exceeded => {
                        tracing::warn!(
                            resource = %resource,
                            current = current,
                            max = limit.max_value,
                            "Resource limit exceeded"
                        );
                    }
                    ResourceCheckResult::Warning => {
                        tracing::warn!(
                            resource = %resource,
                            current = current,
                            max = limit.max_value,
                            threshold = limit.warn_threshold,
                            "Resource usage approaching limit"
                        );
                    }
                    ResourceCheckResult::Ok => {
                        tracing::debug!(
                            resource = %resource,
                            current = current,
                            max = limit.max_value,
                            "Resource usage normal"
                        );
                    }
                }

                result
            }
            None => ResourceCheckResult::Ok,
        }
    }

    /// 列出所有资源限制和用量
    pub fn list_all(&self) -> Vec<(ResourceType, ResourceLimit, ResourceUsage)> {
        self.limits
            .iter()
            .map(|(rt, limit)| {
                let usage = self.get_usage(rt);
                (*rt, limit.clone(), usage)
            })
            .collect()
    }
}

impl Default for ResourceLimitManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_type_parse() {
        assert_eq!(ResourceType::from_str("cpu"), Some(ResourceType::Cpu));
        assert_eq!(ResourceType::from_str("CPU"), Some(ResourceType::Cpu));
        assert_eq!(ResourceType::from_str("memory"), Some(ResourceType::Memory));
        assert_eq!(ResourceType::from_str("mem"), Some(ResourceType::Memory));
        assert_eq!(ResourceType::from_str("disk"), Some(ResourceType::Disk));
        assert_eq!(
            ResourceType::from_str("execution_time"),
            Some(ResourceType::ExecutionTime)
        );
        assert_eq!(
            ResourceType::from_str("time"),
            Some(ResourceType::ExecutionTime)
        );
        assert_eq!(
            ResourceType::from_str("connections"),
            Some(ResourceType::NetworkConnections)
        );
        assert!(ResourceType::from_str("unknown").is_none());
    }

    #[test]
    fn test_resource_type_display() {
        assert_eq!(ResourceType::Cpu.to_string(), "cpu");
        assert_eq!(ResourceType::Memory.to_string(), "memory");
        assert_eq!(ResourceType::ExecutionTime.to_string(), "execution_time");
    }

    #[test]
    fn test_resource_type_defaults() {
        assert_eq!(ResourceType::Cpu.default_limit_value(), 80.0);
        assert_eq!(
            ResourceType::Memory.default_limit_value(),
            512.0 * 1024.0 * 1024.0
        );
        assert_eq!(ResourceType::ExecutionTime.default_limit_value(), 300.0);
    }

    #[test]
    fn test_resource_limit_new() {
        let limit = ResourceLimit::new(ResourceType::Cpu, 90.0);
        assert_eq!(limit.resource_type, ResourceType::Cpu);
        assert_eq!(limit.max_value, 90.0);
        assert_eq!(limit.warn_threshold, 0.8);
        assert!(limit.enabled);
    }

    #[test]
    fn test_resource_limit_with_warn_threshold() {
        let limit = ResourceLimit::new(ResourceType::Memory, 1024.0).with_warn_threshold(0.9);
        assert_eq!(limit.warn_threshold, 0.9);
    }

    #[test]
    fn test_resource_limit_warn_threshold_clamp() {
        let limit = ResourceLimit::new(ResourceType::Cpu, 100.0).with_warn_threshold(1.5);
        assert_eq!(limit.warn_threshold, 1.0);

        let limit = ResourceLimit::new(ResourceType::Cpu, 100.0).with_warn_threshold(-0.5);
        assert_eq!(limit.warn_threshold, 0.0);
    }

    #[test]
    fn test_resource_limit_disabled() {
        let limit = ResourceLimit::new(ResourceType::Cpu, 80.0).disabled();
        assert!(!limit.enabled);

        // 禁用的限制总是返回 Ok
        assert_eq!(limit.check_usage(999.0), ResourceCheckResult::Ok);
    }

    #[test]
    fn test_resource_limit_check_usage() {
        let limit = ResourceLimit::new(ResourceType::Cpu, 100.0).with_warn_threshold(0.8);

        assert_eq!(limit.check_usage(50.0), ResourceCheckResult::Ok);
        assert_eq!(limit.check_usage(80.0), ResourceCheckResult::Warning);
        assert_eq!(limit.check_usage(85.0), ResourceCheckResult::Warning);
        assert_eq!(limit.check_usage(100.0), ResourceCheckResult::Exceeded);
        assert_eq!(limit.check_usage(150.0), ResourceCheckResult::Exceeded);
    }

    #[test]
    fn test_resource_usage_new() {
        let usage = ResourceUsage::new(ResourceType::Memory, 256.0, 512.0);
        assert_eq!(usage.current_value, 256.0);
        assert_eq!(usage.max_value, 512.0);
        assert!((usage.usage_percent - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_resource_usage_zero_max() {
        let usage = ResourceUsage::new(ResourceType::Memory, 100.0, 0.0);
        assert_eq!(usage.usage_percent, 0.0);
    }

    #[test]
    fn test_resource_usage_capped_at_100() {
        let usage = ResourceUsage::new(ResourceType::Cpu, 150.0, 100.0);
        assert_eq!(usage.usage_percent, 100.0);
    }

    #[test]
    fn test_resource_limit_manager_new() {
        let manager = ResourceLimitManager::new();

        // 默认注册了所有资源类型
        assert!(manager.get_limit(&ResourceType::Cpu).is_some());
        assert!(manager.get_limit(&ResourceType::Memory).is_some());
        assert!(manager.get_limit(&ResourceType::Disk).is_some());
        assert!(manager.get_limit(&ResourceType::ExecutionTime).is_some());
        assert!(manager
            .get_limit(&ResourceType::NetworkConnections)
            .is_some());
    }

    #[test]
    fn test_resource_limit_manager_set_limit() {
        let mut manager = ResourceLimitManager::new();

        let limit = ResourceLimit::new(ResourceType::Cpu, 95.0);
        manager.set_limit(ResourceType::Cpu, limit);

        let limit = manager.get_limit(&ResourceType::Cpu).unwrap();
        assert_eq!(limit.max_value, 95.0);
    }

    #[test]
    fn test_resource_limit_manager_update_usage() {
        let mut manager = ResourceLimitManager::new();

        // 正常用量
        let result = manager.update_usage(ResourceType::Cpu, 50.0);
        assert_eq!(result, ResourceCheckResult::Ok);

        // 接近限制
        let result = manager.update_usage(ResourceType::Cpu, 70.0);
        assert_eq!(result, ResourceCheckResult::Warning);

        // 超限
        let result = manager.update_usage(ResourceType::Cpu, 85.0);
        assert_eq!(result, ResourceCheckResult::Exceeded);
    }

    #[test]
    fn test_resource_limit_manager_get_usage() {
        let mut manager = ResourceLimitManager::new();

        manager.update_usage(ResourceType::Memory, 256.0 * 1024.0 * 1024.0);

        let usage = manager.get_usage(&ResourceType::Memory);
        assert_eq!(usage.resource_type, ResourceType::Memory);
        assert_eq!(usage.current_value, 256.0 * 1024.0 * 1024.0);
        assert!((usage.usage_percent - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_resource_limit_manager_list_all() {
        let mut manager = ResourceLimitManager::new();
        manager.update_usage(ResourceType::Cpu, 30.0);
        manager.update_usage(ResourceType::Memory, 100.0);

        let all = manager.list_all();
        assert_eq!(all.len(), 5); // 5 种默认资源类型
    }

    #[test]
    fn test_resource_limit_manager_custom_limit() {
        let mut manager = ResourceLimitManager::new();

        let limit = ResourceLimit::new(ResourceType::ExecutionTime, 60.0).with_warn_threshold(0.9);
        manager.set_limit(ResourceType::ExecutionTime, limit);

        let result = manager.update_usage(ResourceType::ExecutionTime, 55.0);
        assert_eq!(result, ResourceCheckResult::Warning);

        let result = manager.update_usage(ResourceType::ExecutionTime, 65.0);
        assert_eq!(result, ResourceCheckResult::Exceeded);
    }

    #[test]
    fn test_resource_type_serialization() {
        let rt = ResourceType::Cpu;
        let json = serde_json::to_string(&rt).unwrap();
        let deserialized: ResourceType = serde_json::from_str(&json).unwrap();
        assert_eq!(rt, deserialized);
    }

    #[test]
    fn test_resource_limit_serialization() {
        let limit = ResourceLimit::new(ResourceType::Memory, 1024.0);
        let json = serde_json::to_string(&limit).unwrap();
        let deserialized: ResourceLimit = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.resource_type, ResourceType::Memory);
        assert_eq!(deserialized.max_value, 1024.0);
    }

    #[test]
    fn test_resource_limit_manager_default() {
        let manager = ResourceLimitManager::default();
        assert!(manager.get_limit(&ResourceType::Cpu).is_some());
    }
}
