//! 角色模板管理
//!
//! 管理 RoleDefinition 的 CRUD 操作，提供内置角色模板。

use anyhow::Result;
use std::collections::HashMap;

use super::RoleDefinition;

/// 角色管理器
pub struct RoleManager {
    /// 角色存储（id -> RoleDefinition）
    roles: HashMap<String, RoleDefinition>,
}

impl RoleManager {
    /// 创建新的角色管理器（含内置角色）
    pub fn new() -> Self {
        let mut manager = Self {
            roles: HashMap::new(),
        };
        manager.load_builtin_roles();
        manager
    }

    /// 加载内置角色
    fn load_builtin_roles(&mut self) {
        // developer 角色
        self.register(RoleDefinition {
            id: "developer".to_string(),
            name: "开发者".to_string(),
            description: "代码开发角色，负责编写、调试和重构代码".to_string(),
            system_prompt: "你是一个专业的软件开发工程师。".to_string(),
            guidance: Some(
                r#"你是一个独立工作的 Task Sidechain 开发执行者。
- 你不是代码库中唯一的工作者，不要撤销其他人的编辑
- 明确分配文件所有权：只修改与你任务直接相关的文件
- 修改代码前先阅读现有实现，理解上下文
- 遵循项目的代码规范（参见 project_summary 中的 code_standards）
- 每次修改后运行相关测试验证"#
                    .to_string(),
            ),
            skills: vec!["commit".to_string(), "refactor".to_string()],
            commands: vec!["explain".to_string()],
            model_preference: None,
            temperature: Some(0.2),
        });

        // technical-writer 角色
        self.register(RoleDefinition {
            id: "technical-writer".to_string(),
            name: "技术文档编写者".to_string(),
            description: "文档协作角色，负责编写和维护技术文档".to_string(),
            system_prompt: "你是一个专业的技术文档编写者。".to_string(),
            guidance: Some(
                r#"你是一个独立工作的 Task Sidechain 文档执行者。
- 引用代码时附带文件路径和行号
- 文档格式统一使用 Markdown
- 代码示例必须可运行，不要编造不存在的 API
- 优先更新现有文档，而非创建新文件"#
                    .to_string(),
            ),
            skills: vec!["review".to_string(), "explain".to_string()],
            commands: vec!["explain".to_string()],
            model_preference: None,
            temperature: Some(0.5),
        });

        // assistant 角色
        self.register(RoleDefinition {
            id: "assistant".to_string(),
            name: "通用助手".to_string(),
            description: "通用 AI 助手角色".to_string(),
            system_prompt: "你是一个通用 AI 助手。".to_string(),
            guidance: None,
            skills: Vec::new(),
            commands: Vec::new(),
            model_preference: None,
            temperature: Some(0.3),
        });

        tracing::info!(count = self.roles.len(), "Builtin roles loaded");
    }

    /// 注册角色
    pub fn register(&mut self, role: RoleDefinition) {
        let id = role.id.clone();
        tracing::debug!(role_id = %id, name = %role.name, "Registering role");
        self.roles.insert(id, role);
    }

    /// 获取角色
    pub fn get(&self, id: &str) -> Option<&RoleDefinition> {
        self.roles.get(id)
    }

    /// 列出所有角色
    pub fn list(&self) -> Vec<&RoleDefinition> {
        self.roles.values().collect()
    }

    /// 创建角色
    pub fn create(&mut self, role: RoleDefinition) -> Result<()> {
        if self.roles.contains_key(&role.id) {
            return Err(anyhow::anyhow!("Role already exists: {}", role.id));
        }

        let id = role.id.clone();
        tracing::info!(role_id = %id, name = %role.name, "Creating role");
        self.roles.insert(id, role);
        Ok(())
    }

    /// 更新角色
    pub fn update(&mut self, id: &str, updates: RoleUpdate) -> Result<()> {
        let role = self
            .roles
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Role not found: {}", id))?;

        if let Some(name) = updates.name {
            role.name = name;
        }
        if let Some(description) = updates.description {
            role.description = description;
        }
        if let Some(system_prompt) = updates.system_prompt {
            role.system_prompt = system_prompt;
        }
        if let Some(guidance) = updates.guidance {
            role.guidance = Some(guidance);
        }
        if let Some(skills) = updates.skills {
            role.skills = skills;
        }
        if let Some(commands) = updates.commands {
            role.commands = commands;
        }
        if let Some(model_preference) = updates.model_preference {
            role.model_preference = Some(model_preference);
        }
        if let Some(temperature) = updates.temperature {
            role.temperature = Some(temperature);
        }

        tracing::info!(role_id = %id, "Role updated");
        Ok(())
    }

    /// 删除角色
    pub fn delete(&mut self, id: &str) -> Result<()> {
        if self.roles.remove(id).is_some() {
            tracing::info!(role_id = %id, "Role deleted");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Role not found: {}", id))
        }
    }

