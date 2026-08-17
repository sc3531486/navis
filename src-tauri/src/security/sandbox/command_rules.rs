//! 命令规则引擎
//!
//! 基于设计文档 §5 实现，提供命令黑白名单、正则匹配。
//!
//! # 设计思路
//! - 命令规则列表按顺序匹配（黑名单规则通常放在前面）
//! - 匹配到 Deny 规则 → 拒绝
//! - 匹配到 Confirm 规则 → 需要用户确认
//! - 匹配到 Allow 规则 → 允许
//! - 无匹配规则 → 默认需要确认

use regex::Regex;

use super::permission::{CheckResult, PermissionLevel};

// ============================================================================
// 终端命令语义
// ============================================================================

/// 命令所属 shell。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandShell {
    Bash,
    PowerShell,
    Cmd,
}

/// 危险命令 warning 分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DangerousCommandCategory {
    DataLoss,
    RemoteHistoryRewrite,
    FileDeletion,
    DatabaseMutation,
    InfrastructureMutation,
    SafetyBypass,
    SystemDamage,
    CredentialOrConfigWrite,
}

impl std::fmt::Display for DangerousCommandCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DangerousCommandCategory::DataLoss => write!(f, "data_loss"),
            DangerousCommandCategory::RemoteHistoryRewrite => write!(f, "remote_history_rewrite"),
            DangerousCommandCategory::FileDeletion => write!(f, "file_deletion"),
            DangerousCommandCategory::DatabaseMutation => write!(f, "database_mutation"),
            DangerousCommandCategory::InfrastructureMutation => {
                write!(f, "infrastructure_mutation")
            }
            DangerousCommandCategory::SafetyBypass => write!(f, "safety_bypass"),
            DangerousCommandCategory::SystemDamage => write!(f, "system_damage"),
            DangerousCommandCategory::CredentialOrConfigWrite => {
                write!(f, "credential_or_config_write")
            }
        }
    }
}

/// 危险命令 warning。该结果只用于提示/确认，不等同于永久拒绝。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DangerousCommandWarning {
    pub category: DangerousCommandCategory,
    pub message: String,
}

/// 判断某个 shell 命令是否可作为只读命令自动放行。
pub fn is_read_only_command(command: &str, shell: CommandShell) -> bool {
    let trimmed = command.trim();
    if trimmed.is_empty()
        || contains_unquoted_output_redirection(trimmed)
        || contains_unquoted_command_substitution(trimmed)
        || has_trailing_unquoted_background(trimmed)
    {
        return false;
    }

    let segments = split_command_segments(trimmed);
    if segments.is_empty() {
        return false;
    }

    segments
        .iter()
        .all(|segment| is_read_only_segment(segment, shell))
}

/// 判断命令是否在任一受支持 shell 中具有只读语义。
pub fn is_read_only_command_any_shell(command: &str) -> bool {
    [
        CommandShell::Bash,
        CommandShell::PowerShell,
        CommandShell::Cmd,
    ]
    .iter()
    .any(|shell| is_read_only_command(command, *shell))
}

