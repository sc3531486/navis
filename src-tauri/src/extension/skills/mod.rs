//! Skills 技能管理模块
//!
//! 基于设计文档 §19 实现，管理 SKILL.md 文件的加载、解析、匹配和激活计划构建。
//! 同时管理轻量命令（Commands），提供纯 Markdown 提示词模板的快速复用能力。
//!
//! # 三层边界
//!
//! ```text
//! MCP      = 原子工具（read / bash）
//! Skills   = 规则模板（提示词 + 参数 + 工具白名单）
//! Commands = 轻量命令（纯提示词模板，无元数据头）
//! Agent    = 流程决策（调用哪些工具、什么顺序）
//! ```
//!
//! # 子模块
//! - `parser` - SKILL.md 解析器（YAML frontmatter + Markdown 正文）
//! - `loader` - Skill 文件加载器
//! - `validator` - Skill 格式校验
//! - `store` - Skill store（领域提示词包索引，不是 Kernel Registry）
//! - `executor` - Skill 激活服务（标准模式，构建激活计划）
//! - `role` - 角色模板管理
//! - `commands` - 轻量命令加载与管理
//! - `installer` - Skill 安装器（URL 下载、内容安装、卸载、更新）
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

pub mod commands;
pub mod executor;
pub mod installer;
pub mod loader;
pub mod parser;
pub mod role;
pub mod store;
pub mod validator;

// 重导出核心类型
pub use commands::{CommandManager, CommandSource, CommandTemplate};
pub use executor::{SkillActivationPlan, SkillActivationService};
pub use installer::{InstallScope, InstalledSkill, SkillInstaller};
pub use loader::SkillLoader;
pub use parser::SkillParser;
pub use role::RoleManager;
pub use store::SkillStore;
pub use validator::{SkillValidator, ValidationResult};

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::extension::models::SkillDefinition as ExtensionSkillDefinition;
use crate::foundation::config::Config;
use crate::kernel::{EventBus, EventEnvelope, KernelContext, KernelScope};
use triomphe::Arc as SharedArc;

// ============================================================================
// 数据模型
// ============================================================================

/// Skill 模式
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkillMode {
    /// 标准模式 - 纯提示词模板
    Standard,
    /// 增强模式 - 支持步骤/工作流
    Enhanced,
}

impl std::fmt::Display for SkillMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillMode::Standard => write!(f, "standard"),
            SkillMode::Enhanced => write!(f, "enhanced"),
        }
    }
}

impl SkillMode {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "standard" => Some(SkillMode::Standard),
            "enhanced" => Some(SkillMode::Enhanced),
            _ => None,
        }
    }
}

/// Skill 来源
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkillSource {
    /// 内置 Skill
    Builtin,
    /// 用户自定义（~/.navis/skills/）
    User,
    /// 项目级（.navis/skills/）
    Project,
    /// 扩展注册
    Extension,
}

impl std::fmt::Display for SkillSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillSource::Builtin => write!(f, "builtin"),
            SkillSource::User => write!(f, "user"),
            SkillSource::Project => write!(f, "project"),
            SkillSource::Extension => write!(f, "extension"),
        }
    }
}

impl SkillSource {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "builtin" => Some(SkillSource::Builtin),
            "user" => Some(SkillSource::User),
            "project" => Some(SkillSource::Project),
            "extension" => Some(SkillSource::Extension),
            _ => None,
        }
    }

    /// 获取来源显示标签
    pub fn label(&self) -> &str {
        match self {
            SkillSource::Builtin => "[内置]",
            SkillSource::User => "[用户]",
            SkillSource::Project => "[项目]",
            SkillSource::Extension => "[扩展]",
        }
    }
}

/// Skill 参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillParameter {
    /// 参数名
    pub name: String,
    /// 参数描述
    pub description: String,
    /// 是否必填
    pub required: bool,
    /// 默认值
    pub default: Option<String>,
    /// 参数类型（string / number / boolean）
    pub param_type: String,
}

