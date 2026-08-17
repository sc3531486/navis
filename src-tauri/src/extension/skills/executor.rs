//! Skill 激活服务（标准模式）
//!
//! 负责 Skill 的标准模式激活：
//! - 参数校验与替换
//! - 激活计划构建（prompt context + 工具白名单 + 参数）
//!
//! 注意：激活服务不直接执行工具，而是构建激活计划供 Agent 使用。

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::SkillDefinition;

/// Skill 激活计划
///
/// 由 `SkillActivationService::build_activation_plan()` 产出，供 Agent 注入 system prompt
/// 并约束工具调用范围。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillActivationPlan {
    /// Skill ID
    pub skill_id: String,
    /// Skill 名称
    pub skill_name: String,
    /// 注入到 system prompt 的上下文内容
    pub context_prompt: String,
    /// 允许使用的工具列表
    pub tools_whitelist: Vec<String>,
    /// 已解析的参数
    pub parameters: HashMap<String, String>,
    /// 激活来源："user" / "auto" / "trigger"
    pub source: String,
    /// Enhanced 模式下的原始步骤定义（标准模式下为空）
    pub steps: Vec<super::SkillStep>,
    /// Enhanced 模式当前执行到第几步（0-based），由 SkillStepStage 更新
    pub current_step: usize,
    /// 是否为 Enhanced 模式（steps 非空时自动设为 true）
    pub is_enhanced: bool,
}

/// Skill 激活服务
///
/// 构建激活计划（prompt context + 工具白名单 + 参数），供 Agent 消费。
pub struct SkillActivationService;

impl SkillActivationService {
    /// 创建新的激活服务
    pub fn new() -> Self {
        Self
    }

    /// 构建 Skill 激活计划
    ///
    /// 校验必填参数并替换占位符，产出 `SkillActivationPlan` 供 Agent 使用。
    pub fn build_activation_plan(
        &self,
        skill: &SkillDefinition,
        params: HashMap<String, String>,
        source: impl Into<String>,
    ) -> Result<SkillActivationPlan> {
        tracing::info!(
            skill_id = %skill.id,
            name = %skill.name,
            mode = %skill.mode,
            "Building skill activation plan"
        );

        // 1. 校验必填参数
        self.validate_params(skill, &params)?;

        // 2. 构建上下文（参数替换）
        let context_prompt = self.build_context(skill, &params)?;

        tracing::info!(
            skill_id = %skill.id,
            "Skill activation plan built"
        );

        // Enhanced 模式：保留原始 steps 并设置 current_step
        let is_enhanced = skill.mode == super::SkillMode::Enhanced && !skill.steps.is_empty();

        Ok(SkillActivationPlan {
            skill_id: skill.id.clone(),
            skill_name: skill.name.clone(),
            context_prompt,
            tools_whitelist: skill.tools_whitelist.clone(),
            parameters: params,
            source: source.into(),
            steps: if is_enhanced {
                skill.steps.clone()
            } else {
                Vec::new()
            },
            current_step: 0,
            is_enhanced,
        })
    }

    /// 校验必填参数
    fn validate_params(
        &self,
        skill: &SkillDefinition,
        params: &HashMap<String, String>,
    ) -> Result<()> {
        for param in &skill.parameters {
            if param.required && !params.contains_key(&param.name) && param.default.is_none() {
                return Err(anyhow::anyhow!(
                    "Required parameter '{}' not provided for skill '{}'",
                    param.name,
                    skill.name
                ));
            }
        }
        Ok(())
    }

    /// 构建提示词上下文
    ///
    /// 替换占位符 ${param_name}，使用提供的值或默认值
    fn build_context(
        &self,
        skill: &SkillDefinition,
        params: &HashMap<String, String>,
    ) -> Result<String> {
        let mut content = skill.content.clone();

        for param in &skill.parameters {
            let placeholder = format!("${{{}}}", param.name);
            let value = params.get(&param.name).or(param.default.as_ref());

            if let Some(val) = value {
                content = content.replace(&placeholder, val);
            }
            // 必填参数已在 validate_params 中校验
        }

        Ok(content)
    }
}

