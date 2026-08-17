//! 策略配置
//!
//! 基于设计文档 §5 / §7.5 实现，定义网络策略、命令规则配置等可序列化的策略结构。
//!
//! # 数据模型
//! - `NetworkPolicy` - 网络访问策略（仅黑名单，默认放行）
//! - `CommandRuleConfig` - 命令规则配置条目
//! - `SandboxPolicy` - 沙箱总策略

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::command_rules::{CommandRule, RuleAction};

// ============================================================================
// 网络策略
// ============================================================================

/// 网络策略
///
/// 控制 Agent 工具调用产生的网络请求。仅保留黑名单，其余默认放行。
/// Gateway 模型连接不经过此策略，由 Auth/Config 管控。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    /// 黑名单域名（已知恶意域名，默认内置）
    pub blocked_domains: Vec<String>,
}

impl Default for NetworkPolicy {
    /// 默认网络策略：空黑名单，不拦截任何域名
    fn default() -> Self {
        Self {
            blocked_domains: Vec::new(),
        }
    }
}

impl NetworkPolicy {
    /// 创建带自定义黑名单的策略
    pub fn with_blocked(blocked_domains: Vec<String>) -> Self {
        Self { blocked_domains }
    }

    /// 检查域名是否在黑名单中
    pub fn is_blocked(&self, domain: &str) -> bool {
        self.blocked_domains
            .iter()
            .any(|d| domain_matches(domain, d))
    }

    /// 从配置文件加载黑名单，合并用户级和项目级
    ///
    /// 加载顺序：
    /// 1. `~/.navis/sandbox.toml`（用户全局）
    /// 2. `<project>/.navis/sandbox.toml`（项目级）
    /// 3. 合并去重
    pub fn load_from_config(user_home: Option<&Path>, project_dir: Option<&Path>) -> Self {
        let mut blocked = Vec::new();

        // 用户全局配置
        if let Some(home) = user_home {
            let user_config = home.join(".navis").join("sandbox.toml");
            if let Some(domains) = load_blocked_domains_from_toml(&user_config) {
                blocked.extend(domains);
            }
        }

        // 项目级配置
        if let Some(project) = project_dir {
            let project_config = project.join(".navis").join("sandbox.toml");
            if let Some(domains) = load_blocked_domains_from_toml(&project_config) {
                blocked.extend(domains);
            }
        }

        // 去重
        blocked.sort();
        blocked.dedup();

        Self {
            blocked_domains: blocked,
        }
    }
}

/// 从 TOML 配置文件中读取 blocked_domains 列表
fn load_blocked_domains_from_toml(path: &Path) -> Option<Vec<String>> {
    let content = std::fs::read_to_string(path).ok()?;
    let value: toml::Value = toml::from_str(&content).ok()?;
    let blocked = value
        .get("network_policy")?
        .get("blocked_domains")?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    Some(blocked)
}

/// 域名匹配（支持子域名通配符）
///
/// 规则：
/// - "example.com" 匹配 "example.com" 和 "sub.example.com"
/// - "api.github.com" 精确匹配 "api.github.com"
pub fn domain_matches(domain: &str, pattern: &str) -> bool {
    if domain == pattern {
        return true;
    }
    domain.ends_with(&format!(".{}", pattern))
}

// ============================================================================
// 命令规则配置
// ============================================================================

/// 命令规则配置条目
///
/// 用于序列化/反序列化配置文件中的命令规则。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRuleConfig {
    /// 正则表达式模式
    pub pattern: String,
    /// 规则动作
    pub action: RuleActionConfig,
    /// 描述
    pub description: String,
}

/// 规则动作（配置格式）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleActionConfig {
    /// 永久拒绝
    Deny,
    /// 需要用户确认
    Confirm,
    /// 允许
    Allow,
}

impl RuleActionConfig {
    /// 转换为内部 RuleAction
    pub fn to_rule_action(&self) -> RuleAction {
        match self {
            RuleActionConfig::Deny => RuleAction::Deny,
            RuleActionConfig::Confirm => RuleAction::Confirm,
            RuleActionConfig::Allow => RuleAction::Allow,
        }
    }
}

impl CommandRuleConfig {
    /// 转换为内部 CommandRule
    pub fn to_command_rule(&self) -> Result<CommandRule, String> {
        // 验证正则表达式的合法性
        regex::Regex::new(&self.pattern)
            .map_err(|e| format!("无效的正则表达式 '{}': {}", self.pattern, e))?;

        Ok(CommandRule {
            pattern: self.pattern.clone(),
            action: self.action.to_rule_action(),
            description: self.description.clone(),
        })
    }
}

// ============================================================================
// 沙箱总策略
// ============================================================================

/// 沙箱总策略
///
/// 聚合所有沙箱子策略的配置结构，用于一次性加载/保存。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPolicy {
    /// 命令规则列表
    pub command_rules: Vec<CommandRuleConfig>,
    /// 网络策略
    pub network_policy: NetworkPolicy,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            command_rules: default_command_rules(),
            network_policy: NetworkPolicy::default(),
        }
    }
}