/// Skill 定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    /// Skill ID（基于文件名生成）
    pub id: String,
    /// Skill 名称
    pub name: String,
    /// Skill 描述
    pub description: String,
    /// 模式
    pub mode: SkillMode,
    /// 版本
    pub version: String,
    /// 来源
    pub source: SkillSource,
    /// 文件路径
    pub file_path: PathBuf,
    /// 触发命令（如 "/commit"）
    pub trigger: Option<String>,
    /// 工具白名单
    pub tools_whitelist: Vec<String>,
    /// 参数定义
    pub parameters: Vec<SkillParameter>,
    /// 增强模式步骤
    pub steps: Vec<SkillStep>,
    /// 提示词内容
    pub content: String,
    /// 是否启用
    pub enabled: bool,
    /// 来源 URL（用于安装和更新）
    pub source_url: Option<String>,
    /// 安装时间
    pub installed_at: Option<DateTime<Utc>>,
    /// 作者
    pub author: Option<String>,
    /// 标签
    pub tags: Option<Vec<String>>,
    /// 最低 Navis 版本要求
    pub min_navis_version: Option<String>,
}

/// 失败处理策略
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnFailureAction {
    /// 重试（受 max_retries 限制）
    Retry,
    /// 终止整个 Skill 执行
    Fail,
    /// 跳过当前步骤，继续下一步
    Skip,
}

impl OnFailureAction {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "retry" => Some(OnFailureAction::Retry),
            "fail" => Some(OnFailureAction::Fail),
            "skip" => Some(OnFailureAction::Skip),
            _ => None,
        }
    }
}

/// 步骤执行状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    /// 等待执行
    Pending,
    /// 正在执行
    Running,
    /// 执行完成
    Completed,
    /// 执行失败
    Failed,
    /// 被跳过
    Skipped,
}

/// 增强模式步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStep {
    /// 步骤名称
    pub name: String,
    /// 步骤描述
    pub description: String,
    /// 步骤提示词
    pub prompt: String,
    /// 工具列表
    pub tools: Vec<String>,
    /// 依赖的步骤
    pub depends_on: Vec<String>,
    /// 条件表达式
    pub condition: Option<String>,
    /// 失败处理策略
    pub on_failure: OnFailureAction,
    /// 最大重试次数
    pub max_retries: u32,
    /// 步骤超时时间（秒）
    pub timeout_secs: Option<u64>,
}

/// 角色定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDefinition {
    /// 角色 ID
    pub id: String,
    /// 角色名称
    pub name: String,
    /// 角色描述
    pub description: String,
    /// 系统提示词
    pub system_prompt: String,
    /// 角色行为指导
    pub guidance: Option<String>,
    /// 绑定的 Skills
    pub skills: Vec<String>,
    /// 绑定的轻量命令名
    pub commands: Vec<String>,
    /// 模型偏好
    pub model_preference: Option<String>,
    /// Temperature
    pub temperature: Option<f32>,
}

/// 触发候选项（注册到 "/" 触发器的数据格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTriggerCandidate {
    /// Skill/Command 名称
    pub name: String,
    /// 类型
    pub trigger_type: String,
    /// 来源
    pub source: String,
    /// 来源显示标签
    pub source_label: String,
    /// 描述
    pub description: String,
    /// 完整触发路径
    pub trigger: String,
    /// 来源扩展 ID
    pub extension_id: Option<String>,
}

// ============================================================================
// Skills 主入口
// ============================================================================

/// Skills 技能管理器
///
/// 管理 SKILL.md 的加载、解析、执行以及轻量命令
pub struct Skills {
    /// Skill store
    store: SkillStore,
    /// Skill 加载器
    loader: SkillLoader,
    /// Skill 激活服务
    activation_service: SkillActivationService,
    /// 角色管理器
    role_manager: RoleManager,
    /// 命令管理器
    command_manager: CommandManager,
    /// 事件总线
    event_bus: Arc<dyn EventBus>,
    /// 配置
    config: Arc<Mutex<Config>>,
}