/// 返回命中的危险命令 warning。
pub fn dangerous_command_warning(command: &str) -> Option<DangerousCommandWarning> {
    let checks: &[(DangerousCommandCategory, &str, &str)] = &[
        (
            DangerousCommandCategory::SystemDamage,
            r"(?i)(^|[;&|\n]\s*)(shutdown|reboot|halt|poweroff|systemctl\s+(poweroff|reboot|halt)|mkfs(\.[a-z0-9]+)?|dd\s+.*\bof=/dev/(sd|nvme|hd|mmcblk|vd|xvd))\b",
            "命令可能影响系统可用性或写入块设备",
        ),
        (
            DangerousCommandCategory::DataLoss,
            r"(?i)\bgit\s+reset\s+--hard\b",
            "Git hard reset 可能丢弃未提交修改",
        ),
        (
            DangerousCommandCategory::RemoteHistoryRewrite,
            r"(?i)\bgit\s+push\b[^;&|\n]*(--force|--force-with-lease|-f)\b",
            "Git force push 可能覆盖远端历史",
        ),
        (
            DangerousCommandCategory::FileDeletion,
            r"(?i)\bgit\s+clean\b(?![^;&|\n]*(?:-[a-z]*n|--dry-run))[^;&|\n]*-[a-z]*f",
            "Git clean 可能永久删除未跟踪文件",
        ),
        (
            DangerousCommandCategory::DataLoss,
            r"(?i)\bgit\s+(checkout|restore)\s+(--\s+)?\.[ \t]*($|[;&|\n])",
            "命令可能丢弃worktree修改",
        ),
        (
            DangerousCommandCategory::DataLoss,
            r"(?i)\bgit\s+stash[ \t]+(drop|clear)\b",
            "命令可能永久移除 stash",
        ),
        (
            DangerousCommandCategory::FileDeletion,
            r"(?i)\bgit\s+branch\s+(-D[ \t]|--delete\s+--force|--force\s+--delete)\b",
            "命令可能强制删除分支",
        ),
        (
            DangerousCommandCategory::SafetyBypass,
            r"(?i)\bgit\s+(commit|push|merge)\b[^;&|\n]*--no-verify\b",
            "命令可能跳过安全 hooks",
        ),
        (
            DangerousCommandCategory::SafetyBypass,
            r"(?i)\bgit\s+commit\b[^;&|\n]*--amend\b",
            "命令可能改写最近一次提交",
        ),
        (
            DangerousCommandCategory::FileDeletion,
            r"(?i)(^|[;&|\n]\s*)(rm\s+-[a-z]*[rR][a-z]*f|rm\s+-[a-z]*f[a-z]*[rR]|del\s+[^;&|\n]*(/s|/q)|erase\s+[^;&|\n]*(/s|/q)|remove-item\b[^;&|\n]*(-recurse|-force))",
            "命令可能递归或强制删除文件",
        ),
        (
            DangerousCommandCategory::DatabaseMutation,
            r"(?i)\b(DROP|TRUNCATE)\s+(TABLE|DATABASE|SCHEMA)\b",
            "命令可能删除或清空数据库对象",
        ),
        (
            DangerousCommandCategory::DatabaseMutation,
            r#"(?i)\bDELETE\s+FROM\s+\w+[ \t]*(;|"|'|\n|$)"#,
            "命令可能删除数据库表中的所有行",
        ),
        (
            DangerousCommandCategory::InfrastructureMutation,
            r"(?i)\b(kubectl\s+delete|terraform\s+destroy|docker\s+compose\s+(down|restart|stop|kill)|docker\s+(restart|stop|kill))\b",
            "命令可能删除、停止或重启基础设施资源",
        ),
        (
            DangerousCommandCategory::CredentialOrConfigWrite,
            r"(?i)(>\s*[^;&|\n]*(\.env|config\.yaml|\.ssh|\.npmrc|\.pypirc|\.netrc)|\b(set-content|add-content|out-file|tee-object|tee)\b[^;&|\n]*(\.env|config\.yaml|\.ssh|\.npmrc|\.pypirc|\.netrc))",
            "命令可能写入凭据或安全配置文件",
        ),
    ];

    for (category, pattern, message) in checks {
        if Regex::new(pattern)
            .map(|regex| regex.is_match(command))
            .unwrap_or(false)
        {
            return Some(DangerousCommandWarning {
                category: *category,
                message: (*message).to_string(),
            });
        }
    }

    None
}

fn is_read_only_segment(segment: &str, shell: CommandShell) -> bool {
    let tokens = tokenize_command(segment);
    if tokens.is_empty() {
        return false;
    }

    let (command, args) = command_and_args_for_shell(&tokens, shell);
    let Some(command) = command else {
        return false;
    };

    let command = normalize_command_name(command);
    if command == "git" {
        return is_read_only_git(args);
    }

    match shell {
        CommandShell::Bash => is_read_only_bash_command(&command, args),
        CommandShell::PowerShell => is_read_only_powershell_command(&command, args),
        CommandShell::Cmd => is_read_only_cmd_command(&command, args),
    }
}

fn command_and_args_for_shell<'a>(
    tokens: &'a [String],
    shell: CommandShell,
) -> (Option<&'a str>, &'a [String]) {
    if matches!(shell, CommandShell::PowerShell)
        && matches!(tokens.first().map(String::as_str), Some("&" | "."))
    {
        return (
            tokens.get(1).map(String::as_str),
            tokens.get(2..).unwrap_or(&[]),
        );
    }

    (
        tokens.first().map(String::as_str),
        tokens.get(1..).unwrap_or(&[]),
    )
}

fn normalize_command_name(command: &str) -> String {
    command
        .trim_matches(|c| c == '"' || c == '\'')
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(command)
        .to_ascii_lowercase()
        .trim_end_matches(".exe")
        .trim_end_matches(".cmd")
        .trim_end_matches(".bat")
        .to_string()
}

