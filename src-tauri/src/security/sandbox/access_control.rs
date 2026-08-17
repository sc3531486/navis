//! 路径访问控制
//!
//! 基于设计文档 §3 实现，提供路径白名单/黑名单校验。
//!
//! # 设计思路
//! - 白名单路径：允许访问的目录或文件前缀
//! - 黑名单路径：禁止访问的目录或文件前缀（优先级高于白名单）
//! - 匹配逻辑：规范化路径后，使用前缀匹配（目录需以 `/` 结尾表示目录前缀）
//! - 校验流程：黑名单优先 → 白名单 → 默认拒绝

use std::path::Path;

use super::permission::{CheckResult, OperationType, PermissionLevel};

// ============================================================================
// 访问控制规则
// ============================================================================

/// 路径访问规则
#[derive(Debug, Clone)]
pub struct PathRule {
    /// 匹配模式（路径前缀或通配符）
    pub pattern: String,
    /// 是否允许（true=白名单，false=黑名单）
    pub allowed: bool,
    /// 描述
    pub description: String,
}

// ============================================================================
// AccessControl
// ============================================================================

/// 路径访问控制器
///
/// 管理路径白名单/黑名单，提供路径校验能力。
#[derive(Debug)]
pub struct AccessControl {
    /// 路径规则列表（有序，黑名单在前优先匹配）
    rules: Vec<PathRule>,
}

impl AccessControl {
    /// 创建新的路径访问控制器
    pub fn new() -> Self {
        tracing::debug!("Creating new AccessControl");
        Self { rules: Vec::new() }
    }

    /// 添加白名单路径
    ///
    /// # Arguments
    /// * `pattern` - 路径模式（目录需以 `/` 或 `\` 结尾）
    /// * `description` - 规则描述
    pub fn allow(&mut self, pattern: &str, description: &str) {
        tracing::debug!(pattern = %pattern, description = %description, "Adding allow rule");
        self.rules.push(PathRule {
            pattern: normalize_path(pattern),
            allowed: true,
            description: description.to_string(),
        });
    }

    /// 添加黑名单路径
    ///
    /// # Arguments
    /// * `pattern` - 路径模式
    /// * `description` - 规则描述
    pub fn deny(&mut self, pattern: &str, description: &str) {
        tracing::debug!(pattern = %pattern, description = %description, "Adding deny rule");
        self.rules.push(PathRule {
            pattern: normalize_path(pattern),
            allowed: false,
            description: description.to_string(),
        });
    }

    /// 校验路径是否允许访问
    ///
    /// # Arguments
    /// * `path` - 待校验的路径
    /// * `operation` - 操作类型
    /// * `worktree` - worktree根路径
    ///
    /// # Returns
    /// 校验结果
    pub fn check(&self, path: &Path, operation: &OperationType, worktree: &Path) -> CheckResult {
        let path_normalized = normalize_path(&path.to_string_lossy());
        let worktree_normalized = normalize_path(&worktree.to_string_lossy());

        tracing::debug!(
            path = %path_normalized,
            operation = %operation,
            worktree = %worktree_normalized,
            "Checking path access"
        );

        // 检查路径是否在worktree内
        if !path_normalized.starts_with(&worktree_normalized) && !path_normalized.starts_with("/") {
            return CheckResult::denied(
                PermissionLevel::UserConfirm,
                format!(
                    "路径 {} 不在worktree {} 内",
                    path_normalized, worktree_normalized
                ),
            );
        }

        // 黑名单优先匹配
        for rule in &self.rules {
            if !rule.allowed && matches_pattern(&path_normalized, &rule.pattern) {
                tracing::warn!(
                    path = %path_normalized,
                    rule = %rule.description,
                    "Path blocked by deny rule"
                );
                return CheckResult::denied(
                    PermissionLevel::UserConfirm,
                    format!("路径被禁止: {} ({})", path_normalized, rule.description),
                );
            }
        }

        // 白名单匹配
        for rule in &self.rules {
            if rule.allowed && matches_pattern(&path_normalized, &rule.pattern) {
                tracing::debug!(
                    path = %path_normalized,
                    rule = %rule.description,
                    "Path allowed by allow rule"
                );
                return CheckResult::allowed(PermissionLevel::LightCheck);
            }
        }

        // 无匹配规则，默认根据操作类型返回
        tracing::debug!(path = %path_normalized, "No matching path rule, using default level");
        CheckResult::allowed(operation.default_level())
    }

    /// 获取所有规则
    pub fn rules(&self) -> &[PathRule] {
        &self.rules
    }

    /// 清除所有规则
    pub fn clear(&mut self) {
        self.rules.clear();
    }

    /// 获取规则数量
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

impl Default for AccessControl {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 规范化路径
///
/// 统一使用 `/` 作为分隔符，去除尾部的分隔符（但保留根路径 `/`）。
fn normalize_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");

    // 去除尾部的 /（保留根路径 /）
    if normalized.len() > 1 && normalized.ends_with('/') {
        normalized[..normalized.len() - 1].to_string()
    } else {
        normalized
    }
}

/// 检查路径是否匹配模式
///
/// 支持两种匹配方式：
/// - 前缀匹配：模式以 `/` 结尾表示目录前缀，路径以该前缀开头即匹配
/// - 精确匹配：路径完全匹配模式
/// - 通配符匹配：模式包含 `*` 时使用通配符
fn matches_pattern(path: &str, pattern: &str) -> bool {
    // 如果模式包含通配符
    if pattern.contains('*') {
        let regex_pattern = pattern.replace('.', "\\.").replace('*', ".*");
        if let Ok(re) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
            return re.is_match(path);
        }
    }