impl Skills {
    /// 初始化测试用 Skills 管理器。
    ///
    /// 应用运行时必须通过 `with_event_bus()` 注入统一的 Kernel EventBus。
    #[cfg(test)]
    pub fn init_for_test(config: Arc<Mutex<Config>>) -> Result<Self> {
        tracing::info!("Initializing test Skills manager");

        let event_bus = Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            tokio::runtime::Handle::current(),
        ));
        let store = SkillStore::new();
        let loader = SkillLoader::new();
        let activation_service = SkillActivationService::new();
        let role_manager = RoleManager::new();
        let command_manager = CommandManager::new();

        Ok(Self {
            store,
            loader,
            activation_service,
            role_manager,
            command_manager,
            event_bus,
            config,
        })
    }

    /// 使用外部 EventBus 初始化。
    pub fn with_event_bus(
        config: Arc<Mutex<Config>>,
        event_bus: Arc<dyn EventBus>,
    ) -> Result<Self> {
        tracing::info!("Initializing Skills manager with external EventBus");

        let store = SkillStore::new();
        let loader = SkillLoader::new();
        let activation_service = SkillActivationService::new();
        let role_manager = RoleManager::new();
        let command_manager = CommandManager::new();

        Ok(Self {
            store,
            loader,
            activation_service,
            role_manager,
            command_manager,
            event_bus,
            config,
        })
    }

    /// 加载所有 Skills
    ///
    /// 扫描 builtin、user、project 目录下的 SKILL.md 文件
    pub fn load_all(&mut self) -> Result<()> {
        tracing::info!("Loading all skills");

        // 1. 加载内置 Skills
        self.load_builtin()?;

        // 2. 加载用户级 Skills（~/.navis/skills/）
        if let Some(user_dir) = self.loader.user_skills_dir() {
            if user_dir.exists() {
                self.load_from_dir(&user_dir, SkillSource::User)?;
            }
        }

        // 3. 加载项目级 Skills（.navis/skills/）
        if let Some(project_dir) = self.loader.project_skills_dir() {
            if project_dir.exists() {
                self.load_from_dir(&project_dir, SkillSource::Project)?;
            }
        }

        // 4. 加载轻量命令
        self.load_commands()?;

        tracing::info!(
            skill_count = self.store.count(),
            command_count = self.command_manager.count(),
            "All skills and commands loaded"
        );

        Ok(())
    }

    /// 加载内置 Skills
    fn load_builtin(&mut self) -> Result<()> {
        tracing::debug!("Loading builtin skills");

        let builtin_skills = self.loader.load_builtin();
        for skill in builtin_skills {
            let id = skill.id.clone();

            // 校验 Skill 格式
            let validation = validate_skill_definition(&skill);
            if !validation.valid {
                tracing::warn!(
                    skill_id = %id,
                    errors = ?validation.errors,
                    "Skipping invalid builtin skill"
                );
                continue;
            }
            for warning in &validation.warnings {
                tracing::debug!(skill_id = %id, warning = %warning, "Skill validation warning");
            }

            tracing::debug!(skill_id = %id, name = %skill.name, "Registering builtin skill");
            self.store.upsert(skill)?;
            self.emit_skill_loaded(&id, "builtin");
        }

        Ok(())
    }

    /// 从目录加载 Skills
    fn load_from_dir(&mut self, dir: &std::path::Path, source: SkillSource) -> Result<()> {
        tracing::debug!(dir = %dir.display(), source = %source, "Loading skills from directory");

        let skills = self.loader.load_from_dir(dir, source.clone())?;
        for skill in skills {
            let id = skill.id.clone();
            let name = skill.name.clone();

            // 校验 Skill 格式
            let validation = validate_skill_definition(&skill);
            if !validation.valid {
                tracing::warn!(
                    skill_id = %id,
                    name = %name,
                    source = %source,
                    errors = ?validation.errors,
                    "Skipping invalid skill"
                );
                continue;
            }
            for warning in &validation.warnings {
                tracing::debug!(skill_id = %id, warning = %warning, "Skill validation warning");
            }

            tracing::debug!(skill_id = %id, name = %name, source = %source, "Registering skill");
            self.store.upsert(skill)?;
            self.emit_skill_loaded(&id, &source.to_string());
        }

        Ok(())
    }

    /// 加载轻量命令
    pub fn load_commands(&mut self) -> Result<()> {
        tracing::info!("Loading lightweight commands");

        let mut sources = Vec::new();

        // 项目级命令
        if let Some(project_dir) = self.command_manager.project_commands_dir() {
            if project_dir.exists() {
                self.command_manager
                    .load_from_dir(&project_dir, CommandSource::Project)?;
                sources.push("project".to_string());
            }
        }

        // 用户级命令
        if let Some(user_dir) = self.command_manager.user_commands_dir() {
            if user_dir.exists() {
                self.command_manager
                    .load_from_dir(&user_dir, CommandSource::User)?;
                sources.push("user".to_string());
            }
        }

        let count = self.command_manager.count();
        tracing::info!(count = count, "Commands loaded");

        // 发出 command.loaded 事件
        if let Err(error) = self.event_bus.emit(EventEnvelope::new(
            "command.loaded",
            KernelContext::new("skills", KernelScope::global()),
            Some(SharedArc::new(serde_json::json!({
                "count": count,
                "sources": sources,
            }))),
        )) {
            tracing::warn!(event = "command.loaded", error = %error, "Failed to emit skills event");
        }

        Ok(())
    }

    /// 注册扩展贡献的 Skill
    pub fn register_extension_skill(
        &mut self,
        extension_id: &str,
        skill: &ExtensionSkillDefinition,
    ) -> Result<()> {
        let skill = extension_skill_definition(extension_id, skill)?;

        if self.store.contains(&skill.id) {
            return Err(anyhow::anyhow!(
                "Extension skill id conflict: '{}'",
                skill.id
            ));
        }

        if let Some(trigger) = skill.trigger.as_deref() {
            let has_conflict = self
                .store
                .list_enabled()
                .into_iter()
                .any(|registered| registered.trigger.as_deref() == Some(trigger));
            if has_conflict {
                return Err(anyhow::anyhow!(
                    "Extension skill trigger conflict: '{}'",
                    trigger
                ));
            }
        }

        let validation = validate_skill_definition(&skill);
        if !validation.valid {
            return Err(anyhow::anyhow!(
                "Invalid extension skill '{}': {}",
                skill.id,
                validation.errors.join("; ")
            ));
        }

        for warning in validation.warnings {
            tracing::debug!(skill_id = %skill.id, warning = %warning, "Extension skill validation warning");
        }

        let skill_id = skill.id.clone();
        self.store.upsert(skill)?;
        self.emit_skill_loaded(&skill_id, "extension");
        Ok(())
    }

    /// 注销扩展贡献的 Skills
    pub fn unregister_extension_skills(
        &mut self,
        extension_id: &str,
        skills: &[ExtensionSkillDefinition],
    ) -> Result<()> {
        let mut seen = HashSet::new();
        for skill in skills {
            if !seen.insert(skill.id.clone()) {
                continue;
            }
            self.unregister_extension_skill(extension_id, &skill.id)?;
        }

        Ok(())
    }

    /// 注销单个扩展贡献的 Skill。
    pub fn unregister_extension_skill(&mut self, extension_id: &str, skill_id: &str) -> Result<()> {
        let skill_id = extension_skill_id(extension_id, skill_id);
        if self.store.contains(&skill_id) {
            self.store.remove(&skill_id)?;
            if let Err(error) = self.event_bus.emit(EventEnvelope::new(
                "skill.unloaded",
                KernelContext::new("skills", KernelScope::global()),
                Some(SharedArc::new(serde_json::json!({"skillId": skill_id}))),
            )) {
                tracing::warn!(
                    event = "skill.unloaded",
                    error = %error,
                    "Failed to emit skills event"
                );
            }
        }
        Ok(())
    }

    /// 获取 Skill
    pub fn get(&self, id: &str) -> Option<Arc<SkillDefinition>> {
        self.store.get(id)
    }

    /// 根据触发命令查找 Skill
    pub fn find_by_trigger(&self, trigger: &str) -> Option<Arc<SkillDefinition>> {
        self.store.find_by_trigger(trigger)
    }

    /// 获取 Skill 上下文（替换参数）
    pub fn get_context(&self, skill_id: &str, params: HashMap<String, String>) -> Result<String> {
        let skill = self
            .store
            .get(skill_id)
            .ok_or_else(|| anyhow::anyhow!("Skill not found: {}", skill_id))?;

        let mut content = skill.content.clone();

        // 替换命名参数
        for param in &skill.parameters {
            let placeholder = format!("${{{}}}", param.name);
            let value = params
                .get(&param.name)
                .or(param.default.as_ref())
                .ok_or_else(|| {
                    anyhow::anyhow!("Required parameter '{}' not provided", param.name)
                })?;

            content = content.replace(&placeholder, value);
        }

        Ok(content)
    }

    /// 规划 Skill 激活计划
    ///
    /// 通过 `SkillActivationService::build_activation_plan()` 构建激活计划，供 Agent 使用。
    /// 返回 `SkillActivationPlan`，包含替换参数后的上下文、工具白名单和参数。
    ///
    /// 会检查 `skills.enabled` 配置项，若显式设为 `false` 则拒绝激活。
    pub fn plan_skill_activation(
        &self,
        skill_id: &str,
        params: HashMap<String, String>,
        source: &str,
    ) -> Result<SkillActivationPlan> {
        // 检查 skills 是否在全局配置中被禁用
        let config = self
            .config
            .lock()
            .map_err(|error| anyhow::anyhow!("Failed to lock config: {}", error))?;
        if let Some(enabled) = config.get("skills.enabled") {
            if enabled == serde_json::json!(false) {
                tracing::warn!(
                    skill_id = %skill_id,
                    "Skill activation blocked: skills.enabled is false in config"
                );
                return Err(anyhow::anyhow!(
                    "Skill activation is disabled (skills.enabled=false)"
                ));
            }
        }

        let skill = self
            .store
            .get(skill_id)
            .ok_or_else(|| anyhow::anyhow!("Skill not found: {}", skill_id))?;

        tracing::info!(skill_id = %skill_id, "Planning skill activation");

        let plan = self
            .activation_service
            .build_activation_plan(&skill, params, source)?;

        tracing::info!(
            skill_id = %skill_id,
            source = %plan.source,
            "Skill activation plan ready"
        );

        Ok(plan)
    }

    /// 获取所有角色
    pub fn list_roles(&self) -> Vec<&RoleDefinition> {
        self.role_manager.list()
    }

    /// 获取角色
    pub fn get_role(&self, id: &str) -> Option<&RoleDefinition> {
        self.role_manager.get(id)
    }

    /// 获取所有命令
    pub fn list_commands(&self) -> Vec<&CommandTemplate> {
        self.command_manager.list()
    }

    /// 获取命令
    pub fn get_command(&self, name: &str) -> Option<&CommandTemplate> {
        self.command_manager.get(name)
    }

    /// 根据触发路径查找命令或 Skill（Commands 优先）
    pub fn resolve_trigger(&self, trigger: &str) -> Option<ResolvedTrigger> {
        // 1. 优先匹配命令
        if let Some(cmd) = self.command_manager.get_by_trigger(trigger) {
            return Some(ResolvedTrigger::Command(cmd.name.clone()));
        }

        // 2. 再匹配 Skill
        if let Some(skill) = self.store.find_by_trigger(trigger) {
            return Some(ResolvedTrigger::Skill(skill.id.clone()));
        }

        None
    }

    /// 获取所有触发候选项（用于注册到 "/" 触发器）
    pub fn get_trigger_candidates(&self) -> Vec<SkillTriggerCandidate> {
        let mut candidates = Vec::new();
        let mut occupied_triggers = std::collections::HashSet::new();

        // Commands
        let mut commands = self.command_manager.list_available();
        commands.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        for cmd in commands {
            let trigger = format!("/{}", cmd.name);
            occupied_triggers.insert(trigger.clone());
            candidates.push(SkillTriggerCandidate {
                name: cmd.name.clone(),
                trigger_type: "command".to_string(),
                source: cmd.source.to_string(),
                source_label: cmd.source.label().to_string(),
                description: cmd.description(),
                trigger,
                extension_id: None,
            });
        }

        // Skills
        let mut skills = self.store.list_enabled();
        skills.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        for skill in skills {
            let trigger_type = match skill.mode {
                SkillMode::Standard => "skill",
                SkillMode::Enhanced => "enhanced",
            };
            let trigger = skill
                .trigger
                .clone()
                .unwrap_or_else(|| format!("/{}", skill.name));
            if occupied_triggers.contains(&trigger) {
                tracing::debug!(
                    skill_id = %skill.id,
                    trigger = %trigger,
                    "Skipping skill trigger candidate because a command with the same trigger takes precedence"
                );
                continue;
            }
            candidates.push(SkillTriggerCandidate {
                name: skill.name.clone(),
                trigger_type: trigger_type.to_string(),
                source: skill.source.to_string(),
                source_label: skill.source.label().to_string(),
                description: skill.description.clone(),
                trigger,
                extension_id: extension_id_from_skill(&skill),
            });
        }

        candidates
    }

    /// 启用 Skill
    pub fn enable_skill(&mut self, id: &str) -> Result<()> {
        self.store.set_enabled(id, true)?;
        if let Err(error) = self.event_bus.emit(EventEnvelope::new(
            "skill.enabled",
            KernelContext::new("skills", KernelScope::global()),
            Some(SharedArc::new(serde_json::json!({"skillId": id}))),
        )) {
            tracing::warn!(event = "skill.enabled", error = %error, "Failed to emit skills event");
        }
        Ok(())
    }

    /// 禁用 Skill
    pub fn disable_skill(&mut self, id: &str) -> Result<()> {
        self.store.set_enabled(id, false)?;
        if let Err(error) = self.event_bus.emit(EventEnvelope::new(
            "skill.disabled",
            KernelContext::new("skills", KernelScope::global()),
            Some(SharedArc::new(serde_json::json!({"skillId": id}))),
        )) {
            tracing::warn!(event = "skill.disabled", error = %error, "Failed to emit skills event");
        }
        Ok(())
    }

    /// 卸载 Skill
    pub fn uninstall_skill(&mut self, id: &str) -> Result<()> {
        self.store.remove(id)?;
        if let Err(error) = self.event_bus.emit(EventEnvelope::new(
            "skill.unloaded",
            KernelContext::new("skills", KernelScope::global()),
            Some(SharedArc::new(serde_json::json!({"skillId": id}))),
        )) {
            tracing::warn!(event = "skill.unloaded", error = %error, "Failed to emit skills event");
        }
        Ok(())
    }

    /// 获取 Skill store 引用
    pub fn store(&self) -> &SkillStore {
        &self.store
    }

    /// 获取命令管理器引用
    pub fn command_manager(&self) -> &CommandManager {
        &self.command_manager
    }

    /// 发出 skill.loaded 事件
    fn emit_skill_loaded(&self, skill_id: &str, source: &str) {
        if let Err(error) = self.event_bus.emit(EventEnvelope::new(
            "skill.loaded",
            KernelContext::new("skills", KernelScope::global()),
            Some(SharedArc::new(serde_json::json!({
                "skillId": skill_id,
                "source": source,
            }))),
        )) {
            tracing::warn!(event = "skill.loaded", error = %error, "Failed to emit skills event");
        }
    }
}

