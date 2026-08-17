//! 轻量命令加载与管理
//!
//! 管理纯 Markdown 提示词模板（Commands），支持：
//! - 项目级命令（.navis/commands/）
//! - 用户级命令（~/.navis/commands/）
//! - $ARGUMENTS 占位符替换
//! - 安全校验（危险指令检测）

use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 命令来源
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CommandSource {
    /// 项目级（.navis/commands/）
    Project,
    /// 用户级（~/.navis/commands/）
    User,
}

impl std::fmt::Display for CommandSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandSource::Project => write!(f, "project"),
            CommandSource::User => write!(f, "user"),
        }
    }
}

impl CommandSource {
    /// 获取来源显示标签
    pub fn label(&self) -> &str {
        match self {
            CommandSource::Project => "[项目]",
            CommandSource::User => "[用户]",
        }
    }
}

/// 轻量命令模板
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommandTemplate {
    /// 命令名（文件名去 .md）
    pub name: String,
    /// .md 文件路径
    pub file_path: PathBuf,
    /// 来源
    pub source: CommandSource,
    /// 原始 Markdown 内容（即提示词模板）
    pub content: String,
    /// 是否包含 $ARGUMENTS 占位符
    pub has_arguments: bool,
    /// 是否需要审核（安全校验未通过）
    pub needs_review: bool,
    /// 是否启用
    pub enabled: bool,
}

impl CommandTemplate {
    /// 获取描述（取内容首行非空文本）
    pub fn description(&self) -> String {
        self.content
            .lines()
            .find(|line| !line.trim().is_empty())
            .map(|line| {
                let trimmed = line.trim();
                if trimmed.len() > 80 {
                    format!("{}...", &trimmed[..80])
                } else {
                    trimmed.to_string()
                }
            })
            .unwrap_or_default()
    }

    /// 替换 $ARGUMENTS 占位符
    pub fn render(&self, arguments: &str) -> String {
        self.content.replace("$ARGUMENTS", arguments)
    }
}

/// 危险指令模式
const DANGEROUS_PATTERNS: &[&str] = &[
    r"rm\s+-rf\s+/",
    r"rm\s+-rf\s+~",
    r"format\s+[a-zA-Z]:",
    r"mkfs\.",
    r"dd\s+if=.*of=/dev/",
    r"eval\s*\(",
    r"exec\s*\(",
    r">\s*/dev/sd",
    r"chmod\s+777\s+/",
    r"shutdown\s",
    r"reboot\s",
];

/// 命令管理器
pub struct CommandManager {
    /// 命令存储（name -> CommandTemplate）
    commands: HashMap<String, CommandTemplate>,
    /// 危险模式正则
    dangerous_regexes: Vec<regex::Regex>,
}

impl CommandManager {
    /// 创建新的命令管理器
    pub fn new() -> Self {
        let dangerous_regexes: Vec<regex::Regex> = DANGEROUS_PATTERNS
            .iter()
            .filter_map(|pattern| regex::Regex::new(pattern).ok())
            .collect();

        Self {
            commands: HashMap::new(),
            dangerous_regexes,
        }
    }

    /// 获取项目级命令目录（.navis/commands/）
    pub fn project_commands_dir(&self) -> Option<PathBuf> {
        std::env::current_dir()
            .ok()
            .map(|cwd| cwd.join(".navis").join("commands"))
    }