fn is_read_only_git(args: &[String]) -> bool {
    if args.iter().any(|arg| {
        let lower = arg.to_ascii_lowercase();
        lower == "-c"
            || lower.starts_with("-c=")
            || lower.starts_with("--exec-path")
            || lower.starts_with("--config-env")
    }) {
        return false;
    }

    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        if arg == "--no-pager" || arg == "--version" || arg == "--help" {
            index += 1;
            continue;
        }
        if matches!(arg, "-C" | "--git-dir" | "--work-tree") {
            index += 2;
            continue;
        }
        break;
    }

    let Some(subcommand) = args.get(index).map(|s| s.to_ascii_lowercase()) else {
        return false;
    };
    let sub_args = args.get(index + 1..).unwrap_or(&[]);

    match subcommand.as_str() {
        "status" | "diff" | "log" | "show" | "ls-files" | "blame" | "rev-parse" | "rev-list"
        | "describe" | "remote" => true,
        "branch" => !sub_args.iter().any(|arg| {
            matches!(
                arg.as_str(),
                "-d" | "-D" | "-m" | "-M" | "--delete" | "--move" | "--copy" | "-c" | "-C"
            )
        }),
        "tag" => !sub_args.iter().any(|arg| {
            matches!(
                arg.as_str(),
                "-d" | "--delete" | "-a" | "--annotate" | "-s" | "--sign" | "-f" | "--force"
            )
        }),
        _ => false,
    }
}

fn is_read_only_bash_command(command: &str, args: &[String]) -> bool {
    match command {
        "pwd" | "whoami" | "id" | "groups" | "uname" | "date" | "hostname" | "ls" | "dir"
        | "cat" | "head" | "tail" | "wc" | "grep" | "egrep" | "fgrep" | "rg" | "fd" | "fdfind"
        | "du" | "df" | "stat" | "file" | "which" | "whereis" | "type" | "command" | "hash"
        | "echo" | "printf" | "sort" | "uniq" | "realpath" | "readlink" => true,
        "find" => !args.iter().any(|arg| {
            matches!(
                arg.as_str(),
                "-delete"
                    | "-exec"
                    | "-execdir"
                    | "-ok"
                    | "-okdir"
                    | "-fprint"
                    | "-fprint0"
                    | "-fls"
                    | "-fprintf"
            )
        }),
        "sed" => !args
            .iter()
            .any(|arg| arg == "-i" || arg.starts_with("-i") || arg == "--in-place"),
        _ => false,
    }
}

fn is_read_only_powershell_command(command: &str, _args: &[String]) -> bool {
    matches!(
        command,
        "get-childitem"
            | "gci"
            | "dir"
            | "ls"
            | "get-content"
            | "gc"
            | "cat"
            | "type"
            | "select-string"
            | "sls"
            | "get-location"
            | "pwd"
            | "get-command"
            | "get-process"
            | "gps"
            | "ps"
            | "get-service"
            | "test-path"
            | "get-item"
            | "gi"
            | "get-date"
            | "measure-object"
            | "compare-object"
            | "where-object"
            | "sort-object"
            | "format-table"
            | "format-list"
            | "whoami"
            | "findstr"
            | "rg"
            | "grep"
    )
}

fn is_read_only_cmd_command(command: &str, _args: &[String]) -> bool {
    matches!(
        command,
        "dir"
            | "type"
            | "findstr"
            | "find"
            | "where"
            | "whoami"
            | "ver"
            | "cd"
            | "chdir"
            | "echo"
            | "set"
            | "time"
            | "date"
    )
}

fn tokenize_command(segment: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    for ch in segment.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' if !in_single_quote => {
                escaped = true;
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            c if c.is_whitespace() && !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn split_command_segments(command: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;
    let mut chars = command.chars().peekable();

    while let Some(ch) = chars.next() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' && !in_single_quote {
            current.push(ch);
            escaped = true;
            continue;
        }

        if ch == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            current.push(ch);
            continue;
        }

        if ch == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            current.push(ch);
            continue;
        }

        if !in_single_quote && !in_double_quote {
            if matches!(ch, ';' | '|' | '\n') {
                push_segment(&mut segments, &mut current);
                if ch == '|' && matches!(chars.peek(), Some('|')) {
                    chars.next();
                }
                continue;
            }

            if ch == '&' {
                push_segment(&mut segments, &mut current);
                if matches!(chars.peek(), Some('&')) {
                    chars.next();
                }
                continue;
            }
        }

        current.push(ch);
    }

    push_segment(&mut segments, &mut current);
    segments
}

fn push_segment(segments: &mut Vec<String>, current: &mut String) {
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        segments.push(trimmed.to_string());
    }
    current.clear();
}

fn contains_unquoted_output_redirection(command: &str) -> bool {
    scan_unquoted(command, |ch, _next| ch == '>')
}