impl Default for SkillActivationService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::{SkillMode, SkillParameter, SkillSource};
    use super::*;
    use std::path::PathBuf;

    fn create_test_skill() -> SkillDefinition {
        SkillDefinition {
            id: "test:review".to_string(),
            name: "review".to_string(),
            description: "Code review".to_string(),
            mode: SkillMode::Standard,
            version: "1.0.0".to_string(),
            source: SkillSource::Builtin,
            file_path: PathBuf::from("builtin:review"),
            trigger: Some("/review".to_string()),
            tools_whitelist: vec!["read".to_string(), "lsp.diagnostics".to_string()],
            parameters: vec![
                SkillParameter {
                    name: "focus".to_string(),
                    description: "审查重点".to_string(),
                    required: false,
                    default: Some("all".to_string()),
                    param_type: "string".to_string(),
                },
                SkillParameter {
                    name: "target".to_string(),
                    description: "审查目标".to_string(),
                    required: true,
                    default: None,
                    param_type: "string".to_string(),
                },
            ],
            steps: Vec::new(),
            content: "审查 ${target}，重点：${focus}".to_string(),
            enabled: true,
            source_url: None,
            installed_at: None,
            author: None,
            tags: None,
            min_navis_version: None,
        }
    }

    #[test]
    fn test_build_activation_plan_with_all_params() {
        let service = SkillActivationService::new();
        let skill = create_test_skill();

        let mut params = HashMap::new();
        params.insert("target".to_string(), "src/auth.rs".to_string());
        params.insert("focus".to_string(), "security".to_string());

        let plan = service
            .build_activation_plan(&skill, params, "user")
            .unwrap();
        assert!(plan.context_prompt.contains("src/auth.rs"));
        assert!(plan.context_prompt.contains("security"));
        assert_eq!(plan.tools_whitelist.len(), 2);
        assert_eq!(plan.source, "user");
        assert_eq!(plan.skill_id, "test:review");
    }

    #[test]
    fn test_build_activation_plan_with_default_param() {
        let service = SkillActivationService::new();
        let skill = create_test_skill();

        let mut params = HashMap::new();
        params.insert("target".to_string(), "src/main.rs".to_string());

        let plan = service
            .build_activation_plan(&skill, params, "trigger")
            .unwrap();
        assert!(plan.context_prompt.contains("src/main.rs"));
        assert!(plan.context_prompt.contains("all")); // default value
        assert_eq!(plan.source, "trigger");
    }

    #[test]
    fn test_build_activation_plan_missing_required_param() {
        let service = SkillActivationService::new();
        let skill = create_test_skill();

        let params = HashMap::new(); // missing 'target'

        let result = service.build_activation_plan(&skill, params, "auto");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Required parameter 'target'"));
    }

    #[test]
    fn test_build_activation_plan_no_params_needed() {
        let service = SkillActivationService::new();
        let skill = SkillDefinition {
            id: "test:simple".to_string(),
            name: "simple".to_string(),
            description: "Simple skill".to_string(),
            mode: SkillMode::Standard,
            version: "1.0.0".to_string(),
            source: SkillSource::Builtin,
            file_path: PathBuf::from("builtin:simple"),
            trigger: None,
            tools_whitelist: Vec::new(),
            parameters: Vec::new(),
            steps: Vec::new(),
            content: "固定提示词内容".to_string(),
            enabled: true,
            source_url: None,
            installed_at: None,
            author: None,
            tags: None,
            min_navis_version: None,
        };

        let plan = service
            .build_activation_plan(&skill, HashMap::new(), "user")
            .unwrap();
        assert_eq!(plan.context_prompt, "固定提示词内容");
    }

    #[test]
    fn test_build_activation_plan_preserves_unreplaced_placeholders() {
        let service = SkillActivationService::new();
        let skill = SkillDefinition {
            id: "test:partial".to_string(),
            name: "partial".to_string(),
            description: "Partial replace".to_string(),
            mode: SkillMode::Standard,
            version: "1.0.0".to_string(),
            source: SkillSource::Builtin,
            file_path: PathBuf::from("test"),
            trigger: None,
            tools_whitelist: Vec::new(),
            parameters: Vec::new(),
            steps: Vec::new(),
            content: "提示词含 ${unknown} 占位符".to_string(),
            enabled: true,
            source_url: None,
            installed_at: None,
            author: None,
            tags: None,
            min_navis_version: None,
        };

        let plan = service
            .build_activation_plan(&skill, HashMap::new(), "user")
            .unwrap();
        // 未定义的参数不替换
        assert!(plan.context_prompt.contains("${unknown}"));
    }

    #[test]
    fn test_validate_params_all_optional() {
        let service = SkillActivationService::new();
        let skill = SkillDefinition {
            id: "test:opt".to_string(),
            name: "opt".to_string(),
            description: "".to_string(),
            mode: SkillMode::Standard,
            version: "1.0.0".to_string(),
            source: SkillSource::Builtin,
            file_path: PathBuf::from("test"),
            trigger: None,
            tools_whitelist: Vec::new(),
            parameters: vec![SkillParameter {
                name: "opt_param".to_string(),
                description: "".to_string(),
                required: false,
                default: None,
                param_type: "string".to_string(),
            }],
            steps: Vec::new(),
            content: "content".to_string(),
            enabled: true,
            source_url: None,
            installed_at: None,
            author: None,
            tags: None,
            min_navis_version: None,
        };

        // 不提供可选参数应能通过
        let plan = service
            .build_activation_plan(&skill, HashMap::new(), "user")
            .unwrap();
        assert_eq!(plan.parameters.len(), 0);
    }
}
