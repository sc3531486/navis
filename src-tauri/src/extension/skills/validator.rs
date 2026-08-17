//! Skill 格式校验器
//!
//! 校验 SKILL.md 文件格式的正确性，包括：
//! - YAML frontmatter 完整性
//! - 必填字段检查
//! - 模式合法性
//! - 参数定义完整性
//! - 增强模式步骤依赖检查

use std::path::Path;

use super::parser::SkillParser;
use super::{OnFailureAction, SkillMode, SkillStep};

/// 校验结果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// 是否通过
    pub valid: bool,
    /// 错误列表
    pub errors: Vec<String>,
    /// 警告列表
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// 创建通过结果
    pub fn ok() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// 创建失败结果
    pub fn fail(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    /// 添加警告
    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings = warnings;
        self
    }
}

/// Skill 格式校验器
pub struct SkillValidator {
    parser: SkillParser,
}

impl SkillValidator {
    /// 创建新的校验器
    pub fn new() -> Self {
        Self {
            parser: SkillParser::new(),
        }
    }

    /// 校验 SKILL.md 文件路径
    pub fn validate_file(&self, path: &Path) -> ValidationResult {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                return ValidationResult::fail(vec![format!(
                    "Failed to read file '{}': {}",
                    path.display(),
                    e
                )]);
            }
        };

        self.validate_content(&content)
    }

    /// 校验 SKILL.md 内容
    pub fn validate_content(&self, content: &str) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // 1. 解析 frontmatter
        let parsed = match self.parser.parse(content) {
            Ok(p) => p,
            Err(e) => {
                return ValidationResult::fail(vec![format!("Parse error: {}", e)]);
            }
        };

        // 2. 检查名称
        if parsed.name.is_none() {
            warnings.push("Missing 'name' field in frontmatter, will use filename".to_string());
        } else {
            let name = parsed.name.as_ref().unwrap();
            if name.is_empty() {
                errors.push("'name' field cannot be empty".to_string());
            }
        }

        // 3. 检查描述
        if parsed.description.is_none()
            || parsed.description.as_ref().map_or(true, |d| d.is_empty())
        {
            warnings.push("Missing 'description' field".to_string());
        }

        // 4. 校验模式
        let mode = parsed.mode.clone().unwrap_or(SkillMode::Standard);
        match mode {
            SkillMode::Standard => {
                // 标准模式不需要额外校验
            }
            SkillMode::Enhanced => {
                // 增强模式必须有步骤定义
                match &parsed.steps {
                    Some(steps) if !steps.is_empty() => {
                        self.validate_steps(steps, &mut errors);
                    }
                    _ => {
                        errors.push(
                            "Enhanced mode requires at least one step definition".to_string(),
                        );
                    }
                }
            }
        }

        // 5. 校验触发命令格式
        if let Some(ref trigger) = parsed.trigger {
            if !trigger.starts_with('/') {
                errors.push(format!("Trigger must start with '/': '{}'", trigger));
            }
            if trigger.len() < 2 {
                errors.push("Trigger must have a name after '/'".to_string());
            }
        }

        // 6. 校验参数定义
        if let Some(ref params) = parsed.parameters {
            for param in params {
                if param.name.is_empty() {
                    errors.push("Parameter name cannot be empty".to_string());
                }
                if param.required && param.default.is_some() {
                    warnings.push(format!(
                        "Parameter '{}' is required but has a default value",
                        param.name
                    ));
                }
            }
        }

        // 7. 检查正文内容
        if parsed.content.trim().is_empty() {
            warnings.push("Skill content (body) is empty".to_string());
        }

        if errors.is_empty() {
            ValidationResult::ok().with_warnings(warnings)
        } else {
            ValidationResult::fail(errors).with_warnings(warnings)
        }
    }

    /// 校验增强模式步骤
    pub(crate) fn validate_steps(&self, steps: &[SkillStep], errors: &mut Vec<String>) {
        let step_names: Vec<&str> = steps.iter().map(|s| s.name.as_str()).collect();

        // 检查步骤名称唯一性
        let mut seen = std::collections::HashSet::new();
        for name in &step_names {
            if !seen.insert(name) {
                errors.push(format!("Duplicate step name: '{}'", name));
            }
        }

        for step in steps {
            // 检查步骤名称非空
            if step.name.is_empty() {
                errors.push("Step name cannot be empty".to_string());
            }

            // 检查提示词非空
            if step.prompt.is_empty() {
                errors.push(format!("Step '{}' prompt cannot be empty", step.name));
            }

            // 检查依赖步骤存在
            for dep in &step.depends_on {
                if !step_names.contains(&dep.as_str()) {
                    errors.push(format!(
                        "Step '{}' depends on non-existent step '{}'",
                        step.name, dep
                    ));
                }
            }

            // 自引用检查
            if step.depends_on.contains(&step.name) {
                errors.push(format!("Step '{}' cannot depend on itself", step.name));
            }

            // Retry 策略必须有 max_retries > 0
            if step.on_failure == OnFailureAction::Retry && step.max_retries == 0 {
                errors.push(format!(
                    "Step '{}' has on_failure=Retry but max_retries=0",
                    step.name
                ));
            }
        }

        // 检测循环依赖
        if self.has_circular_dependency(steps) {
            errors.push("Circular dependency detected in steps".to_string());
        }
    }

    /// 检测步骤间的循环依赖（拓扑排序）
    fn has_circular_dependency(&self, steps: &[SkillStep]) -> bool {
        use std::collections::{HashMap, HashSet, VecDeque};

        let n = steps.len();
        if n == 0 {
            return false;
        }

        // 构建邻接表和入度
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();

        for step in steps {
            in_degree.entry(&step.name).or_insert(0);
            graph.entry(&step.name).or_default();
            for dep in &step.depends_on {
                graph.entry(dep.as_str()).or_default().push(&step.name);
                *in_degree.entry(&step.name).or_insert(0) += 1;
            }
        }

        // Kahn 算法
        let mut queue: VecDeque<&str> = VecDeque::new();
        for (&name, &deg) in &in_degree {
            if deg == 0 {
                queue.push_back(name);
            }
        }

        let mut visited = HashSet::new();
        while let Some(node) = queue.pop_front() {
            if !visited.insert(node) {
                continue;
            }
            if let Some(neighbors) = graph.get(node) {
                for &neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }

        visited.len() != n
    }
}