fn extension_skill_id(extension_id: &str, skill_id: &str) -> String {
    format!("extension:{}/{}", extension_id.trim(), skill_id.trim())
}

fn extension_skill_definition(
    extension_id: &str,
    skill: &ExtensionSkillDefinition,
) -> Result<SkillDefinition> {
    let config = skill.config.as_object().ok_or_else(|| {
        anyhow::anyhow!("Extension skill '{}' config must be an object", skill.id)
    })?;

    let name = skill.name.trim();
    if name.is_empty() {
        return Err(anyhow::anyhow!(
            "Extension skill id '{}' has an empty name",
            skill.id
        ));
    }

    let skill_id = skill.id.trim();
    if skill_id.is_empty() {
        return Err(anyhow::anyhow!(
            "Extension skill name '{}' has an empty id",
            skill.name
        ));
    }

    let mode = config
        .get("mode")
        .and_then(|value| value.as_str())
        .and_then(SkillMode::from_str)
        .unwrap_or(SkillMode::Standard);
    let version = config
        .get("version")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("1.0.0")
        .to_string();
    let trigger = config
        .get("trigger")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let tools_whitelist = config
        .get("tools")
        .and_then(|value| value.as_array())
        .map(|tools| {
            tools
                .iter()
                .filter_map(|tool| tool.as_str().map(str::trim))
                .filter(|tool| !tool.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let parameters = config
        .get("parameters")
        .cloned()
        .map(serde_json::from_value::<Vec<SkillParameter>>)
        .transpose()
        .map_err(|error| {
            anyhow::anyhow!(
                "Extension skill '{}' parameters invalid: {}",
                skill.id,
                error
            )
        })?
        .unwrap_or_default();
    let steps = config
        .get("steps")
        .cloned()
        .map(serde_json::from_value::<Vec<SkillStep>>)
        .transpose()
        .map_err(|error| {
            anyhow::anyhow!("Extension skill '{}' steps invalid: {}", skill.id, error)
        })?
        .unwrap_or_default();
    let content = config
        .get("content")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Extension skill '{}' must define non-empty config.content",
                skill.id
            )
        })?;

    Ok(SkillDefinition {
        id: extension_skill_id(extension_id, skill_id),
        name: name.to_string(),
        description: skill.description.clone().unwrap_or_default(),
        mode,
        version,
        source: SkillSource::Extension,
        file_path: PathBuf::from(format!("extension:{}/{}", extension_id.trim(), skill_id)),
        trigger,
        tools_whitelist,
        parameters,
        steps,
        content,
        enabled: true,
        source_url: None,
        installed_at: None,
        author: None,
        tags: None,
        min_navis_version: None,
    })
}