    /// 获取用户级命令目录（~/.navis/commands/）
    pub fn user_commands_dir(&self) -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".navis").join("commands"))
    }

    /// 从目录加载命令
    pub fn load_from_dir(&mut self, dir: &Path, source: CommandSource) -> Result<()> {
        if !dir.exists() {
            tracing::debug!(dir = %dir.display(), "Commands directory not found, skipping");
            return Ok(());
        }

        let entries = std::fs::read_dir(dir).map_err(|e| {
            anyhow::anyhow!(
                "Failed to read commands directory '{}': {}",
                dir.display(),
                e
            )
        })?;

        let mut count = 0;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            match path.extension().and_then(|ext| ext.to_str()) {
                Some("md") => {}
                _ => continue,
            }

            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            match self.load_command(&path, &name, source.clone()) {
                Ok(cmd) => {
                    tracing::debug!(
                        name = %cmd.name,
                        source = %cmd.source,
                        has_arguments = cmd.has_arguments,
                        needs_review = cmd.needs_review,
                        "Command loaded"
                    );
                    self.commands.insert(cmd.name.clone(), cmd);
                    count += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "Failed to load command"
                    );
                }
            }
        }

        tracing::info!(
            dir = %dir.display(),
            source = %source,
            count = count,
            "Commands loaded from directory"
        );

        Ok(())
    }

    /// 加载单个命令
    fn load_command(
        &self,
        path: &Path,
        name: &str,
        source: CommandSource,
    ) -> Result<CommandTemplate> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read '{}': {}", path.display(), e))?;

        let has_arguments = content.contains("$ARGUMENTS");
        let needs_review = self.scan_dangerous_content(&content);

        if needs_review {
            tracing::warn!(
                name = %name,
                path = %path.display(),
                "Command contains potentially dangerous content, marked for review"
            );
        }

        Ok(CommandTemplate {
            name: name.to_string(),
            file_path: path.to_path_buf(),
            source,
            content,
            has_arguments,
            needs_review,
            enabled: true,
        })
    }

    /// 扫描危险内容
    fn scan_dangerous_content(&self, content: &str) -> bool {
        for regex in &self.dangerous_regexes {
            if regex.is_match(content) {
                return true;
            }
        }
        false
    }

    /// 获取命令
    pub fn get(&self, name: &str) -> Option<&CommandTemplate> {
        self.commands.get(name)
    }

    /// 根据触发路径查找命令
    pub fn get_by_trigger(&self, trigger: &str) -> Option<&CommandTemplate> {
        // trigger 格式: /command-name
        let name = trigger.strip_prefix('/').unwrap_or(trigger);
        self.commands
            .get(name)
            .filter(|cmd| cmd.enabled && !cmd.needs_review)
    }

    /// 列出所有命令
    pub fn list(&self) -> Vec<&CommandTemplate> {
        self.commands.values().collect()
    }

    /// 列出可用命令（已启用且无需审核）
    pub fn list_available(&self) -> Vec<&CommandTemplate> {
        self.commands
            .values()
            .filter(|cmd| cmd.enabled && !cmd.needs_review)
            .collect()
    }

    /// 启用命令
    pub fn enable(&mut self, name: &str) -> Result<()> {
        let cmd = self
            .commands
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Command not found: {}", name))?;
        cmd.enabled = true;
        tracing::info!(name = %name, "Command enabled");
        Ok(())
    }

    /// 禁用命令
    pub fn disable(&mut self, name: &str) -> Result<()> {
        let cmd = self
            .commands
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Command not found: {}", name))?;
        cmd.enabled = false;
        tracing::info!(name = %name, "Command disabled");
        Ok(())
    }

    /// 创建命令
    pub fn create(
        &mut self,
        name: &str,
        content: &str,
        dir: &Path,
        source: CommandSource,
    ) -> Result<()> {
        let file_path = dir.join(format!("{}.md", name));

        // 写入文件
        std::fs::create_dir_all(dir).map_err(|e| {
            anyhow::anyhow!("Failed to create directory '{}': {}", dir.display(), e)
        })?;
        std::fs::write(&file_path, content).map_err(|e| {
            anyhow::anyhow!("Failed to write file '{}': {}", file_path.display(), e)
        })?;

        let has_arguments = content.contains("$ARGUMENTS");
        let needs_review = self.scan_dangerous_content(content);

        let cmd = CommandTemplate {
            name: name.to_string(),
            file_path,
            source: source.clone(),
            content: content.to_string(),
            has_arguments,
            needs_review,
            enabled: true,
        };

        tracing::info!(name = %name, source = %source, "Command created");
        self.commands.insert(name.to_string(), cmd);
        Ok(())
    }

    /// 删除命令
    pub fn delete(&mut self, name: &str) -> Result<()> {
        if let Some(cmd) = self.commands.remove(name) {
            // 删除文件
            if cmd.file_path.exists() {
                std::fs::remove_file(&cmd.file_path)
                    .map_err(|e| anyhow::anyhow!("Failed to delete file: {}", e))?;
            }
            tracing::info!(name = %name, "Command deleted");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Command not found: {}", name))
        }
    }

    /// 获取命令数量
    pub fn count(&self) -> usize {
        self.commands.len()
    }

    /// 渲染命令（替换 $ARGUMENTS）
    pub fn render_command(&self, name: &str, arguments: &str) -> Result<String> {
        let cmd = self
            .commands
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Command not found: {}", name))?;

        if cmd.needs_review {
            return Err(anyhow::anyhow!(
                "Command '{}' needs review and cannot be executed",
                name
            ));
        }

        if !cmd.enabled {
            return Err(anyhow::anyhow!("Command '{}' is disabled", name));
        }

        Ok(cmd.render(arguments))
    }
}