fn contains_unquoted_command_substitution(command: &str) -> bool {
    scan_unquoted(command, |ch, next| {
        ch == '`' || (ch == '$' && next == Some('('))
    })
}

fn has_trailing_unquoted_background(command: &str) -> bool {
    let mut last_unquoted = None;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    for ch in command.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && !in_single_quote {
            escaped = true;
            continue;
        }
        if ch == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            continue;
        }
        if ch == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            continue;
        }
        if !in_single_quote && !in_double_quote && !ch.is_whitespace() {
            last_unquoted = Some(ch);
        }
    }

    last_unquoted == Some('&')
}

fn scan_unquoted<F>(command: &str, mut predicate: F) -> bool
where
    F: FnMut(char, Option<char>) -> bool,
{
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;
    let chars: Vec<char> = command.chars().collect();

    for (index, ch) in chars.iter().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if *ch == '\\' && !in_single_quote {
            escaped = true;
            continue;
        }
        if *ch == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            continue;
        }
        if *ch == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            continue;
        }
        if !in_single_quote && !in_double_quote && predicate(*ch, chars.get(index + 1).copied()) {
            return true;
        }
    }

    false
}

// ============================================================================
// 规则动作
// ============================================================================

/// 规则动作
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleAction {
    /// 永久拒绝
    Deny,
    /// 需要用户确认
    Confirm,
    /// 允许
    Allow,
}

impl std::fmt::Display for RuleAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleAction::Deny => write!(f, "deny"),
            RuleAction::Confirm => write!(f, "confirm"),
            RuleAction::Allow => write!(f, "allow"),
        }
    }
}

// ============================================================================
// 命令规则
// ============================================================================

/// 命令规则
#[derive(Debug, Clone)]
pub struct CommandRule {
    /// 正则表达式模式
    pub pattern: String,
    /// 规则动作
    pub action: RuleAction,
    /// 描述
    pub description: String,
}

/// 编译后的命令规则（内部使用）
#[derive(Debug)]
struct CompiledRule {
    /// 编译后的正则表达式
    regex: Regex,
    /// 规则动作
    action: RuleAction,
    /// 描述
    description: String,
    /// 原始模式（用于调试）
    pattern: String,
}

// ============================================================================
// CommandRules
// ============================================================================

/// 命令规则引擎
///
/// 管理命令规则列表，提供命令校验能力。
#[derive(Debug)]
pub struct CommandRules {
    /// 编译后的规则列表
    rules: Vec<CompiledRule>,
}

impl CommandRules {
    /// 创建新的命令规则引擎
    pub fn new() -> Self {
        tracing::debug!("Creating new CommandRules");
        Self { rules: Vec::new() }
    }

    /// 从规则定义列表创建命令规则引擎
    ///
    /// # Arguments
    /// * `rules` - 命令规则定义列表
    ///
    /// # Returns
    /// 命令规则引擎（如果所有正则表达式都合法）
    pub fn from_rules(rules: &[CommandRule]) -> Result<Self, String> {
        let mut engine = Self::new();
        for rule in rules {
            engine.add_rule(rule.clone())?;
        }
        Ok(engine)
    }

    /// 添加命令规则
    ///
    /// # Arguments
    /// * `rule` - 命令规则
    ///
    /// # Returns
    /// 如果正则表达式无效返回错误
    pub fn add_rule(&mut self, rule: CommandRule) -> Result<(), String> {
        let regex = Regex::new(&rule.pattern)
            .map_err(|e| format!("无效的正则表达式 '{}': {}", rule.pattern, e))?;

        tracing::debug!(
            pattern = %rule.pattern,
            action = %rule.action,
            description = %rule.description,
            "Adding command rule"
        );

        self.rules.push(CompiledRule {
            regex,
            action: rule.action,
            description: rule.description,
            pattern: rule.pattern,
        });

        Ok(())
    }

    /// 校验命令
    ///
    /// 按规则列表顺序匹配，返回第一个匹配规则的结果。
    ///
    /// # Arguments
    /// * `command` - 待校验的命令
    ///
    /// # Returns
    /// 校验结果
    pub fn check(&self, command: &str) -> CheckResult {
        self.check_with_shell(command, None)
    }

    /// 使用指定 shell 语义校验命令。
    ///
    /// 当调用方知道命令实际会由哪个 shell 执行时，必须传入 shell，
    /// 避免把其他 shell 的只读语义错误套用到当前执行路径。
    pub fn check_for_shell(&self, command: &str, shell: CommandShell) -> CheckResult {
        self.check_with_shell(command, Some(shell))
    }