fn validate_skill_definition(skill: &SkillDefinition) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if skill.name.trim().is_empty() {
        errors.push("'name' field cannot be empty".to_string());
    }

    if skill.description.trim().is_empty() {
        warnings.push("Missing 'description' field".to_string());
    }

    if let Some(trigger) = skill.trigger.as_deref() {
        if !trigger.starts_with('/') {
            errors.push(format!("Trigger must start with '/': '{}'", trigger));
        }
        if trigger.len() < 2 {
            errors.push("Trigger must have a name after '/'".to_string());
        }
    }

    for param in &skill.parameters {
        if param.name.trim().is_empty() {
            errors.push("Parameter name cannot be empty".to_string());
        }
        if param.required && param.default.is_some() {
            warnings.push(format!(
                "Parameter '{}' is required but has a default value",
                param.name
            ));
        }
    }

    match skill.mode {
        SkillMode::Standard => {}
        SkillMode::Enhanced => {
            if skill.steps.is_empty() {
                errors.push("Enhanced mode requires at least one step definition".to_string());
            } else {
                let validator = SkillValidator::new();
                validator.validate_steps(&skill.steps, &mut errors);
            }
        }
    }

    if skill.content.trim().is_empty() {
        warnings.push("Skill content (body) is empty".to_string());
    }

    if errors.is_empty() {
        ValidationResult::ok().with_warnings(warnings)
    } else {
        ValidationResult::fail(errors).with_warnings(warnings)
    }
}