impl Default for CommandManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_commands_from_dir() {
        let tmp_dir = TempDir::new().unwrap();
        let cmd_dir = tmp_dir.path();

        fs::write(
            cmd_dir.join("explain.md"),
            "请解释以下内容：\n\n$ARGUMENTS\n\n要求：通俗易懂",
        )
        .unwrap();

        fs::write(cmd_dir.join("test.md"), "运行测试并分析结果").unwrap();

        // 非 .md 文件应被忽略
        fs::write(cmd_dir.join("readme.txt"), "not a command").unwrap();

        let mut manager = CommandManager::new();
        manager
            .load_from_dir(cmd_dir, CommandSource::Project)
            .unwrap();

        assert_eq!(manager.count(), 2);
    }

    #[test]
    fn test_command_has_arguments() {
        let tmp_dir = TempDir::new().unwrap();
        fs::write(tmp_dir.path().join("explain.md"), "解释 $ARGUMENTS").unwrap();

        let mut manager = CommandManager::new();
        manager
            .load_from_dir(tmp_dir.path(), CommandSource::Project)
            .unwrap();

        let cmd = manager.get("explain").unwrap();
        assert!(cmd.has_arguments);

        let cmd2 = manager.get("nonexistent");
        assert!(cmd2.is_none());
    }

    #[test]
    fn test_command_render() {
        let tmp_dir = TempDir::new().unwrap();
        fs::write(
            tmp_dir.path().join("explain.md"),
            "请解释以下内容：\n\n$ARGUMENTS\n\n要求：通俗易懂",
        )
        .unwrap();

        let mut manager = CommandManager::new();
        manager
            .load_from_dir(tmp_dir.path(), CommandSource::Project)
            .unwrap();

        let result = manager
            .render_command("explain", "async/await 代码")
            .unwrap();
        assert!(result.contains("async/await 代码"));
        assert!(!result.contains("$ARGUMENTS"));
    }

    #[test]
    fn test_render_disabled_command() {
        let tmp_dir = TempDir::new().unwrap();
        fs::write(tmp_dir.path().join("cmd.md"), "内容").unwrap();

        let mut manager = CommandManager::new();
        manager
            .load_from_dir(tmp_dir.path(), CommandSource::User)
            .unwrap();

        manager.disable("cmd").unwrap();
        let result = manager.render_command("cmd", "args");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("disabled"));
    }

    #[test]
    fn test_dangerous_content_detection() {
        let tmp_dir = TempDir::new().unwrap();

        // 危险内容
        fs::write(
            tmp_dir.path().join("dangerous.md"),
            "请执行以下命令：\nrm -rf /",
        )
        .unwrap();

        // 安全内容
        fs::write(tmp_dir.path().join("safe.md"), "请解释代码的工作原理").unwrap();

        let mut manager = CommandManager::new();
        manager
            .load_from_dir(tmp_dir.path(), CommandSource::Project)
            .unwrap();

        let dangerous = manager.get("dangerous").unwrap();
        assert!(dangerous.needs_review);

        let safe = manager.get("safe").unwrap();
        assert!(!safe.needs_review);
    }

    #[test]
    fn test_dangerous_command_cannot_be_rendered() {
        let tmp_dir = TempDir::new().unwrap();
        fs::write(tmp_dir.path().join("bad.md"), "eval(some_code)").unwrap();

        let mut manager = CommandManager::new();
        manager
            .load_from_dir(tmp_dir.path(), CommandSource::Project)
            .unwrap();

        let result = manager.render_command("bad", "");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("needs review"));
    }

    #[test]
    fn test_get_by_trigger() {
        let tmp_dir = TempDir::new().unwrap();
        fs::write(tmp_dir.path().join("explain.md"), "内容").unwrap();

        let mut manager = CommandManager::new();
        manager
            .load_from_dir(tmp_dir.path(), CommandSource::Project)
            .unwrap();

        let cmd = manager.get_by_trigger("/explain");
        assert!(cmd.is_some());
        assert_eq!(cmd.unwrap().name, "explain");

        assert!(manager.get_by_trigger("/nonexistent").is_none());
    }

    #[test]
    fn test_list_available() {
        let tmp_dir = TempDir::new().unwrap();
        fs::write(tmp_dir.path().join("good.md"), "安全内容").unwrap();
        fs::write(tmp_dir.path().join("bad.md"), "rm -rf /").unwrap();

        let mut manager = CommandManager::new();
        manager
            .load_from_dir(tmp_dir.path(), CommandSource::Project)
            .unwrap();

        let available = manager.list_available();
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].name, "good");
    }

    #[test]
    fn test_enable_disable() {
        let tmp_dir = TempDir::new().unwrap();
        fs::write(tmp_dir.path().join("cmd.md"), "内容").unwrap();

        let mut manager = CommandManager::new();
        manager
            .load_from_dir(tmp_dir.path(), CommandSource::User)
            .unwrap();

        let cmd = manager.get("cmd").unwrap();
        assert!(cmd.enabled);

        manager.disable("cmd").unwrap();
        assert!(!manager.get("cmd").unwrap().enabled);

        manager.enable("cmd").unwrap();
        assert!(manager.get("cmd").unwrap().enabled);
    }

    #[test]
    fn test_create_and_delete_command() {
        let tmp_dir = TempDir::new().unwrap();
        let cmd_dir = tmp_dir.path().join("commands");

        let mut manager = CommandManager::new();

        manager
            .create("my-cmd", "提示词内容", &cmd_dir, CommandSource::Project)
            .unwrap();

        assert!(manager.get("my-cmd").is_some());
        assert!(cmd_dir.join("my-cmd.md").exists());

        manager.delete("my-cmd").unwrap();
        assert!(manager.get("my-cmd").is_none());
    }

    #[test]
    fn test_command_description() {
        let tmp_dir = TempDir::new().unwrap();
        fs::write(
            tmp_dir.path().join("desc.md"),
            "\n\n这是第一行有效文本\n第二行",
        )
        .unwrap();

        let mut manager = CommandManager::new();
        manager
            .load_from_dir(tmp_dir.path(), CommandSource::Project)
            .unwrap();

        let cmd = manager.get("desc").unwrap();
        assert_eq!(cmd.description(), "这是第一行有效文本");
    }

    #[test]
    fn test_command_source_display() {
        assert_eq!(format!("{}", CommandSource::Project), "project");
        assert_eq!(format!("{}", CommandSource::User), "user");
    }

    #[test]
    fn test_command_source_label() {
        assert_eq!(CommandSource::Project.label(), "[项目]");
        assert_eq!(CommandSource::User.label(), "[用户]");
    }

    #[test]
    fn test_load_nonexistent_dir() {
        let mut manager = CommandManager::new();
        let result = manager.load_from_dir(Path::new("/nonexistent"), CommandSource::User);
        assert!(result.is_ok());
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_delete_nonexistent_command() {
        let mut manager = CommandManager::new();
        let result = manager.delete("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_enable_nonexistent_command() {
        let mut manager = CommandManager::new();
        let result = manager.enable("nonexistent");
        assert!(result.is_err());
    }
}