    fn check_with_shell(&self, command: &str, shell: Option<CommandShell>) -> CheckResult {
        tracing::debug!(command = %command, "Checking command against rules");

        let mut first_non_deny_match: Option<&CompiledRule> = None;

        for rule in &self.rules {
            if rule.regex.is_match(command) {
                tracing::debug!(
                    command = %command,
                    pattern = %rule.pattern,
                    action = %rule.action,
                    description = %rule.description,
                    "Command matched rule"
                );

                if matches!(rule.action, RuleAction::Deny) {
                    return CheckResult::denied(
                        PermissionLevel::UserConfirm,
                        format!("命令被禁止: {} ({})", command, rule.description),
                    );
                }

                first_non_deny_match = Some(rule);
                break;
            }
        }

        if let Some(warning) = dangerous_command_warning(command) {
            tracing::warn!(
                command = %command,
                category = %warning.category,
                message = %warning.message,
                "Command matched dangerous warning"
            );
            return CheckResult::needs_confirm(
                PermissionLevel::UserConfirm,
                format!(
                    "命令需要确认: {} (warning:{}: {})",
                    command, warning.category, warning.message
                ),
            );
        }

        if let Some(rule) = first_non_deny_match {
            return match &rule.action {
                RuleAction::Deny => unreachable!("deny rules return immediately"),
                RuleAction::Confirm => CheckResult::needs_confirm(
                    PermissionLevel::UserConfirm,
                    format!("命令需要确认: {} ({})", command, rule.description),
                ),
                RuleAction::Allow => CheckResult::allowed(PermissionLevel::Unrestricted),
            };
        }

        let read_only = match shell {
            Some(shell) => is_read_only_command(command, shell),
            None => is_read_only_command_any_shell(command),
        };

        if read_only {
            tracing::debug!(command = %command, "Command allowed by read-only semantics");
            return CheckResult::allowed(PermissionLevel::Unrestricted);
        }

        // 无匹配规则，默认需要确认
        tracing::debug!(command = %command, "No matching rule, defaulting to confirm");
        CheckResult::needs_confirm(
            PermissionLevel::UserConfirm,
            format!("命令无匹配规则，需要确认: {}", command),
        )
    }

