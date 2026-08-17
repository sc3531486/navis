//! Skill 安装器
//!
//! 提供从 URL 或内容安装 Skill 的能力：
//! - `install_from_url` - 从远程 URL 下载并安装
//! - `install_from_content` - 从字符串内容直接安装
//! - `uninstall` - 卸载已安装的 Skill
//! - `list_installed` - 列出所有已安装的 Skill
//! - `update` - 更新已安装的 Skill（重新下载）
//!
//! 安装范围：
//! - `InstallScope::User` - 用户级，安装到 `~/.navis/skills/`
//! - `InstallScope::Project` - 项目级，安装到 `{cwd}/.navis/skills/`

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use super::loader::SkillLoader;
use super::validator::SkillValidator;
use super::SkillDefinition;
use crate::kernel::{EventBus, EventEnvelope, KernelContext, KernelScope};
use triomphe::Arc as SharedArc;

// ============================================================================
// 安装范围
// ============================================================================

/// Skill 安装范围
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallScope {
    /// 用户级（~/.navis/skills/）
    User,
    /// 项目级（{cwd}/.navis/skills/）
    Project,
}

// ============================================================================
// 安装元数据
// ============================================================================

/// 已安装 Skill 的元数据（注册表条目）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledMeta {
    /// 文件路径
    pub file_path: PathBuf,
    /// 来源 URL（可选）
    pub source_url: Option<String>,
    /// 安装时间
    pub installed_at: DateTime<Utc>,
}

/// 已安装 Skill 信息（用于 list_installed 返回）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledSkill {
    /// Skill 定义
    pub definition: SkillDefinition,
    /// 文件路径
    pub file_path: PathBuf,
    /// 来源 URL
    pub source_url: Option<String>,
    /// 安装时间
    pub installed_at: DateTime<Utc>,
}

// ============================================================================
// Skill 安装器
// ============================================================================

/// Skill 安装器
///
/// 管理 Skill 的安装、卸载、更新和查询。
/// 安装后将 Skill 文件写入磁盘并通过 EventBus 通知系统。
pub struct SkillInstaller {
    /// Skill 格式校验器
    validator: SkillValidator,
    /// 事件总线（用于发布安装/卸载事件）
    event_bus: Arc<dyn EventBus>,
    /// 已安装 Skill 注册表（内存，按需持久化）
    registry: HashMap<String, InstalledMeta>,
}

impl SkillInstaller {
    /// 创建新的 Skill 安装器
    pub fn new(event_bus: Arc<dyn EventBus>) -> Self {
        tracing::debug!("Creating SkillInstaller");
        Self {
            validator: SkillValidator::new(),
            event_bus,
            registry: HashMap::new(),
        }
    }

