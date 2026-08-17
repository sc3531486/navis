//! SKILL.md 解析器
//!
//! 解析 SKILL.md 文件的 YAML frontmatter 和 Markdown 正文。
//!
//! SKILL.md 格式：
//! ```markdown
//! ---
//! name: code-review
//! description: 代码审查
//! mode: standard
//! trigger: /review
//! tools: [read, lsp.diagnostics, lsp.references]
//! parameters:
//!   - name: focus
//!     description: 审查重点
//!     required: false
//!     default: "all"
//! ---
//!
//! 你是一个资深代码审查员。
//! ```

use super::{OnFailureAction, SkillMode, SkillParameter, SkillStep};

/// 解析结果
#[derive(Debug, Clone)]
pub struct ParsedSkill {
    /// 名称
    pub name: Option<String>,
    /// 描述
    pub description: Option<String>,
    /// 模式
    pub mode: Option<SkillMode>,
    /// 版本
    pub version: Option<String>,
    /// 触发命令
    pub trigger: Option<String>,
    /// 工具白名单
    pub tools: Option<Vec<String>>,
    /// 参数定义
    pub parameters: Option<Vec<SkillParameter>>,
    /// 步骤定义（增强模式）
    pub steps: Option<Vec<SkillStep>>,
    /// 来源 URL
    pub source_url: Option<String>,
    /// 作者
    pub author: Option<String>,
    /// 标签
    pub tags: Option<Vec<String>>,
    /// 最低 Navis 版本要求
    pub min_navis_version: Option<String>,
    /// Markdown 正文
    pub content: String,
}

/// SKILL.md 解析器
pub struct SkillParser;

impl SkillParser {
    /// 创建新的解析器
    pub fn new() -> Self {
        Self
    }

    /// 解析 SKILL.md 内容
    ///
    /// 分离 YAML frontmatter 和 Markdown 正文
    pub fn parse(&self, content: &str) -> Result<ParsedSkill, String> {
        let (frontmatter, body) = self.split_frontmatter(content)?;

        if let Some(fm) = frontmatter {
            self.parse_frontmatter(&fm, body)
        } else {
            // 没有 frontmatter，整个内容作为正文
            Ok(ParsedSkill {
                name: None,
                description: None,
                mode: None,
                version: None,
                trigger: None,
                tools: None,
                parameters: None,
                steps: None,
                source_url: None,
                author: None,
                tags: None,
                min_navis_version: None,
                content: content.to_string(),
            })
        }
    }

