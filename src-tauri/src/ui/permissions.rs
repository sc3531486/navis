use crate::extension::types::MCP;

use super::dto::UiToolPermissionRule;

use crate::security::sandbox::permission::ApprovalMode;

use crate::extension::types::NAVIS_TOOL_SEARCH;



pub(super) const TOOL_PERMISSION_KEYS: [&str; 14] = [

    "read",

    "edit",

    "glob",

    "grep",

    "list",

    "bash",

    "todo",

    "task",

    "skill",

    "lsp",

    "webfetch",

    "websearch",

    "external_directory",

    "browser",

];



pub(super) const RISKY_TOOL_PERMISSION_KEYS: [&str; 8] = [

    "edit",

    "bash",

    "skill",

    "lsp",

    "webfetch",

    "websearch",

    "external_directory",

    "browser",

];



#[derive(Debug, Clone, Copy, PartialEq, Eq)]

pub(super) enum ApprovalPromptAction {

    /// UI may attach approval evidence without showing a prompt.

    Allow,

    /// UI must ask the user before attaching approval evidence.

    Ask,

    /// UI must refuse to attach approval evidence.

    Deny,

}



impl ApprovalPromptAction {

    pub(super) fn from_str(value: &str) -> Option<Self> {

        match value.trim().to_ascii_lowercase().as_str() {

            "allow" => Some(Self::Allow),

            "ask" => Some(Self::Ask),

            "deny" => Some(Self::Deny),

            _ => None,

        }

    }



    fn as_str(self) -> &'static str {

        match self {

            Self::Allow => "allow",

            Self::Ask => "ask",

            Self::Deny => "deny",

        }

    }

}



pub(super) fn normalize_permission_key(value: &str) -> String {

    value.trim().to_ascii_lowercase().replace('-', "_")

}



pub(super) fn normalize_permission_pattern(value: &str) -> String {

    let pattern = value.trim();

    if pattern.is_empty() {

        "*".to_string()

    } else {

        pattern.to_string()

    }

}



fn approval_prompt_value(value: ApprovalPromptAction) -> String {

    value.as_str().to_string()

}



pub(super) fn default_tool_permission_rules() -> Vec<UiToolPermissionRule> {

    TOOL_PERMISSION_KEYS

        .into_iter()

        .map(|permission| {

            let (suggest, auto_edit, full_auto) = match permission {

                "read" | "glob" | "grep" | "list" | "todo" => (

                    ApprovalPromptAction::Allow,

                    ApprovalPromptAction::Allow,

                    ApprovalPromptAction::Allow,

                ),

                "edit" => (

                    ApprovalPromptAction::Ask,

                    ApprovalPromptAction::Allow,

                    ApprovalPromptAction::Allow,

                ),

                "task" => (

                    ApprovalPromptAction::Allow,

                    ApprovalPromptAction::Allow,

                    ApprovalPromptAction::Allow,

                ),

                "external_directory" | "browser" => (

                    ApprovalPromptAction::Ask,

                    ApprovalPromptAction::Ask,

                    ApprovalPromptAction::Ask,

                ),

                _ => (

                    ApprovalPromptAction::Ask,

                    ApprovalPromptAction::Ask,

                    ApprovalPromptAction::Allow,

                ),

            };

            UiToolPermissionRule {

                permission: permission.to_string(),

                pattern: "*".to_string(),

                suggest: approval_prompt_value(suggest),

                auto_edit: approval_prompt_value(auto_edit),

                full_auto: approval_prompt_value(full_auto),

            }

        })

        .collect()

}



pub(super) fn tool_permission_from_names(gateway_tool: &str, mcp_tool: &str) -> String {

    let gateway = normalize_permission_key(gateway_tool);

    let mcp = normalize_permission_key(mcp_tool);

    match gateway.as_str() {

        "tool_search" => "list".to_string(),

        "read" => "read".to_string(),

        "write" | "edit" => "edit".to_string(),

        "glob" => "glob".to_string(),

        "grep" | "search" => "grep".to_string(),

        "list" => "list".to_string(),

        "bash" => "bash".to_string(),

        "todo" => "todo".to_string(),

        "task" => "task".to_string(),

        "skill" => "skill".to_string(),

        "lsp" => "lsp".to_string(),

        "webfetch" | "web_fetch" => "webfetch".to_string(),

        "websearch" | "web_search" => "websearch".to_string(),

        "external_directory" => "external_directory".to_string(),

        "browser" => "browser".to_string(),

        "inspect" => "read".to_string(),

        _ => {

            if mcp == normalize_permission_key(NAVIS_TOOL_SEARCH) || mcp == "tool_search" {

                "list".to_string()

            } else if mcp.starts_with("fs_read") || mcp == "fs_file_info" {

                "read".to_string()

            } else if mcp.starts_with("fs_write") || mcp.starts_with("fs_replace") {

                "edit".to_string()

            } else if mcp.starts_with("fs_search") {

                "grep".to_string()

            } else if mcp.starts_with("fs_list") {

                "list".to_string()

            } else if mcp.starts_with("terminal") || mcp.starts_with("shell") {

                "bash".to_string()

            } else if mcp.starts_with("browser") || gateway.contains("browser") {

                "browser".to_string()

            } else if mcp.contains("webfetch")

                || mcp.contains("web_fetch")

                || gateway.contains("fetch")

            {

                "webfetch".to_string()

            } else if mcp.contains("websearch")

                || mcp.contains("web_search")

                || gateway.contains("websearch")

            {

                "websearch".to_string()

            } else if mcp.starts_with("lsp") || gateway.starts_with("lsp") {

                "lsp".to_string()

            } else if mcp.starts_with("skill") || gateway.starts_with("skill") {

                "skill".to_string()

            } else if mcp == "navis.todo" || mcp == "todo" || gateway.starts_with("todo") {

                "todo".to_string()

            } else if mcp.starts_with("task") || gateway.starts_with("task") {

                "task".to_string()

            } else if mcp.contains("external_directory") || gateway.contains("external_directory") {

                "external_directory".to_string()

            } else {

                gateway

            }

        }

    }

}



