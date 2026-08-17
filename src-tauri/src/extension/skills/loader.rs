//! Skill 文件加载器
//!
//! 负责从文件系统加载 SKILL.md 文件，支持：
//! - 内置 Skills（编译时嵌入）
//! - 用户级 Skills（~/.navis/skills/）
//! - 项目级 Skills（.navis/skills/）

use anyhow::Result;
use std::path::{Path, PathBuf};

use super::{SkillDefinition, SkillMode, SkillParser, SkillSource};

/// Skill 加载器
pub struct SkillLoader {
    /// 解析器
    parser: SkillParser,
}

impl SkillLoader {
    /// 创建新的加载器
    pub fn new() -> Self {
        Self {
            parser: SkillParser::new(),
        }
    }

    /// 获取用户级 Skills 目录（~/.navis/skills/）
    pub fn user_skills_dir(&self) -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".navis").join("skills"))
    }

    /// 获取项目级 Skills 目录（.navis/skills/）
    pub fn project_skills_dir(&self) -> Option<PathBuf> {
        // 使用当前工作目录
        std::env::current_dir()
            .ok()
            .map(|cwd| cwd.join(".navis").join("skills"))
    }

    /// 加载内置 Skills
    pub fn load_builtin(&self) -> Vec<SkillDefinition> {
        let mut skills = Vec::new();

        // 加载内嵌的内置 Skill 文件
        let builtin_files: Vec<(&str, &str)> = vec![
            ("commit", include_str!("builtin/commit.md")),
            ("review", include_str!("builtin/review.md")),
            ("explain", include_str!("builtin/explain.md")),
            ("refactor", include_str!("builtin/refactor.md")),
        ];

        for (name, content) in builtin_files {
            match self.parse_and_build(name, content, SkillSource::Builtin, None) {
                Ok(skill) => {
                    tracing::debug!(skill_id = %skill.id, name = %skill.name, "Builtin skill loaded");
                    skills.push(skill);
                }
                Err(e) => {
                    tracing::warn!(name = %name, error = %e, "Failed to load builtin skill");
                }
            }
        }

        tracing::info!(count = skills.len(), "Builtin skills loaded");
        skills
    }

    /// 从目录加载 Skills
    pub fn load_from_dir(&self, dir: &Path, source: SkillSource) -> Result<Vec<SkillDefinition>> {
        let mut skills = Vec::new();

        if !dir.exists() {
            tracing::debug!(dir = %dir.display(), "Skills directory not found, skipping");
            return Ok(skills);
        }

        let entries = std::fs::read_dir(dir).map_err(|e| {
            anyhow::anyhow!("Failed to read skills directory '{}': {}", dir.display(), e)
        })?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            // 只处理 .md 文件
            match path.extension().and_then(|ext| ext.to_str()) {
                Some("md") => {}
                _ => continue,
            }

            match self.load_file(&path, source.clone()) {
                Ok(skill) => {
                    tracing::debug!(
                        skill_id = %skill.id,
                        name = %skill.name,
                        path = %path.display(),
                        "Skill file loaded"
                    );
                    skills.push(skill);
                }
                Err(e) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "Failed to load skill file"
                    );
                }
            }
        }

        tracing::info!(
            dir = %dir.display(),
            source = %source,
            count = skills.len(),
            "Skills loaded from directory"
        );

        Ok(skills)
    }

    /// 加载单个 Skill 文件
    pub fn load_file(&self, path: &Path, source: SkillSource) -> Result<SkillDefinition> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read file '{}': {}", path.display(), e))?;

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        self.parse_and_build(&name, &content, source, Some(path.to_path_buf()))
    }

    /// 从 URL 加载 Skill（仅解析，不写入磁盘）
    ///
    /// 下载指定 URL 的 .md 文件内容并解析为 `SkillDefinition`。
    /// 如果 URL 包含文件路径，则以此作为 `file_path` 的回退值。
    pub async fn load_from_url(&self, url: &str) -> Result<SkillDefinition> {
        tracing::debug!(url = %url, "Loading skill from URL");

        let response = reqwest::get(url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch skill from '{}': {}", url, e))?;

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
            .map_err(|e| anyhow::anyhow!("Failed to read response from '{}': {}", url, e))?;

        // 从 URL 提取文件名作为 fallback_name
        let fallback_name = url
            .rsplit('/')
            .next()
            .unwrap_or("remote-skill")
            .trim_end_matches(".md")
            .trim_end_matches(".markdown")
            .to_string();

        // 将 URL 作为 file_path 的回退
        let fake_path = PathBuf::from(format!("url:{}", url));

        let mut skill =
            self.parse_and_build(&fallback_name, &content, SkillSource::User, Some(fake_path))?;

        // 记录来源 URL，供后续安装使用
        skill.source_url = Some(url.to_string());

        tracing::info!(
            skill_id = %skill.id,
            name = %skill.name,
            url = %url,
            "Skill loaded from URL"
        );

        Ok(skill)
    }

    /// 解析内容并构建 SkillDefinition（不依赖文件系统）
    ///
    /// 用于从 URL 或 API 安装前的预解析校验。
    /// `hint` 用于在 frontmatter 缺少 name 时作为回退值。
    pub fn parse_from_content(&self, content: &str, hint: &str) -> Result<SkillDefinition> {
        let fallback_name = hint
            .rsplit('/')
            .next()
            .unwrap_or(hint)
            .trim_end_matches(".md")
            .trim_end_matches(".markdown")
            .to_string();

        self.parse_and_build(&fallback_name, content, SkillSource::User, None)
    }

    /// 解析内容并构建 SkillDefinition
    fn parse_and_build(
        &self,
        fallback_name: &str,
        content: &str,
        source: SkillSource,
        file_path: Option<PathBuf>,
    ) -> Result<SkillDefinition> {
        let parsed = self
            .parser
            .parse(content)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        let name = parsed.name.unwrap_or_else(|| fallback_name.to_string());
        let id = generate_skill_id(&name, &source);
        let description = parsed.description.unwrap_or_default();
        let mode = parsed.mode.unwrap_or(SkillMode::Standard);
        let version = parsed.version.unwrap_or_else(|| "1.0.0".to_string());
        let trigger = parsed.trigger;
        let tools_whitelist = parsed.tools.unwrap_or_default();
        let parameters = parsed.parameters.unwrap_or_default();
        let steps = parsed.steps.unwrap_or_default();

        // 扩展元数据字段
        let source_url = parsed.source_url;
        let author = parsed.author;
        let tags = parsed.tags;
        let min_navis_version = parsed.min_navis_version;

        Ok(SkillDefinition {
            id,
            name,
            description,
            mode,
            version,
            source,
            file_path: file_path
                .unwrap_or_else(|| PathBuf::from(format!("builtin:{}", fallback_name))),
            trigger,
            tools_whitelist,
            parameters,
            steps,
            content: parsed.content,
            enabled: true,
            source_url,
            installed_at: None,
            author,
            tags,
            min_navis_version,
        })
    }
}