fn extension_id_from_skill(skill: &SkillDefinition) -> Option<String> {
    if skill.source != SkillSource::Extension {
        return None;
    }

    skill
        .id
        .strip_prefix("extension:")
        .and_then(|value| value.split('/').next())
        .map(str::to_string)
}

/// 触发解析结果
#[derive(Debug, Clone)]
pub enum ResolvedTrigger {
    /// 匹配到命令
    Command(String),
    /// 匹配到 Skill
    Skill(String),
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;
    use tokio::runtime::Runtime;

    fn test_runtime_handle() -> tokio::runtime::Handle {
        static RUNTIME: OnceLock<Runtime> = OnceLock::new();
        RUNTIME
            .get_or_init(|| Runtime::new().expect("test tokio runtime"))
            .handle()
            .clone()
    }

    // ============ SkillMode ============

    #[test]
    fn test_skill_mode_display() {
        assert_eq!(format!("{}", SkillMode::Standard), "standard");
        assert_eq!(format!("{}", SkillMode::Enhanced), "enhanced");
    }

    #[test]
    fn test_skill_mode_from_str() {
        assert_eq!(SkillMode::from_str("standard"), Some(SkillMode::Standard));
        assert_eq!(SkillMode::from_str("enhanced"), Some(SkillMode::Enhanced));
        assert_eq!(SkillMode::from_str("STANDARD"), Some(SkillMode::Standard));
        assert_eq!(SkillMode::from_str("unknown"), None);
    }