    /// 获取所有规则定义
    pub fn rules(&self) -> Vec<CommandRule> {
        self.rules
            .iter()
            .map(|r| CommandRule {
                pattern: r.pattern.clone(),
                action: r.action.clone(),
                description: r.description.clone(),
            })
            .collect()
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

impl Default for CommandRules {
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

    fn create_default_rules() -> CommandRules {
        let rules = vec![
            CommandRule {
                pattern: r"^rm\s+-rf\s+/$".to_string(),
                action: RuleAction::Deny,
                description: "禁止删除根目录".to_string(),
            },
            CommandRule {
                pattern: r"^sudo\s+rm".to_string(),
                action: RuleAction::Deny,
                description: "禁止 sudo 删除".to_string(),
            },
            CommandRule {
                pattern: r":\(\)\{.*:\|.*\}".to_string(),
                action: RuleAction::Deny,
                description: "禁止 fork bomb".to_string(),
            },
            CommandRule {
                pattern: r"^sudo\s+".to_string(),
                action: RuleAction::Confirm,
                description: "sudo 命令需要确认".to_string(),
            },
            CommandRule {
                pattern: r"^git\s+push".to_string(),
                action: RuleAction::Confirm,
                description: "Git 推送需要确认".to_string(),
            },
            CommandRule {
                pattern: r"^git\s+reset\s+--hard".to_string(),
                action: RuleAction::Confirm,
                description: "Git 硬重置需要确认".to_string(),
            },
            CommandRule {
                pattern: r"rm\s+.*-r".to_string(),
                action: RuleAction::Confirm,
                description: "递归删除需要确认".to_string(),
            },
            CommandRule {
                pattern: r"^git\s+(status|diff|log|branch)".to_string(),
                action: RuleAction::Allow,
                description: "Git 只读操作".to_string(),
            },
            CommandRule {
                pattern: r"^(npm|cargo|pnpm)\s+(test|run|build)".to_string(),
                action: RuleAction::Allow,
                description: "包管理器常用命令".to_string(),
            },
        ];

        CommandRules::from_rules(&rules).unwrap()
    }

    #[test]
    fn test_command_rules_new() {
        let cr = CommandRules::new();
        assert_eq!(cr.rule_count(), 0);
    }

    #[test]
    fn test_command_rules_from_rules() {
        let cr = create_default_rules();
        assert_eq!(cr.rule_count(), 9);
    }

    // ======================================================================
    // 终端命令语义测试
    // ======================================================================

    #[test]
    fn test_bash_read_only_commands() {
        assert!(is_read_only_command("ls -la src", CommandShell::Bash));
        assert!(is_read_only_command(
            "git status --short",
            CommandShell::Bash
        ));
        assert!(is_read_only_command("git diff --stat", CommandShell::Bash));
        assert!(is_read_only_command(
            "grep -R needle src",
            CommandShell::Bash
        ));
        assert!(is_read_only_command(
            "find . -name '*.rs'",
            CommandShell::Bash
        ));
        assert!(is_read_only_command(
            "cat Cargo.toml | wc -l",
            CommandShell::Bash
        ));
    }

    #[test]
    fn test_bash_rejects_mutating_or_ambiguous_commands() {
        assert!(!is_read_only_command("rm -rf target", CommandShell::Bash));
        assert!(!is_read_only_command(
            "sed -i 's/a/b/' file",
            CommandShell::Bash
        ));
        assert!(!is_read_only_command("find . -delete", CommandShell::Bash));
        assert!(!is_read_only_command(
            "echo hello > file.txt",
            CommandShell::Bash
        ));
        assert!(!is_read_only_command(
            "echo $(rm -rf target)",
            CommandShell::Bash
        ));
        assert!(!is_read_only_command(
            "git branch -D old",
            CommandShell::Bash
        ));
    }

    #[test]
    fn test_powershell_read_only_commands() {
        assert!(is_read_only_command(
            "Get-ChildItem -Recurse",
            CommandShell::PowerShell
        ));
        assert!(is_read_only_command(
            "Get-Content .\\Cargo.toml",
            CommandShell::PowerShell
        ));
        assert!(is_read_only_command(
            "Select-String -Path *.rs -Pattern test",
            CommandShell::PowerShell
        ));
        assert!(is_read_only_command(
            "Test-Path .\\src",
            CommandShell::PowerShell
        ));
        assert!(is_read_only_command(
            "& \"findstr.exe\" /s /i needle *.rs",
            CommandShell::PowerShell
        ));
    }

    #[test]
    fn test_powershell_rejects_mutating_commands() {
        assert!(!is_read_only_command(
            "Remove-Item -Recurse -Force target",
            CommandShell::PowerShell
        ));
        assert!(!is_read_only_command(
            "Set-Content file.txt value",
            CommandShell::PowerShell
        ));
        assert!(!is_read_only_command(
            "Get-Content a.txt | Set-Content b.txt",
            CommandShell::PowerShell
        ));
        assert!(!is_read_only_command(
            "Get-Content a.txt > b.txt",
            CommandShell::PowerShell
        ));
    }

    #[test]
    fn test_cmd_read_only_commands() {
        assert!(is_read_only_command("dir /s", CommandShell::Cmd));
        assert!(is_read_only_command("type Cargo.toml", CommandShell::Cmd));
        assert!(is_read_only_command(
            "findstr /s /i needle *.rs",
            CommandShell::Cmd
        ));
        assert!(is_read_only_command("where git", CommandShell::Cmd));
        assert!(is_read_only_command(
            "git log --oneline -5",
            CommandShell::Cmd
        ));
    }

    #[test]
    fn test_cmd_rejects_mutating_commands() {
        assert!(!is_read_only_command("del /s /q target", CommandShell::Cmd));
        assert!(!is_read_only_command("copy a.txt b.txt", CommandShell::Cmd));
        assert!(!is_read_only_command(
            "robocopy src dst /mir",
            CommandShell::Cmd
        ));
        assert!(!is_read_only_command("dir > out.txt", CommandShell::Cmd));
    }

    #[test]
    fn test_dangerous_warning_categories() {
        assert_eq!(
            dangerous_command_warning("git reset --hard HEAD")
                .unwrap()
                .category,
            DangerousCommandCategory::DataLoss
        );
        assert_eq!(
            dangerous_command_warning("git push --force origin main")
                .unwrap()
                .category,
            DangerousCommandCategory::RemoteHistoryRewrite
        );
        assert_eq!(
            dangerous_command_warning("rm -rf target").unwrap().category,
            DangerousCommandCategory::FileDeletion
        );
        assert_eq!(
            dangerous_command_warning("DROP TABLE users")
                .unwrap()
                .category,
            DangerousCommandCategory::DatabaseMutation
        );
        assert_eq!(
            dangerous_command_warning("kubectl delete pod navis")
                .unwrap()
                .category,
            DangerousCommandCategory::InfrastructureMutation
        );
    }

    #[test]
    fn test_command_rules_allow_semantic_read_only_without_config_rule() {
        let cr = CommandRules::new();
        let result = cr.check("Get-ChildItem -Recurse");
        assert!(result.allowed);
        assert!(!result.require_confirm);
        assert_eq!(result.level, PermissionLevel::Unrestricted);
    }

    #[test]
    fn test_command_rules_use_actual_shell_for_semantic_read_only() {
        let cr = CommandRules::new();

        let powershell = cr.check_for_shell("Get-ChildItem -Recurse", CommandShell::PowerShell);
        assert!(powershell.allowed);
        assert!(!powershell.require_confirm);

        let cmd = cr.check_for_shell("Get-ChildItem -Recurse", CommandShell::Cmd);
        assert!(cmd.allowed);
        assert!(cmd.require_confirm);
    }

    #[test]
    fn test_command_rules_dangerous_warning_overrides_broad_allow_rule() {
        let cr = create_default_rules();
        let result = cr.check("git branch -D old");
        assert!(result.allowed);
        assert!(result.require_confirm);
        assert!(result
            .confirm_message
            .as_ref()
            .unwrap()
            .contains("warning:file_deletion"));
    }

    #[test]
    fn test_command_rules_invalid_regex() {
        let rules = vec![CommandRule {
            pattern: r"[invalid".to_string(),
            action: RuleAction::Deny,
            description: "无效规则".to_string(),
        }];

        let result = CommandRules::from_rules(&rules);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("无效的正则表达式"));
    }

    // ======================================================================
    // 黑名单测试
    // ======================================================================

    #[test]
    fn test_deny_rm_rf_root() {
        let cr = create_default_rules();
        let result = cr.check("rm -rf /");
        assert!(!result.allowed);
        assert!(result.reason.as_ref().unwrap().contains("禁止删除根目录"));
    }

    #[test]
    fn test_deny_sudo_rm() {
        let cr = create_default_rules();
        let result = cr.check("sudo rm -rf /home/user");
        assert!(!result.allowed);
        assert!(result.reason.as_ref().unwrap().contains("禁止 sudo 删除"));
    }

    #[test]
    fn test_deny_fork_bomb() {
        let cr = create_default_rules();
        let result = cr.check(":(){ :|:& };:");
        assert!(!result.allowed);
        assert!(result.reason.as_ref().unwrap().contains("禁止 fork bomb"));
    }

    // ======================================================================
    // 需确认规则测试
    // ======================================================================

    #[test]
    fn test_confirm_sudo() {
        let cr = create_default_rules();
        let result = cr.check("sudo apt install vim");
        assert!(result.allowed);
        assert!(result.require_confirm);
        assert!(result
            .confirm_message
            .as_ref()
            .unwrap()
            .contains("sudo 命令需要确认"));
    }

    #[test]
    fn test_confirm_git_push() {
        let cr = create_default_rules();
        let result = cr.check("git push origin main");
        assert!(result.allowed);
        assert!(result.require_confirm);
        assert!(result
            .confirm_message
            .as_ref()
            .unwrap()
            .contains("Git 推送需要确认"));
    }

    #[test]
    fn test_confirm_git_reset_hard() {
        let cr = create_default_rules();
        let result = cr.check("git reset --hard HEAD~1");
        assert!(result.allowed);
        assert!(result.require_confirm);
        assert!(result
            .confirm_message
            .as_ref()
            .unwrap()
            .contains("warning:data_loss"));
    }

    #[test]
    fn test_confirm_rm_recursive() {
        let cr = create_default_rules();
        let result = cr.check("rm -r node_modules");
        assert!(result.allowed);
        assert!(result.require_confirm);
        assert!(result
            .confirm_message
            .as_ref()
            .unwrap()
            .contains("递归删除需要确认"));
    }

    // ======================================================================
    // 白名单测试
    // ======================================================================

    #[test]
    fn test_allow_git_status() {
        let cr = create_default_rules();
        let result = cr.check("git status");
        assert!(result.allowed);
        assert!(!result.require_confirm);
        assert_eq!(result.level, PermissionLevel::Unrestricted);
    }

    #[test]
    fn test_allow_git_diff() {
        let cr = create_default_rules();
        let result = cr.check("git diff");
        assert!(result.allowed);
        assert!(!result.require_confirm);
    }

    #[test]
    fn test_allow_git_log() {
        let cr = create_default_rules();
        let result = cr.check("git log --oneline -10");
        assert!(result.allowed);
        assert!(!result.require_confirm);
    }

    #[test]
    fn test_allow_git_branch() {
        let cr = create_default_rules();
        let result = cr.check("git branch -a");
        assert!(result.allowed);
        assert!(!result.require_confirm);
    }

    #[test]
    fn test_allow_cargo_test() {
        let cr = create_default_rules();
        let result = cr.check("cargo test");
        assert!(result.allowed);
        assert!(!result.require_confirm);
    }

    #[test]
    fn test_allow_npm_run() {
        let cr = create_default_rules();
        let result = cr.check("npm run dev");
        assert!(result.allowed);
        assert!(!result.require_confirm);
    }

    #[test]
    fn test_allow_pnpm_build() {
        let cr = create_default_rules();
        let result = cr.check("pnpm build");
        assert!(result.allowed);
        assert!(!result.require_confirm);
    }

    // ======================================================================
    // 默认行为测试
    // ======================================================================

    #[test]
    fn test_no_matching_rule_default_confirm() {
        let cr = create_default_rules();

        // 没有匹配的规则，默认需要确认
        let result = cr.check("unknown-command --some-flag");
        assert!(result.allowed);
        assert!(result.require_confirm);
        assert!(result
            .confirm_message
            .as_ref()
            .unwrap()
            .contains("无匹配规则"));
    }

    // ======================================================================
    // 规则优先级测试
    // ======================================================================

    #[test]
    fn test_deny_rule_priority_over_allow() {
        // sudo rm 应该先命中 sudo rm 的 deny 规则，而不是 sudo 的 confirm 规则
        let cr = create_default_rules();
        let result = cr.check("sudo rm -rf /tmp/test");
        assert!(!result.allowed);
        assert!(result.reason.as_ref().unwrap().contains("禁止 sudo 删除"));
    }

    #[test]
    fn test_rules_order_matters() {
        // 创建自定义规则，验证顺序影响匹配
        let rules = vec![
            CommandRule {
                pattern: r"^test".to_string(),
                action: RuleAction::Deny,
                description: "拒绝 test 开头".to_string(),
            },
            CommandRule {
                pattern: r"^test".to_string(),
                action: RuleAction::Allow,
                description: "允许 test 开头".to_string(),
            },
        ];

        let cr = CommandRules::from_rules(&rules).unwrap();
        let result = cr.check("test command");

        // 第一条规则（Deny）应该先被匹配
        assert!(!result.allowed);
    }

    // ======================================================================
    // 动态规则管理测试
    // ======================================================================

    #[test]
    fn test_add_rule_dynamically() {
        let mut cr = CommandRules::new();

        cr.add_rule(CommandRule {
            pattern: r"^custom-command".to_string(),
            action: RuleAction::Allow,
            description: "自定义命令".to_string(),
        })
        .unwrap();

        assert_eq!(cr.rule_count(), 1);

        let result = cr.check("custom-command --flag");
        assert!(result.allowed);
        assert!(!result.require_confirm);
    }

    #[test]
    fn test_add_invalid_rule() {
        let mut cr = CommandRules::new();

        let result = cr.add_rule(CommandRule {
            pattern: r"[invalid".to_string(),
            action: RuleAction::Deny,
            description: "无效规则".to_string(),
        });

        assert!(result.is_err());
        assert_eq!(cr.rule_count(), 0);
    }

    #[test]
    fn test_clear_rules() {
        let mut cr = create_default_rules();
        assert_eq!(cr.rule_count(), 9);

        cr.clear();
        assert_eq!(cr.rule_count(), 0);

        // 清除后，默认需要确认
        let result = cr.check("git status");
        assert!(result.allowed);
        assert!(!result.require_confirm);
    }

    #[test]
    fn test_get_rules() {
        let cr = create_default_rules();
        let rules = cr.rules();
        assert_eq!(rules.len(), 9);

        // 验证规则信息完整
        assert_eq!(rules[0].pattern, r"^rm\s+-rf\s+/$");
        assert_eq!(rules[0].action, RuleAction::Deny);
    }

    // ======================================================================
    // RuleAction Display 测试
    // ======================================================================

    #[test]
    fn test_rule_action_display() {
        assert_eq!(RuleAction::Deny.to_string(), "deny");
        assert_eq!(RuleAction::Confirm.to_string(), "confirm");
        assert_eq!(RuleAction::Allow.to_string(), "allow");
    }

    // ======================================================================
    // 边界情况测试
    // ======================================================================

    #[test]
    fn test_empty_command() {
        let cr = create_default_rules();
        let result = cr.check("");
        // 空命令无匹配规则，默认需要确认
        assert!(result.require_confirm);
    }

    #[test]
    fn test_whitespace_command() {
        let cr = create_default_rules();
        let result = cr.check("  git status  ");
        // 新语义层会 trim 后识别只读命令。
        assert!(result.allowed);
        assert!(!result.require_confirm);
    }
}