    /// 分离 YAML frontmatter 和 Markdown 正文
    ///
    /// frontmatter 由 `---` 分隔符界定
    fn split_frontmatter<'a>(
        &self,
        content: &'a str,
    ) -> Result<(Option<&'a str>, &'a str), String> {
        let trimmed = content.trim_start();

        if !trimmed.starts_with("---") {
            return Ok((None, content));
        }

        // 找到第二个 ---
        let after_first = &trimmed[3..];
        if let Some(end_idx) = after_first.find("\n---") {
            let fm = after_first[..end_idx].trim();
            let body_start = end_idx + 4; // \n--- 的长度
            let body = if body_start < after_first.len() {
                after_first[body_start..]
                    .trim_start_matches('\n')
                    .trim_start()
            } else {
                ""
            };
            Ok((Some(fm), body))
        } else {
            Err("Missing closing '---' in frontmatter".to_string())
        }
    }

    /// 解析 YAML frontmatter
    fn parse_frontmatter(&self, fm: &str, body: &str) -> Result<ParsedSkill, String> {
        let yaml: serde_yaml::Value = serde_yaml::from_str(fm)
            .map_err(|e| format!("Failed to parse YAML frontmatter: {}", e))?;

        let map = match yaml {
            serde_yaml::Value::Mapping(m) => m,
            _ => return Err("Frontmatter must be a YAML mapping".to_string()),
        };

        let name = self.get_string(&map, "name");
        let description = self.get_string(&map, "description");
        let trigger = self.get_string(&map, "trigger");
        let version = self
            .get_string(&map, "version")
            .or_else(|| Some("1.0.0".to_string()));

        let mode = self
            .get_string(&map, "mode")
            .and_then(|s| SkillMode::from_str(&s));

        let tools = self.get_string_list(&map, "tools");
        let parameters = self.get_parameters(&map);
        let steps = self.get_steps(&map);

        // 扩展元数据字段（全部可选）
        let source_url = self.get_string(&map, "source_url");
        let author = self.get_string(&map, "author");
        let tags = self.get_string_list(&map, "tags");
        let min_navis_version = self.get_string(&map, "min_navis_version");

        Ok(ParsedSkill {
            name,
            description,
            mode,
            version,
            trigger,
            tools,
            parameters,
            steps,
            source_url,
            author,
            tags,
            min_navis_version,
            content: body.to_string(),
        })
    }

    /// 从 YAML mapping 获取字符串值
    fn get_string(&self, map: &serde_yaml::Mapping, key: &str) -> Option<String> {
        map.get(&serde_yaml::Value::String(key.to_string()))
            .and_then(|v| match v {
                serde_yaml::Value::String(s) => Some(s.clone()),
                _ => None,
            })
    }

    /// 从 YAML mapping 获取字符串列表
    fn get_string_list(&self, map: &serde_yaml::Mapping, key: &str) -> Option<Vec<String>> {
        map.get(&serde_yaml::Value::String(key.to_string()))
            .and_then(|v| match v {
                serde_yaml::Value::Sequence(seq) => {
                    let items: Vec<String> = seq
                        .iter()
                        .filter_map(|item| match item {
                            serde_yaml::Value::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .collect();
                    Some(items)
                }
                _ => None,
            })
    }

    /// 解析参数定义列表
    fn get_parameters(&self, map: &serde_yaml::Mapping) -> Option<Vec<SkillParameter>> {
        map.get(&serde_yaml::Value::String("parameters".to_string()))
            .and_then(|v| match v {
                serde_yaml::Value::Sequence(seq) => {
                    let params: Vec<SkillParameter> = seq
                        .iter()
                        .filter_map(|item| self.parse_parameter(item))
                        .collect();
                    if params.is_empty() {
                        None
                    } else {
                        Some(params)
                    }
                }
                _ => None,
            })
    }

    /// 解析单个参数
    fn parse_parameter(&self, value: &serde_yaml::Value) -> Option<SkillParameter> {
        match value {
            serde_yaml::Value::Mapping(map) => {
                let name = self.get_string(map, "name")?;
                let description = self.get_string(map, "description").unwrap_or_default();
                let required = map
                    .get(&serde_yaml::Value::String("required".to_string()))
                    .and_then(|v| match v {
                        serde_yaml::Value::Bool(b) => Some(*b),
                        _ => None,
                    })
                    .unwrap_or(false);
                let default = self.get_string(map, "default");
                let param_type = self
                    .get_string(map, "type")
                    .unwrap_or_else(|| "string".to_string());

                Some(SkillParameter {
                    name,
                    description,
                    required,
                    default,
                    param_type,
                })
            }
            _ => None,
        }
    }

    /// 解析步骤定义列表（增强模式）
    fn get_steps(&self, map: &serde_yaml::Mapping) -> Option<Vec<SkillStep>> {
        map.get(&serde_yaml::Value::String("steps".to_string()))
            .and_then(|v| match v {
                serde_yaml::Value::Sequence(seq) => {
                    let steps: Vec<SkillStep> = seq
                        .iter()
                        .filter_map(|item| self.parse_step(item))
                        .collect();
                    if steps.is_empty() {
                        None
                    } else {
                        Some(steps)
                    }
                }
                _ => None,
            })
    }

    /// 解析单个步骤
    fn parse_step(&self, value: &serde_yaml::Value) -> Option<SkillStep> {
        match value {
            serde_yaml::Value::Mapping(map) => {
                let name = self.get_string(map, "name")?;
                let description = self.get_string(map, "description").unwrap_or_default();
                let prompt = self.get_string(map, "prompt").unwrap_or_default();
                let tools = self.get_string_list(map, "tools").unwrap_or_default();
                let depends_on = self.get_string_list(map, "depends_on").unwrap_or_default();
                let condition = self.get_string(map, "condition");
                let on_failure = self
                    .get_string(map, "on_failure")
                    .and_then(|s| OnFailureAction::from_str(&s))
                    .unwrap_or(OnFailureAction::Fail);
                let max_retries = map
                    .get(&serde_yaml::Value::String("max_retries".to_string()))
                    .and_then(|v| match v {
                        serde_yaml::Value::Number(n) => n.as_u64().map(|n| n as u32),
                        _ => None,
                    })
                    .unwrap_or(0);
                let timeout_secs = map
                    .get(&serde_yaml::Value::String("timeout".to_string()))
                    .and_then(|v| match v {
                        serde_yaml::Value::Number(n) => n.as_u64(),
                        _ => None,
                    });

                Some(SkillStep {
                    name,
                    description,
                    prompt,
                    tools,
                    depends_on,
                    condition,
                    on_failure,
                    max_retries,
                    timeout_secs,
                })
            }
            _ => None,
        }
    }
}

impl Default for SkillParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_skill_md() {
        let content = r#"---
name: code-review
description: 代码审查
mode: standard
trigger: /review
tools: [read, lsp.diagnostics, lsp.references]
parameters:
  - name: focus
    description: 审查重点
    required: false
    default: "all"
  - name: severity
    description: 最低严重程度
    required: true
---

你是一个资深代码审查员。

## 行为规范
- 审查代码时关注：安全性、性能、可读性、可维护性
"#;

        let parser = SkillParser::new();
        let result = parser.parse(content).unwrap();

        assert_eq!(result.name, Some("code-review".to_string()));
        assert_eq!(result.description, Some("代码审查".to_string()));
        assert_eq!(result.mode, Some(SkillMode::Standard));
        assert_eq!(result.trigger, Some("/review".to_string()));
        assert!(result.tools.is_some());
        let tools = result.tools.unwrap();
        assert_eq!(tools.len(), 3);
        assert!(tools.contains(&"read".to_string()));

        let params = result.parameters.unwrap();
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "focus");
        assert!(!params[0].required);
        assert_eq!(params[0].default, Some("all".to_string()));
        assert_eq!(params[1].name, "severity");
        assert!(params[1].required);

        assert!(result.content.contains("你是一个资深代码审查员"));
    }

    #[test]
    fn test_parse_no_frontmatter() {
        let content = "这是一个简单的提示词模板，没有 frontmatter。";
        let parser = SkillParser::new();
        let result = parser.parse(content).unwrap();

        assert!(result.name.is_none());
        assert!(result.content.contains("这是一个简单的提示词模板"));
    }

    #[test]
    fn test_parse_missing_closing_delimiter() {
        let content = r#"---
name: test
description: 测试

正文内容"#;

        let parser = SkillParser::new();
        let result = parser.parse(content);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing closing"));
    }

    #[test]
    fn test_parse_minimal_frontmatter() {
        let content = r#"---
name: minimal
---

正文内容"#;

        let parser = SkillParser::new();
        let result = parser.parse(content).unwrap();
        assert_eq!(result.name, Some("minimal".to_string()));
        assert_eq!(result.version, Some("1.0.0".to_string()));
        assert_eq!(result.mode, None);
        assert_eq!(result.trigger, None);
    }

    #[test]
    fn test_parse_enhanced_mode_with_steps() {
        let content = r#"---
name: multi-step-review
description: 多步骤审查
mode: enhanced
steps:
  - name: analyze
    description: 分析代码结构
    prompt: 分析代码的整体结构
    tools: [read]
    on_failure: retry
    max_retries: 3
    timeout: 60
  - name: report
    description: 生成报告
    prompt: 生成审查报告
    depends_on: [analyze]
    on_failure: fail
---

多步骤代码审查流程
"#;

        let parser = SkillParser::new();
        let result = parser.parse(content).unwrap();

        assert_eq!(result.mode, Some(SkillMode::Enhanced));
        let steps = result.steps.unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].name, "analyze");
        assert_eq!(steps[0].on_failure, OnFailureAction::Retry);
        assert_eq!(steps[0].max_retries, 3);
        assert_eq!(steps[0].timeout_secs, Some(60));
        assert_eq!(steps[1].name, "report");
        assert_eq!(steps[1].depends_on, vec!["analyze".to_string()]);
        assert_eq!(steps[1].on_failure, OnFailureAction::Fail);
    }

    #[test]
    fn test_split_frontmatter() {
        let content = "---\nname: test\n---\nBody";
        let parser = SkillParser::new();
        let (fm, body) = parser.split_frontmatter(content).unwrap();
        assert_eq!(fm, Some("name: test"));
        assert_eq!(body, "Body");
    }

    #[test]
    fn test_split_no_frontmatter() {
        let content = "No frontmatter here";
        let parser = SkillParser::new();
        let (fm, body) = parser.split_frontmatter(content).unwrap();
        assert!(fm.is_none());
        assert_eq!(body, "No frontmatter here");
    }

    #[test]
    fn test_parse_parameter_with_type() {
        let content = r#"---
name: typed-params
parameters:
  - name: count
    description: 数量
    required: true
    type: number
  - name: verbose
    description: 详细输出
    type: boolean
---

带类型参数的 Skill
"#;

        let parser = SkillParser::new();
        let result = parser.parse(content).unwrap();
        let params = result.parameters.unwrap();
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].param_type, "number");
        assert_eq!(params[1].param_type, "boolean");
    }
}