    // ============ SkillSource ============

    #[test]
    fn test_skill_source_display() {
        assert_eq!(format!("{}", SkillSource::Builtin), "builtin");
        assert_eq!(format!("{}", SkillSource::User), "user");
        assert_eq!(format!("{}", SkillSource::Project), "project");
        assert_eq!(format!("{}", SkillSource::Extension), "extension");
    }

    #[test]
    fn test_skill_source_from_str() {
        assert_eq!(SkillSource::from_str("builtin"), Some(SkillSource::Builtin));
        assert_eq!(SkillSource::from_str("user"), Some(SkillSource::User));
        assert_eq!(SkillSource::from_str("project"), Some(SkillSource::Project));
        assert_eq!(
            SkillSource::from_str("extension"),
            Some(SkillSource::Extension)
        );
        assert_eq!(SkillSource::from_str("unknown"), None);
    }

    #[test]
    fn test_skill_source_label() {
        assert_eq!(SkillSource::Builtin.label(), "[内置]");
        assert_eq!(SkillSource::User.label(), "[用户]");
        assert_eq!(SkillSource::Project.label(), "[项目]");
        assert_eq!(SkillSource::Extension.label(), "[扩展]");
    }

    // ============ OnFailureAction ============

    #[test]
    fn test_on_failure_action_from_str() {
        assert_eq!(
            OnFailureAction::from_str("retry"),
            Some(OnFailureAction::Retry)
        );
        assert_eq!(
            OnFailureAction::from_str("fail"),
            Some(OnFailureAction::Fail)
        );
        assert_eq!(
            OnFailureAction::from_str("skip"),
            Some(OnFailureAction::Skip)
        );
        assert_eq!(OnFailureAction::from_str("unknown"), None);
    }