    /// 获取用户级 Skills 目录（~/.navis/skills/）
    fn user_skills_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".navis").join("skills"))
    }

    /// 获取项目级 Skills 目录（{cwd}/.navis/skills/）
    fn project_skills_dir() -> Option<PathBuf> {
        std::env::current_dir()
            .ok()
            .map(|cwd| cwd.join(".navis").join("skills"))
    }

    /// 根据安装范围获取目标目录
    fn target_dir(scope: &InstallScope) -> Option<PathBuf> {
        match scope {
            InstallScope::User => Self::user_skills_dir(),
            InstallScope::Project => Self::project_skills_dir(),
        }
    }

    /// 从 URL 下载并安装 Skill
    ///
    /// 1. 通过 reqwest 下载 .md 文件
    /// 2. 校验内容是合法的 SKILL.md（YAML frontmatter 能解析）
    /// 3. 根据 scope 写入对应目录
    /// 4. 加载并注册到 SkillStore（通过 SkillLoader）
    /// 5. 通过 EventBus 发布 `skill.installed` 事件
    pub async fn install_from_url(
        &mut self,
        url: &str,
        scope: InstallScope,
    ) -> Result<SkillDefinition> {
        tracing::info!(url = %url, scope = ?scope, "Installing skill from URL");

        // 1. 下载
        let response = reqwest::get(url)
            .await
            .with_context(|| format!("Failed to fetch skill from '{}'", url))?;

        let status = response.status();
        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "HTTP {} when fetching skill from '{}'",
                status,
                url
            ));
        }

        let content = response
            .text()
            .await
            .with_context(|| format!("Failed to read response body from '{}'", url))?;

        // 2. 校验内容
        let validation = self.validator.validate_content(&content);
        if !validation.valid {
            return Err(anyhow::anyhow!(
                "Invalid SKILL.md content from '{}': {}",
                url,
                validation.errors.join("; ")
            ));
        }
        for warning in &validation.warnings {
            tracing::debug!(url = %url, warning = %warning, "Skill validation warning");
        }

        // 3. 获取目标目录
        let target_dir = Self::target_dir(&scope).ok_or_else(|| {
            anyhow::anyhow!("Cannot determine target directory for scope {:?}", scope)
        })?;

        // 4. 用 SkillLoader 解析内容（提取 name、验证格式）
        let loader = SkillLoader::new();
        let parsed = loader
            .parse_from_content(&content, url)
            .map_err(|e| anyhow::anyhow!("Failed to parse skill content: {}", e))?;

        // 5. 计算文件名（基于 parsed name）
        let filename = format!("{}.md", parsed.name);
        let target_path = target_dir.join(&filename);

        // 确保目录存在
        std::fs::create_dir_all(&target_dir).with_context(|| {
            format!(
                "Failed to create skills directory '{}'",
                target_dir.display()
            )
        })?;

        // 6. 写入文件
        std::fs::write(&target_path, &content)
            .with_context(|| format!("Failed to write skill file '{}'", target_path.display()))?;

        tracing::info!(path = %target_path.display(), "Skill file written");

        // 7. 用 loader 加载完整 SkillDefinition（含 file_path）
        let mut skill = loader
            .load_file(&target_path, super::SkillSource::User)
            .map_err(|e| anyhow::anyhow!("Failed to load installed skill: {}", e))?;

        // 8. 填充安装元数据
        skill.source_url = Some(url.to_string());
        skill.installed_at = Some(Utc::now());
        skill.file_path = target_path.clone();

        // 9. 记录注册表
        self.registry.insert(
            skill.id.clone(),
            InstalledMeta {
                file_path: target_path,
                source_url: skill.source_url.clone(),
                installed_at: skill.installed_at.unwrap(),
            },
        );

        // 10. 发布事件
        self.emit_installed_event(&skill);

        tracing::info!(
            skill_id = %skill.id,
            name = %skill.name,
            path = %skill.file_path.display(),
            "Skill installed from URL"
        );

        Ok(skill)
    }

    /// 从内容字符串安装 Skill
    ///
    /// 直接从内容安装，用于手动粘贴或 API 调用。
    /// 同样的校验和写入逻辑。
    pub async fn install_from_content(
        &mut self,
        name: &str,
        content: &str,
        scope: InstallScope,
    ) -> Result<SkillDefinition> {
        tracing::info!(name = %name, scope = ?scope, "Installing skill from content");

        // 1. 校验内容
        let validation = self.validator.validate_content(content);
        if !validation.valid {
            return Err(anyhow::anyhow!(
                "Invalid SKILL.md content: {}",
                validation.errors.join("; ")
            ));
        }
        for warning in &validation.warnings {
            tracing::debug!(name = %name, warning = %warning, "Skill validation warning");
        }

        // 2. 获取目标目录
        let target_dir = Self::target_dir(&scope).ok_or_else(|| {
            anyhow::anyhow!("Cannot determine target directory for scope {:?}", scope)
        })?;

        // 3. 用 SkillLoader 解析内容
        let loader = SkillLoader::new();
        let parsed = loader
            .parse_from_content(content, name)
            .map_err(|e| anyhow::anyhow!("Failed to parse skill content: {}", e))?;

        // 4. 计算文件名
        let filename = format!("{}.md", parsed.name);
        let target_path = target_dir.join(&filename);

        // 确保目录存在
        std::fs::create_dir_all(&target_dir).with_context(|| {
            format!(
                "Failed to create skills directory '{}'",
                target_dir.display()
            )
        })?;

        // 5. 写入文件
        std::fs::write(&target_path, content)
            .with_context(|| format!("Failed to write skill file '{}'", target_path.display()))?;

        tracing::info!(path = %target_path.display(), "Skill file written");

        // 6. 加载完整 SkillDefinition
        let mut skill = loader
            .load_file(&target_path, super::SkillSource::User)
            .map_err(|e| anyhow::anyhow!("Failed to load installed skill: {}", e))?;

        // 7. 填充安装元数据
        skill.installed_at = Some(Utc::now());
        skill.file_path = target_path.clone();

        // 8. 记录注册表
        self.registry.insert(
            skill.id.clone(),
            InstalledMeta {
                file_path: target_path,
                source_url: skill.source_url.clone(),
                installed_at: skill.installed_at.unwrap(),
            },
        );

        // 9. 发布事件
        self.emit_installed_event(&skill);

        tracing::info!(
            skill_id = %skill.id,
            name = %skill.name,
            path = %skill.file_path.display(),
            "Skill installed from content"
        );

        Ok(skill)
    }

    /// 卸载已安装的 Skill
    ///
    /// 根据 skill_id 找到文件路径，删除文件，从 SkillStore 注销，
    /// 并通过 EventBus 发布 `skill.uninstalled` 事件。
    pub fn uninstall(&mut self, skill_id: &str) -> Result<()> {
        tracing::info!(skill_id = %skill_id, "Uninstalling skill");

        // 1. 从注册表获取元数据
        let meta = self
            .registry
            .remove(skill_id)
            .ok_or_else(|| anyhow::anyhow!("Skill not found in registry: {}", skill_id))?;

        // 2. 删除文件
        if meta.file_path.exists() {
            std::fs::remove_file(&meta.file_path).with_context(|| {
                format!("Failed to delete skill file '{}'", meta.file_path.display())
            })?;
            tracing::info!(path = %meta.file_path.display(), "Skill file deleted");
        }

        // 3. 发布事件
        if let Err(error) = self.event_bus.emit(EventEnvelope::new(
            "skill.uninstalled",
            KernelContext::new("skill_installer", KernelScope::global()),
            Some(SharedArc::new(serde_json::json!({
                "skillId": skill_id,
                "filePath": meta.file_path.to_string_lossy(),
            }))),
        )) {
            tracing::warn!(
                event = "skill.uninstalled",
                error = %error,
                "Failed to emit skill.uninstalled event"
            );
        }

        tracing::info!(skill_id = %skill_id, "Skill uninstalled");
        Ok(())
    }

    /// 列出所有已安装的 Skill
    ///
    /// 扫描用户和项目级 skills 目录，结合注册表元数据返回完整信息。
    pub fn list_installed(&self) -> Vec<InstalledSkill> {
        let mut installed = Vec::new();

        // 扫描所有可能的 skills 目录
        let dirs: Vec<(PathBuf, super::SkillSource)> = [
            Self::user_skills_dir().map(|d| (d, super::SkillSource::User)),
            Self::project_skills_dir().map(|d| (d, super::SkillSource::Project)),
        ]
        .into_iter()
        .flatten()
        .collect();

        let loader = SkillLoader::new();

        for (dir, source) in &dirs {
            if !dir.exists() {
                continue;
            }

            let entries = match std::fs::read_dir(dir) {
                Ok(entries) => entries,
                Err(e) => {
                    tracing::warn!(dir = %dir.display(), error = %e, "Failed to read skills directory");
                    continue;
                }
            };

            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                match path.extension().and_then(|ext| ext.to_str()) {
                    Some("md") => {}
                    _ => continue,
                }

                match loader.load_file(&path, source.clone()) {
                    Ok(mut skill) => {
                        // 从注册表补充元数据
                        if let Some(meta) = self.registry.get(&skill.id) {
                            skill.source_url = meta.source_url.clone();
                            skill.installed_at = Some(meta.installed_at);
                            skill.file_path = meta.file_path.clone();
                        } else {
                            // 未在注册表中（可能是手动放入的文件），仍然列出
                            skill.file_path = path.clone();
                        }

                        let source_url = skill.source_url.clone();
                        let installed_at = skill.installed_at.unwrap_or_else(Utc::now);

                        installed.push(InstalledSkill {
                            definition: skill,
                            file_path: path,
                            source_url,
                            installed_at,
                        });
                    }
                    Err(e) => {
                        tracing::debug!(
                            path = %path.display(),
                            error = %e,
                            "Skipping unparseable skill file"
                        );
                    }
                }
            }
        }

        installed
    }

    /// 更新已安装的 Skill
    ///
    /// 如果 skill 有 source_url，重新下载并覆盖。
    /// 保留旧版本备份（.md.bak）。
    pub async fn update(&mut self, skill_id: &str) -> Result<SkillDefinition> {
        tracing::info!(skill_id = %skill_id, "Updating skill");

        // 1. 获取注册表元数据
        let meta = self
            .registry
            .get(skill_id)
            .ok_or_else(|| anyhow::anyhow!("Skill not found in registry: {}", skill_id))?;

        let source_url = meta
            .source_url
            .as_ref()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Skill '{}' has no source_url, cannot update automatically",
                    skill_id
                )
            })?
            .clone();

        let file_path = meta.file_path.clone();

        // 2. 创建备份
        let backup_path = PathBuf::from(format!("{}.bak", file_path.display()));
        if file_path.exists() {
            std::fs::copy(&file_path, &backup_path).with_context(|| {
                format!(
                    "Failed to create backup '{}' for update",
                    backup_path.display()
                )
            })?;
            tracing::debug!(path = %backup_path.display(), "Backup created");
        }

        // 3. 重新下载
        let response = reqwest::get(&source_url)
            .await
            .with_context(|| format!("Failed to fetch skill from '{}'", source_url))?;

        let status = response.status();
        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "HTTP {} when fetching skill from '{}' for update",
                status,
                source_url
            ));
        }

        let content = response
            .text()
            .await
            .with_context(|| format!("Failed to read response from '{}'", source_url))?;

        // 4. 校验
        let validation = self.validator.validate_content(&content);
        if !validation.valid {
            // 还原备份
            if backup_path.exists() {
                std::fs::copy(&backup_path, &file_path)?;
                tracing::warn!("Invalid content, restored from backup");
            }
            return Err(anyhow::anyhow!(
                "Invalid SKILL.md content from '{}': {}",
                source_url,
                validation.errors.join("; ")
            ));
        }

        // 5. 写入新内容
        std::fs::write(&file_path, &content).with_context(|| {
            format!(
                "Failed to write updated skill file '{}'",
                file_path.display()
            )
        })?;

        // 6. 重新加载
        let loader = SkillLoader::new();
        let mut skill = loader
            .load_file(&file_path, super::SkillSource::User)
            .map_err(|e| anyhow::anyhow!("Failed to reload updated skill: {}", e))?;

        // 7. 保留元数据
        skill.source_url = Some(source_url);
        skill.installed_at = Some(Utc::now());
        skill.file_path = file_path.clone();

        // 8. 更新注册表
        self.registry.insert(
            skill.id.clone(),
            InstalledMeta {
                file_path,
                source_url: skill.source_url.clone(),
                installed_at: skill.installed_at.unwrap(),
            },
        );

        // 9. 发布事件
        self.emit_updated_event(&skill);

        tracing::info!(
            skill_id = %skill.id,
            name = %skill.name,
            "Skill updated"
        );

        Ok(skill)
    }

    /// 发出 skill.installed 事件
    fn emit_installed_event(&self, skill: &SkillDefinition) {
        if let Err(error) = self.event_bus.emit(EventEnvelope::new(
            "skill.installed",
            KernelContext::new("skill_installer", KernelScope::global()),
            Some(SharedArc::new(serde_json::json!({
                "skillId": skill.id,
                "name": skill.name,
                "filePath": skill.file_path.to_string_lossy(),
                "sourceUrl": skill.source_url,
            }))),
        )) {
            tracing::warn!(
                event = "skill.installed",
                error = %error,
                "Failed to emit skill.installed event"
            );
        }
    }

    /// 发出 skill.updated 事件
    fn emit_updated_event(&self, skill: &SkillDefinition) {
        if let Err(error) = self.event_bus.emit(EventEnvelope::new(
            "skill.updated",
            KernelContext::new("skill_installer", KernelScope::global()),
            Some(SharedArc::new(serde_json::json!({
                "skillId": skill.id,
                "name": skill.name,
                "filePath": skill.file_path.to_string_lossy(),
                "sourceUrl": skill.source_url,
            }))),
        )) {
            tracing::warn!(
                event = "skill.updated",
                error = %error,
                "Failed to emit skill.updated event"
            );
        }
    }
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

    fn test_installer() -> SkillInstaller {
        let event_bus = Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        ));
        SkillInstaller::new(event_bus)
    }

    #[test]
    fn test_installer_creation() {
        let installer = test_installer();
        assert!(installer.registry.is_empty());
    }

    #[test]
    fn test_user_skills_dir() {
        let dir = SkillInstaller::user_skills_dir();
        assert!(dir.is_some());
        let dir = dir.unwrap();
        assert!(dir.ends_with(".navis/skills"));
    }

    #[test]
    fn test_target_dir_user() {
        let dir = SkillInstaller::target_dir(&InstallScope::User);
        assert!(dir.is_some());
    }

    #[test]
    fn test_target_dir_project() {
        let dir = SkillInstaller::target_dir(&InstallScope::Project);
        assert!(dir.is_some());
    }

    #[test]
    fn test_uninstall_nonexistent() {
        let handle = test_runtime_handle();
        let _guard = handle.enter();
        let mut installer = test_installer();
        let result = installer.uninstall("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_list_installed_empty() {
        let installer = test_installer();
        let _ = installer.list_installed();
        // 可能非空（如果有已安装的 skill 文件）
        // 但注册表应为空
        assert!(installer.registry.is_empty());
    }
}