pub(super) fn is_risky_tool_permission(permission: &str) -> bool {

    let permission = normalize_permission_key(permission);

    !TOOL_PERMISSION_KEYS.contains(&permission.as_str())

        || RISKY_TOOL_PERMISSION_KEYS.contains(&permission.as_str())

}



fn command_tokens(command: &str) -> Vec<String> {

    command

        .split_whitespace()

        .map(|token| {

            token

                .trim_matches(|c: char| matches!(c, '"' | '\'' | ';' | '&' | '|'))

                .to_ascii_lowercase()

        })

        .filter(|token| !token.is_empty())

        .collect()

}



fn command_has_flag(tokens: &[String], flag: &str) -> bool {

    tokens.iter().any(|token| token == flag)

}



fn command_has_any_flag(tokens: &[String], flags: &[&str]) -> bool {

    flags.iter().any(|flag| command_has_flag(tokens, flag))

}



fn is_hardline_blocked_bash_command(command: &str) -> bool {

    let trimmed = command.trim().to_ascii_lowercase();

    if trimmed.contains(":(){") || trimmed.contains(":|:") {

        return true;

    }



    let tokens = command_tokens(&trimmed);

    if tokens.is_empty() {

        return false;

    }



    let first = tokens.first().map(String::as_str);

    let second = tokens.get(1).map(String::as_str);

    let third = tokens.get(2).map(String::as_str);



    matches!((first, second), (Some("sudo"), Some("rm")))

        || matches!((first, second), (Some("rm"), Some(flag)) if matches!(flag, "-rf" | "-fr") && tokens.iter().any(|token| token == "/" || token == "/*"))

        || matches!(first, Some("del")) && command_has_any_flag(&tokens, &["/s", "/q"])

        || matches!(first, Some("remove-item"))

            && command_has_any_flag(&tokens, &["-recurse", "-force"])

        || matches!((first, second), (Some("git"), Some("clean")))

            && command_has_any_flag(&tokens, &["-f", "-fd", "-df"])

        || matches!((first, second), (Some("kubectl"), Some("delete")))

        || matches!((first, second), (Some("terraform"), Some("destroy")))

        || matches!(

            (first, second),

            (Some("docker"), Some("down" | "stop" | "restart" | "kill"))

        )

        || matches!(

            (first, second, third),

            (Some("docker"), Some("compose"), Some("down"))

        )

}



pub(super) fn hardline_block_reason(permission: &str, pattern: &str) -> Option<String> {

    let permission = normalize_permission_key(permission);

    let pattern = normalize_permission_pattern(pattern);



    if !TOOL_PERMISSION_KEYS.contains(&permission.as_str()) {

        return Some(format!(

            "Unknown tool permission '{}' is blocked by the hardline policy",

            permission

        ));

    }



    if permission == "bash" && is_hardline_blocked_bash_command(&pattern) {

        return Some(format!(

            "Command pattern '{}' is blocked by the hardline policy",

            pattern

        ));

    }



    None

}



fn wildcard_matches(pattern: &str, value: &str) -> bool {

    let pattern = normalize_permission_pattern(pattern);

    if pattern == "*" {

        return true;

    }

    let mut remainder = value;

    let anchored_start = !pattern.starts_with('*');

    let anchored_end = !pattern.ends_with('*');

    let parts = pattern.split('*').collect::<Vec<_>>();

    for (index, part) in parts.iter().filter(|part| !part.is_empty()).enumerate() {

        if index == 0 && anchored_start {

            if !remainder.starts_with(part) {

                return false;

            }

            remainder = &remainder[part.len()..];

            continue;

        }

        let Some(position) = remainder.find(part) else {

            return false;

        };

        remainder = &remainder[position + part.len()..];

    }

    if anchored_end {

        if let Some(last) = parts.iter().rev().find(|part| !part.is_empty()) {

            return value.ends_with(last);

        }

    }

    true

}



fn approval_prompt_action_for_policy(

    rule: &UiToolPermissionRule,

    policy: ApprovalMode,

) -> Option<ApprovalPromptAction> {

    match policy {

        ApprovalMode::Suggest => ApprovalPromptAction::from_str(&rule.suggest),

        ApprovalMode::AutoEdit => ApprovalPromptAction::from_str(&rule.auto_edit),

        ApprovalMode::FullAuto => ApprovalPromptAction::from_str(&rule.full_auto),

    }

}



pub(super) fn approval_prompt_action(

    rules: &[UiToolPermissionRule],

    policy: ApprovalMode,

    permission: &str,

    pattern: &str,

) -> Result<ApprovalPromptAction, String> {

    let permission = normalize_permission_key(permission);

    if hardline_block_reason(&permission, pattern).is_some() {

        return Ok(ApprovalPromptAction::Deny);

    }

    let matching_rule = rules

        .iter()

        .find(|rule| rule.permission == permission && wildcard_matches(&rule.pattern, pattern));

    match matching_rule.and_then(|rule| approval_prompt_action_for_policy(rule, policy)) {

        Some(action) => Ok(action),

        None if policy == ApprovalMode::Suggest => Ok(ApprovalPromptAction::Ask),

        None if is_risky_tool_permission(&permission) => Err(format!(

            "Missing permission rule for risky tool '{}' and pattern '{}'",

            permission, pattern

        )),

        None => Ok(ApprovalPromptAction::Allow),

    }

}