impl Default for SkillValidator {
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
    fn test_validate_valid_skill() {
        let content = r#"---
name: valid-skill
description: 有效的 Skill
mode: standard
trigger: /valid
---

提示词内容
"#;

        let validator = SkillValidator::new();
        let result = validator.validate_content(content);
        assert!(
            result.valid,
            "Expected valid, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_validate_missing_name() {
        let content = r#"---
description: 没有名称
mode: standard
---

内容
"#;

        let validator = SkillValidator::new();
        let result = validator.validate_content(content);
        // 缺少 name 只是警告，不是错误
        assert!(result.valid);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_validate_invalid_trigger() {
        let content = r#"---
name: bad-trigger
description: 无效触发
trigger: no-slash
---

内容
"#;

        let validator = SkillValidator::new();
        let result = validator.validate_content(content);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("must start with '/'")));
    }

    #[test]
    fn test_validate_enhanced_no_steps() {
        let content = r#"---
name: no-steps
description: 缺少步骤
mode: enhanced
---

内容
"#;

        let validator = SkillValidator::new();
        let result = validator.validate_content(content);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("at least one step")));
    }

    #[test]
    fn test_validate_enhanced_with_valid_steps() {
        let content = r#"---
name: valid-enhanced
description: 有效的增强模式
mode: enhanced
steps:
  - name: step1
    description: 第一步
    prompt: 执行第一步
    on_failure: fail
  - name: step2
    description: 第二步
    prompt: 执行第二步
    depends_on: [step1]
    on_failure: retry
    max_retries: 3
---

增强模式内容
"#;

        let validator = SkillValidator::new();
        let result = validator.validate_content(content);
        assert!(
            result.valid,
            "Expected valid, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_validate_duplicate_step_names() {
        let content = r#"---
name: dup-steps
description: 重复步骤名
mode: enhanced
steps:
  - name: step1
    prompt: 第一步
    on_failure: fail
  - name: step1
    prompt: 重复步骤
    on_failure: fail
---

内容
"#;

        let validator = SkillValidator::new();
        let result = validator.validate_content(content);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("Duplicate step name")));
    }

    #[test]
    fn test_validate_nonexistent_dependency() {
        let content = r#"---
name: bad-dep
description: 不存在的依赖
mode: enhanced
steps:
  - name: step1
    prompt: 第一步
    depends_on: [nonexistent]
    on_failure: fail
---

内容
"#;

        let validator = SkillValidator::new();
        let result = validator.validate_content(content);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("non-existent step")));
    }

    #[test]
    fn test_validate_self_dependency() {
        let content = r#"---
name: self-dep
description: 自引用
mode: enhanced
steps:
  - name: step1
    prompt: 第一步
    depends_on: [step1]
    on_failure: fail
---

内容
"#;

        let validator = SkillValidator::new();
        let result = validator.validate_content(content);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("cannot depend on itself")));
    }

    #[test]
    fn test_validate_circular_dependency() {
        let content = r#"---
name: circular
description: 循环依赖
mode: enhanced
steps:
  - name: step1
    prompt: 第一步
    depends_on: [step2]
    on_failure: fail
  - name: step2
    prompt: 第二步
    depends_on: [step1]
    on_failure: fail
---

内容
"#;

        let validator = SkillValidator::new();
        let result = validator.validate_content(content);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("Circular dependency")));
    }

    #[test]
    fn test_validate_retry_without_retries() {
        let content = r#"---
name: bad-retry
description: 无重试次数
mode: enhanced
steps:
  - name: step1
    prompt: 第一步
    on_failure: retry
    max_retries: 0
---

内容
"#;

        let validator = SkillValidator::new();
        let result = validator.validate_content(content);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("max_retries=0")));
    }

    #[test]
    fn test_validate_file() {
        let tmp_dir = TempDir::new().unwrap();
        let file_path = tmp_dir.path().join("test.md");
        fs::write(
            &file_path,
            r#"---
name: file-test
description: 文件校验
---

内容
"#,
        )
        .unwrap();

        let validator = SkillValidator::new();
        let result = validator.validate_file(&file_path);
        assert!(result.valid);
    }

    #[test]
    fn test_validate_file_not_found() {
        let validator = SkillValidator::new();
        let result = validator.validate_file(Path::new("/nonexistent/file.md"));
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("Failed to read file")));
    }

    #[test]
    fn test_validate_empty_content() {
        let content = "";
        let validator = SkillValidator::new();
        let result = validator.validate_content(content);
        // 空内容无 frontmatter，解析为纯文本，合法但有警告
        assert!(result.valid);
    }

    #[test]
    fn test_validate_malformed_yaml() {
        let content = r#"---
name: [invalid
yaml: {broken
---

内容
"#;

        let validator = SkillValidator::new();
        let result = validator.validate_content(content);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("Parse error")));
    }

    #[test]
    fn test_validate_param_required_with_default() {
        let content = r#"---
name: param-warning
description: 参数警告
parameters:
  - name: mode
    description: 模式
    required: true
    default: "auto"
---

内容
"#;

        let validator = SkillValidator::new();
        let result = validator.validate_content(content);
        // 合法但有警告
        assert!(result.valid);
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("required") && w.contains("default")));
    }
}