    /// 获取角色数量
    pub fn count(&self) -> usize {
        self.roles.len()
    }
}

impl Default for RoleManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 角色更新数据
#[derive(Debug, Clone, Default)]
pub struct RoleUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub guidance: Option<String>,
    pub skills: Option<Vec<String>>,
    pub commands: Option<Vec<String>>,
    pub model_preference: Option<String>,
    pub temperature: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_roles_loaded() {
        let manager = RoleManager::new();
        assert!(manager.count() >= 3);

        // 检查内置角色
        assert!(manager.get("developer").is_some());
        assert!(manager.get("technical-writer").is_some());
        assert!(manager.get("assistant").is_some());
    }

    #[test]
    fn test_developer_role() {
        let manager = RoleManager::new();
        let dev = manager.get("developer").unwrap();

        assert_eq!(dev.id, "developer");
        assert_eq!(dev.name, "开发者");
        assert!(dev.guidance.is_some());
        assert!(dev.guidance.as_ref().unwrap().contains("独立工作"));
        assert!(dev.skills.contains(&"commit".to_string()));
        assert_eq!(dev.temperature, Some(0.2));
    }

    #[test]
    fn test_technical_writer_role() {
        let manager = RoleManager::new();
        let writer = manager.get("technical-writer").unwrap();

        assert_eq!(writer.id, "technical-writer");
        assert!(writer.guidance.is_some());
        assert!(writer.guidance.as_ref().unwrap().contains("Markdown"));
        assert_eq!(writer.temperature, Some(0.5));
    }

    #[test]
    fn test_create_role() {
        let mut manager = RoleManager::new();

        let role = RoleDefinition {
            id: "custom-role".to_string(),
            name: "自定义角色".to_string(),
            description: "测试角色".to_string(),
            system_prompt: "自定义系统提示词".to_string(),
            guidance: None,
            skills: Vec::new(),
            commands: Vec::new(),
            model_preference: Some("gpt-4".to_string()),
            temperature: Some(0.7),
        };

        manager.create(role).unwrap();
        let created = manager.get("custom-role").unwrap();
        assert_eq!(created.name, "自定义角色");
        assert_eq!(created.model_preference, Some("gpt-4".to_string()));
    }

    #[test]
    fn test_create_duplicate_role() {
        let mut manager = RoleManager::new();

        let role = RoleDefinition {
            id: "developer".to_string(),
            name: "重复角色".to_string(),
            description: "".to_string(),
            system_prompt: "".to_string(),
            guidance: None,
            skills: Vec::new(),
            commands: Vec::new(),
            model_preference: None,
            temperature: None,
        };

        let result = manager.create(role);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_update_role() {
        let mut manager = RoleManager::new();

        let updates = RoleUpdate {
            name: Some("新名称".to_string()),
            temperature: Some(0.9),
            ..Default::default()
        };

        manager.update("developer", updates).unwrap();

        let updated = manager.get("developer").unwrap();
        assert_eq!(updated.name, "新名称");
        assert_eq!(updated.temperature, Some(0.9));
        // 未更新的字段保持不变
        assert_eq!(updated.id, "developer");
    }

    #[test]
    fn test_update_nonexistent_role() {
        let mut manager = RoleManager::new();
        let updates = RoleUpdate::default();
        let result = manager.update("nonexistent", updates);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_role() {
        let mut manager = RoleManager::new();

        let role = RoleDefinition {
            id: "to-delete".to_string(),
            name: "待删除".to_string(),
            description: "".to_string(),
            system_prompt: "".to_string(),
            guidance: None,
            skills: Vec::new(),
            commands: Vec::new(),
            model_preference: None,
            temperature: None,
        };

        manager.create(role).unwrap();
        assert!(manager.get("to-delete").is_some());

        manager.delete("to-delete").unwrap();
        assert!(manager.get("to-delete").is_none());
    }

    #[test]
    fn test_delete_nonexistent_role() {
        let mut manager = RoleManager::new();
        let result = manager.delete("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_roles() {
        let manager = RoleManager::new();
        let roles = manager.list();
        assert!(!roles.is_empty());

        let ids: Vec<&str> = roles.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"developer"));
        assert!(ids.contains(&"technical-writer"));
        assert!(ids.contains(&"assistant"));
    }

    #[test]
    fn test_role_guidance_content() {
        let manager = RoleManager::new();
        let dev = manager.get("developer").unwrap();

        let guidance = dev.guidance.as_ref().unwrap();
        assert!(guidance.contains("不要撤销其他人的编辑"));
        assert!(guidance.contains("运行相关测试验证"));

        let writer = manager.get("technical-writer").unwrap();
        let guidance = writer.guidance.as_ref().unwrap();
        assert!(guidance.contains("文件路径和行号"));
        assert!(guidance.contains("可运行"));
    }
}