/// 默认命令规则（对标设计文档 §5）
///
/// 包含黑名单（永久拒绝）、需确认、白名单（无需确认）三类规则。
pub fn default_command_rules() -> Vec<CommandRuleConfig> {
    vec![
        // 黑名单（永远禁止）
        CommandRuleConfig {
            pattern: r"^rm\s+-rf\s+/$".to_string(),
            action: RuleActionConfig::Deny,
            description: "禁止删除根目录".to_string(),
        },
        CommandRuleConfig {
            pattern: r"^sudo\s+rm".to_string(),
            action: RuleActionConfig::Deny,
            description: "禁止 sudo 删除".to_string(),
        },
        CommandRuleConfig {
            pattern: r":\(\)\{.*:\|.*\}".to_string(),
            action: RuleActionConfig::Deny,
            description: "禁止 fork bomb".to_string(),
        },
        // 需要确认
        CommandRuleConfig {
            pattern: r"^sudo\s+".to_string(),
            action: RuleActionConfig::Confirm,
            description: "sudo 命令需要确认".to_string(),
        },
        CommandRuleConfig {
            pattern: r"^git\s+push".to_string(),
            action: RuleActionConfig::Confirm,
            description: "Git 推送需要确认".to_string(),
        },
        CommandRuleConfig {
            pattern: r"^git\s+reset\s+--hard".to_string(),
            action: RuleActionConfig::Confirm,
            description: "Git 硬重置需要确认".to_string(),
        },
        CommandRuleConfig {
            pattern: r"rm\s+.*-r".to_string(),
            action: RuleActionConfig::Confirm,
            description: "递归删除需要确认".to_string(),
        },
        // 白名单（无需确认）
        CommandRuleConfig {
            pattern: r"^git\s+(status|diff|log|branch)".to_string(),
            action: RuleActionConfig::Allow,
            description: "Git 只读操作".to_string(),
        },
        CommandRuleConfig {
            pattern: r"^(npm|cargo|pnpm)\s+(test|run|build)".to_string(),
            action: RuleActionConfig::Allow,
            description: "包管理器常用命令".to_string(),
        },
    ]
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_policy_default() {
        let policy = NetworkPolicy::default();
        assert!(policy.blocked_domains.is_empty());
    }

    #[test]
    fn test_network_policy_with_blocked() {
        let policy = NetworkPolicy::with_blocked(vec!["evil.com".to_string()]);
        assert_eq!(policy.blocked_domains.len(), 1);
        assert!(policy.is_blocked("evil.com"));
        assert!(policy.is_blocked("sub.evil.com"));
        assert!(!policy.is_blocked("good.com"));
    }

    #[test]
    fn test_network_policy_localhost_allowed_by_default() {
        let policy = NetworkPolicy::default();
        assert!(!policy.is_blocked("localhost"));
        assert!(!policy.is_blocked("127.0.0.1"));
        assert!(!policy.is_blocked("192.168.1.1"));
        assert!(!policy.is_blocked("10.0.0.1"));
    }

    #[test]
    fn test_domain_matches() {
        assert!(domain_matches("example.com", "example.com"));
        assert!(domain_matches("sub.example.com", "example.com"));
        assert!(domain_matches("deep.sub.example.com", "example.com"));
        assert!(!domain_matches("notexample.com", "example.com"));
        assert!(!domain_matches("example.org", "example.com"));
    }

    #[test]
    fn test_rule_action_config_serde() {
        let json = r#""deny""#;
        let action: RuleActionConfig = serde_json::from_str(json).unwrap();
        assert_eq!(action, RuleActionConfig::Deny);

        let json = r#""confirm""#;
        let action: RuleActionConfig = serde_json::from_str(json).unwrap();
        assert_eq!(action, RuleActionConfig::Confirm);

        let json = r#""allow""#;
        let action: RuleActionConfig = serde_json::from_str(json).unwrap();
        assert_eq!(action, RuleActionConfig::Allow);
    }

    #[test]
    fn test_command_rule_config_to_rule() {
        let config = CommandRuleConfig {
            pattern: r"^git\s+status$".to_string(),
            action: RuleActionConfig::Allow,
            description: "Git 状态查看".to_string(),
        };

        let rule = config.to_command_rule().unwrap();
        assert_eq!(rule.pattern, r"^git\s+status$");
        assert_eq!(rule.action, RuleAction::Allow);
        assert_eq!(rule.description, "Git 状态查看");
    }

    #[test]
    fn test_command_rule_config_invalid_regex() {
        let config = CommandRuleConfig {
            pattern: r"[invalid".to_string(),
            action: RuleActionConfig::Deny,
            description: "无效规则".to_string(),
        };

        let result = config.to_command_rule();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("无效的正则表达式"));
    }

    #[test]
    fn test_default_command_rules() {
        let rules = default_command_rules();
        assert!(!rules.is_empty());

        let deny_rules: Vec<_> = rules
            .iter()
            .filter(|r| r.action == RuleActionConfig::Deny)
            .collect();
        assert!(!deny_rules.is_empty());

        let confirm_rules: Vec<_> = rules
            .iter()
            .filter(|r| r.action == RuleActionConfig::Confirm)
            .collect();
        assert!(!confirm_rules.is_empty());

        let allow_rules: Vec<_> = rules
            .iter()
            .filter(|r| r.action == RuleActionConfig::Allow)
            .collect();
        assert!(!allow_rules.is_empty());

        for rule in &rules {
            let result = rule.to_command_rule();
            assert!(
                result.is_ok(),
                "规则 '{}' 的正则表达式无效: {:?}",
                rule.pattern,
                result.err()
            );
        }
    }

    #[test]
    fn test_sandbox_policy_default() {
        let policy = SandboxPolicy::default();
        assert!(!policy.command_rules.is_empty());
        assert!(policy.network_policy.blocked_domains.is_empty());
    }

    #[test]
    fn test_sandbox_policy_serialization() {
        let policy = SandboxPolicy::default();
        let json = serde_json::to_string_pretty(&policy).unwrap();
        let deserialized: SandboxPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.command_rules.len(), policy.command_rules.len());
        assert_eq!(
            deserialized.network_policy.blocked_domains.len(),
            policy.network_policy.blocked_domains.len()
        );
    }

    #[test]
    fn test_network_policy_serialization() {
        let policy = NetworkPolicy {
            blocked_domains: vec!["evil.com".to_string()],
        };

        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: NetworkPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.blocked_domains, vec!["evil.com"]);
    }

    #[test]
    fn test_load_blocked_domains_from_toml() {
        let dir = tempfile::tempdir().unwrap();
        let sandbox_dir = dir.path().join(".navis");
        std::fs::create_dir_all(&sandbox_dir).unwrap();

        let toml_content = r#"
[network_policy]
blocked_domains = ["evil.com", "cryptominer.pool.com"]
"#;
        std::fs::write(sandbox_dir.join("sandbox.toml"), toml_content).unwrap();

        let policy = NetworkPolicy::load_from_config(None, Some(dir.path()));
        assert_eq!(policy.blocked_domains.len(), 2);
        assert!(policy.is_blocked("evil.com"));
        assert!(policy.is_blocked("sub.evil.com"));
        assert!(policy.is_blocked("cryptominer.pool.com"));
        assert!(!policy.is_blocked("good.com"));
    }

    #[test]
    fn test_load_config_no_file() {
        let policy = NetworkPolicy::load_from_config(None, Some(Path::new("/nonexistent")));
        assert!(policy.blocked_domains.is_empty());
    }

    #[test]
    fn test_load_config_merge_user_and_project() {
        let user_dir = tempfile::tempdir().unwrap();
        let project_dir = tempfile::tempdir().unwrap();

        // 用户级配置
        let user_sandbox = user_dir.path().join(".navis");
        std::fs::create_dir_all(&user_sandbox).unwrap();
        std::fs::write(
            user_sandbox.join("sandbox.toml"),
            r#"[network_policy]
blocked_domains = ["evil.com"]"#,
        )
        .unwrap();

        // 项目级配置
        let project_sandbox = project_dir.path().join(".navis");
        std::fs::create_dir_all(&project_sandbox).unwrap();
        std::fs::write(
            project_sandbox.join("sandbox.toml"),
            r#"[network_policy]
blocked_domains = ["malware.com"]"#,
        )
        .unwrap();

        let policy =
            NetworkPolicy::load_from_config(Some(user_dir.path()), Some(project_dir.path()));
        assert_eq!(policy.blocked_domains.len(), 2);
        assert!(policy.is_blocked("evil.com"));
        assert!(policy.is_blocked("malware.com"));
    }

    #[test]
    fn test_load_config_dedup() {
        let user_dir = tempfile::tempdir().unwrap();
        let project_dir = tempfile::tempdir().unwrap();

        let user_sandbox = user_dir.path().join(".navis");
        std::fs::create_dir_all(&user_sandbox).unwrap();
        std::fs::write(
            user_sandbox.join("sandbox.toml"),
            r#"[network_policy]
blocked_domains = ["evil.com"]"#,
        )
        .unwrap();

        let project_sandbox = project_dir.path().join(".navis");
        std::fs::create_dir_all(&project_sandbox).unwrap();
        std::fs::write(
            project_sandbox.join("sandbox.toml"),
            r#"[network_policy]
blocked_domains = ["evil.com", "malware.com"]"#,
        )
        .unwrap();

        let policy =
            NetworkPolicy::load_from_config(Some(user_dir.path()), Some(project_dir.path()));
        // evil.com 重复，去重后只有 2 个
        assert_eq!(policy.blocked_domains.len(), 2);
    }
}