    // ============ StepStatus ============

    #[test]
    fn test_step_status_default() {
        let status = StepStatus::Pending;
        assert_eq!(status, StepStatus::Pending);
    }

    // ============ Skills 初始化 ============

    #[test]
    fn test_skills_init() {
        let config = Arc::new(Mutex::new(Config::new(Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        ))));
        let handle = test_runtime_handle();
        let _guard = handle.enter();
        let skills = Skills::init_for_test(config).unwrap();
        assert_eq!(skills.store().count(), 0);
        assert_eq!(skills.command_manager().count(), 0);
    }

    // ============ 序列化 ============

    #[test]
    fn test_skill_mode_serialization() {
        let mode = SkillMode::Standard;
        let json_str = serde_json::to_string(&mode).unwrap();
        let deserialized: SkillMode = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized, SkillMode::Standard);
    }

    #[test]
    fn test_skill_source_serialization() {
        let source = SkillSource::Project;
        let json_str = serde_json::to_string(&source).unwrap();
        let deserialized: SkillSource = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized, SkillSource::Project);
    }

    #[test]
    fn test_on_failure_action_serialization() {
        let action = OnFailureAction::Skip;
        let json_str = serde_json::to_string(&action).unwrap();
        let deserialized: OnFailureAction = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized, OnFailureAction::Skip);
    }
}