    // 精确匹配
    if path == pattern {
        return true;
    }

    // 前缀匹配（路径以 pattern/ 开头）
    path.starts_with(&format!("{}/", pattern))
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn worktree() -> PathBuf {
        PathBuf::from("/home/user/worktree")
    }

    #[test]
    fn test_access_control_new() {
        let ac = AccessControl::new();
        assert_eq!(ac.rule_count(), 0);
        assert!(ac.rules().is_empty());
    }

    #[test]
    fn test_access_control_allow_deny() {
        let mut ac = AccessControl::new();
        ac.allow("/home/user/worktree/src", "源代码目录");
        ac.deny("/home/user/worktree/node_modules", "依赖目录");

        assert_eq!(ac.rule_count(), 2);
        assert!(ac.rules()[0].allowed);
        assert!(!ac.rules()[1].allowed);
    }

    #[test]
    fn test_access_control_deny_priority() {
        let mut ac = AccessControl::new();
        ac.allow("/home/user/worktree", "worktree");
        ac.deny("/home/user/worktree/.env", "环境变量文件");

        let ws = worktree();

        // .env 文件应该被拒绝（黑名单优先）
        let result = ac.check(
            Path::new("/home/user/worktree/.env"),
            &OperationType::FileRead,
            &ws,
        );
        assert!(!result.allowed);
        assert!(result.reason.as_ref().unwrap().contains("环境变量文件"));
    }

    #[test]
    fn test_access_control_allow_path() {
        let mut ac = AccessControl::new();
        ac.allow("/home/user/worktree/src", "源代码目录");

        let ws = worktree();

        let result = ac.check(
            Path::new("/home/user/worktree/src/main.rs"),
            &OperationType::FileRead,
            &ws,
        );
        assert!(result.allowed);
    }

    #[test]
    fn test_access_control_deny_path() {
        let mut ac = AccessControl::new();
        ac.deny("/home/user/worktree/.git", "Git 目录");

        let ws = worktree();

        let result = ac.check(
            Path::new("/home/user/worktree/.git/config"),
            &OperationType::FileRead,
            &ws,
        );
        assert!(!result.allowed);
    }

    #[test]
    fn test_access_control_wildcard_pattern() {
        let mut ac = AccessControl::new();
        ac.deny("*.env", "环境变量文件");

        let ws = worktree();

        let result = ac.check(
            Path::new("/home/user/worktree/.env"),
            &OperationType::FileRead,
            &ws,
        );
        assert!(!result.allowed);
    }

    #[test]
    fn test_access_control_no_matching_rule() {
        let ac = AccessControl::new();
        let ws = worktree();

        // 没有规则时，默认根据操作类型返回
        let result = ac.check(
            Path::new("/home/user/worktree/src/main.rs"),
            &OperationType::FileRead,
            &ws,
        );
        assert!(result.allowed);
        assert_eq!(result.level, PermissionLevel::LightCheck);
    }

    #[test]
    fn test_access_control_clear() {
        let mut ac = AccessControl::new();
        ac.allow("/path/1", "Rule 1");
        ac.deny("/path/2", "Rule 2");
        assert_eq!(ac.rule_count(), 2);

        ac.clear();
        assert_eq!(ac.rule_count(), 0);
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("/home/user"), "/home/user");
        assert_eq!(normalize_path("/home/user/"), "/home/user");
        assert_eq!(normalize_path("C:\\Users\\admin"), "C:/Users/admin");
        assert_eq!(normalize_path("C:\\Users\\admin\\"), "C:/Users/admin");
        assert_eq!(normalize_path("/"), "/");
    }

    #[test]
    fn test_matches_pattern_exact() {
        assert!(matches_pattern(
            "/home/user/file.txt",
            "/home/user/file.txt"
        ));
        assert!(!matches_pattern(
            "/home/user/file.txt",
            "/home/user/other.txt"
        ));
    }

    #[test]
    fn test_matches_pattern_prefix() {
        assert!(matches_pattern("/home/user/src/main.rs", "/home/user/src"));
        assert!(matches_pattern(
            "/home/user/src/sub/file.rs",
            "/home/user/src"
        ));
        assert!(!matches_pattern("/home/user/lib/file.rs", "/home/user/src"));
    }

    #[test]
    fn test_matches_pattern_wildcard() {
        assert!(matches_pattern(".env", "*.env"));
        assert!(matches_pattern("file.env", "*.env"));
        assert!(!matches_pattern("file.txt", "*.env"));
    }

    #[test]
    fn test_check_windows_path() {
        let mut ac = AccessControl::new();
        ac.allow("C:/Users/admin/worktree/src", "源代码目录");

        let ws = PathBuf::from("C:/Users/admin/worktree");

        let result = ac.check(
            Path::new("C:/Users/admin/worktree/src/main.rs"),
            &OperationType::FileRead,
            &ws,
        );
        assert!(result.allowed);
    }

    #[test]
    fn test_check_result_contains_path_info() {
        let mut ac = AccessControl::new();
        ac.deny("/home/user/worktree/.secret", "机密文件");

        let ws = worktree();

        let result = ac.check(
            Path::new("/home/user/worktree/.secret"),
            &OperationType::FileRead,
            &ws,
        );
        assert!(!result.allowed);
        let reason = result.reason.unwrap();
        assert!(reason.contains(".secret"));
        assert!(reason.contains("机密文件"));
    }
}