impl Default for SkillLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// 生成 Skill ID
///
/// 格式：{source}:{name}
fn generate_skill_id(name: &str, source: &SkillSource) -> String {
    format!("{}:{}", source, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_builtin_skills() {
        let loader = SkillLoader::new();
        let skills = loader.load_builtin();

        // 应该加载到 4 个内置 Skill
        assert_eq!(skills.len(), 4);

        let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"commit"));
        assert!(names.contains(&"review"));
        assert!(names.contains(&"explain"));
        assert!(names.contains(&"refactor"));
    }

    #[test]
    fn test_builtin_skills_have_correct_source() {
        let loader = SkillLoader::new();
        let skills = loader.load_builtin();

        for skill in &skills {
            assert_eq!(skill.source, SkillSource::Builtin);
            assert!(skill.enabled);
        }
    }

    #[test]
    fn test_builtin_skills_have_triggers() {
        let loader = SkillLoader::new();
        let skills = loader.load_builtin();

        let commit = skills.iter().find(|s| s.name == "commit").unwrap();
        assert_eq!(commit.trigger, Some("/commit".to_string()));

        let review = skills.iter().find(|s| s.name == "review").unwrap();
        assert_eq!(review.trigger, Some("/review".to_string()));
    }

    #[test]
    fn test_load_from_dir() {
        let tmp_dir = TempDir::new().unwrap();
        let skills_dir = tmp_dir.path().join("skills");
        fs::create_dir_all(&skills_dir).unwrap();

        // 写入一个测试 Skill
        let skill_content = r#"---
name: test-skill
description: 测试 Skill
mode: standard
trigger: /test
tools: [read]
---

这是一个测试 Skill。
"#;
        fs::write(skills_dir.join("test-skill.md"), skill_content).unwrap();

        // 写入一个非 .md 文件（应该被忽略）
        fs::write(skills_dir.join("readme.txt"), "not a skill").unwrap();

        let loader = SkillLoader::new();
        let skills = loader
            .load_from_dir(&skills_dir, SkillSource::Project)
            .unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "test-skill");
        assert_eq!(skills[0].source, SkillSource::Project);
    }

    #[test]
    fn test_load_from_nonexistent_dir() {
        let loader = SkillLoader::new();
        let result = loader.load_from_dir(Path::new("/nonexistent/path"), SkillSource::User);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_load_file() {
        let tmp_dir = TempDir::new().unwrap();
        let skill_file = tmp_dir.path().join("my-skill.md");

        let content = r#"---
name: my-skill
description: 我的 Skill
---

提示词内容
"#;
        fs::write(&skill_file, content).unwrap();

        let loader = SkillLoader::new();
        let skill = loader.load_file(&skill_file, SkillSource::User).unwrap();

        assert_eq!(skill.name, "my-skill");
        assert_eq!(skill.description, "我的 Skill");
        assert_eq!(skill.source, SkillSource::User);
        assert_eq!(skill.file_path, skill_file);
        assert!(skill.content.contains("提示词内容"));
    }

    #[test]
    fn test_load_file_without_frontmatter() {
        let tmp_dir = TempDir::new().unwrap();
        let skill_file = tmp_dir.path().join("plain-skill.md");

        let content = "这是一个没有 frontmatter 的 Skill 文件。";
        fs::write(&skill_file, content).unwrap();

        let loader = SkillLoader::new();
        let skill = loader.load_file(&skill_file, SkillSource::Project).unwrap();

        // 名称应使用文件名作为回退
        assert_eq!(skill.name, "plain-skill");
        assert_eq!(skill.mode, SkillMode::Standard);
    }

    #[test]
    fn test_user_skills_dir() {
        let loader = SkillLoader::new();
        let dir = loader.user_skills_dir();
        assert!(dir.is_some());
        let dir = dir.unwrap();
        assert!(dir.ends_with(".navis/skills"));
    }

    #[test]
    fn test_generate_skill_id() {
        let id = generate_skill_id("review", &SkillSource::Builtin);
        assert_eq!(id, "builtin:review");

        let id = generate_skill_id("my-skill", &SkillSource::Project);
        assert_eq!(id, "project:my-skill");
    }
}
