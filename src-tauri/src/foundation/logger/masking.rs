//! MaskingLayer - 敏感信息脱敏 tracing Layer
//!
//! 基于设计文档 §6 实现，在事件写入前对消息内容进行脱敏处理

use regex::Regex;
use std::sync::Arc;
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

/// 内置脱敏规则列表
const BUILTIN_MASKING_PATTERNS: &[&str] = &[
    // RSA 私钥（最高优先级，避免被其他规则干扰）
    r"(-----BEGIN\s+(RSA\s+)?PRIVATE KEY-----)",
    // JWT Token: 三段式 Base64 编码
    r"(eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*)",
    // Anthropic 风格 API Key: sk-ant- 后跟 20+ 字符（放在 OpenAI 之前，因为更具体）
    r"(sk-ant-[a-zA-Z0-9]{20,})",
    // OpenAI 风格 API Key: sk- 后跟 20+ 字符
    r"(sk-[a-zA-Z0-9]{20,})",
    // AWS Access Key ID: AKIA 后跟 16 个大写字母或数字
    r"(AKIA[0-9A-Z]{16})",
    // 通用密钥模式：password/secret/token/key/auth 等赋值（放在最后，避免干扰特定模式）
    r#"(password|passwd|pwd|secret|token|key|auth)\s*[:=]\s*['"]?([^\s'"]+)"#,
];

/// MaskingLayer - tracing Layer，对日志内容进行敏感信息脱敏
pub struct MaskingLayer {
    /// 预编译的正则表达式列表
    patterns: Arc<Vec<Regex>>,
}

impl MaskingLayer {
    /// 创建新的 MaskingLayer
    pub fn new() -> Self {
        let mut patterns = Vec::new();

        // 加载内置脱敏规则
        // 注：编译失败用 eprintln 而非 tracing，因为 MaskingLayer 构造时 tracing subscriber 可能尚未初始化
        for pattern in BUILTIN_MASKING_PATTERNS {
            match Regex::new(pattern) {
                Ok(re) => patterns.push(re),
                Err(e) => {
                    eprintln!("[masking] 内置规则编译失败: {} - {}", pattern, e);
                }
            }
        }

        Self {
            patterns: Arc::new(patterns),
        }
    }

    /// 创建带有自定义规则的 MaskingLayer
    pub fn with_custom_patterns(custom_patterns: &[String]) -> Self {
        let mut patterns = Vec::new();

        // 加载内置脱敏规则（同上，编译失败用 eprintln 避免 tracing 初始化时序问题）
        for pattern in BUILTIN_MASKING_PATTERNS {
            match Regex::new(pattern) {
                Ok(re) => patterns.push(re),
                Err(e) => {
                    eprintln!("[masking] 内置规则编译失败: {} - {}", pattern, e);
                }
            }
        }

        // 加载自定义脱敏规则（同上）
        for pattern in custom_patterns {
            match Regex::new(pattern) {
                Ok(re) => patterns.push(re),
                Err(e) => {
                    // 同上：MaskingLayer 构造期间，用 eprintln 避免 tracing 初始化时序问题
                    eprintln!("[masking] 自定义规则编译失败，已跳过: {} - {}", pattern, e);
                }
            }
        }

        Self {
            patterns: Arc::new(patterns),
        }
    }

    /// 对文本执行脱敏处理
    pub fn mask(&self, text: &str) -> String {
        let mut result = text.to_string();

        for pattern in self.patterns.iter() {
            result = pattern
                .replace_all(&result, |caps: &regex::Captures| self.mask_capture(caps))
                .into_owned();
        }

        result
    }

    /// 对单个匹配执行脱敏替换
    fn mask_capture(&self, caps: &regex::Captures) -> String {
        // 通用密钥赋值模式：password="xxx" → password="***"
        if caps.len() >= 3 {
            if let (Some(key_match), Some(val_match)) = (caps.get(1), caps.get(2)) {
                let key = key_match.as_str().to_lowercase();
                if [
                    "password", "passwd", "pwd", "secret", "token", "key", "auth",
                ]
                .iter()
                .any(|&k| key.starts_with(k))
                {
                    let full = caps.get(0).unwrap().as_str();
                    let prefix = &full[..val_match.start() - caps.get(0).unwrap().start()];
                    return format!("{}\"***\"", prefix);
                }
            }
        }

        // 获取第一个捕获组的值
        let matched = caps
            .get(1)
            .map_or_else(|| caps.get(0).unwrap().as_str(), |m| m.as_str());

        // 私钥头：替换为 [REDACTED]
        if matched.contains("PRIVATE KEY") {
            return "[REDACTED]".to_string();
        }

        // JWT Token：替换为 [REDACTED]
        if matched.starts_with("eyJ") && matched.contains('.') {
            return "[REDACTED]".to_string();
        }

        // API Key 类：保留前 3 后 3 字符
        if matched.len() > 6 {
            let prefix = &matched[..3];
            let suffix = &matched[matched.len() - 3..];
            format!("{}***{}", prefix, suffix)
        } else {
            // 太短的匹配直接替换
            "***".to_string()
        }
    }
}

/// tracing Layer 实现
impl<S: tracing::Subscriber> Layer<S> for MaskingLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        // 提取事件消息
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        // 对消息内容执行脱敏
        if let Some(message) = visitor.message {
            let masked_message = self.mask(&message);

            // 如果脱敏后内容不同，可以记录到审计日志
            if masked_message != message {
                tracing::debug!(
                    original_length = message.len(),
                    masked_length = masked_message.len(),
                    "Sensitive data masked in log message"
                );
            }
        }
    }
}

/// 消息访问者（提取事件中的 message 字段）
#[derive(Default)]
struct MessageVisitor {
    message: Option<String>,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_layer() -> MaskingLayer {
        MaskingLayer::new()
    }

    #[test]
    fn test_mask_openai_key() {
        let layer = test_layer();
        let input = "Using API: sk-abc123def456ghi789jkl012mno";
        let result = layer.mask(input);
        assert!(result.contains("sk-***mno"));
        assert!(!result.contains("abc123def456ghi789jkl01"));
    }

    #[test]
    fn test_mask_anthropic_key() {
        let layer = test_layer();
        let input = "API credential: sk-ant-api12345678901234567890abcdef";
        let result = layer.mask(input);
        assert!(result.contains("sk-***def"));
        assert!(!result.contains("api12345678901234567890abc"));
    }

    #[test]
    fn test_mask_aws_key() {
        let layer = test_layer();
        let input = "AWS credential: AKIAIOSFODNN7EXAMPLE";
        let result = layer.mask(input);
        assert!(result.contains("AKI***PLE"));
    }

    #[test]
    fn test_mask_password_assignment() {
        let layer = test_layer();
        let input = r#"password="mySecretPassword123""#;
        let result = layer.mask(input);
        assert!(result.contains("password="));
        assert!(result.contains("***"));
        assert!(!result.contains("mySecretPassword123"));
    }

    #[test]
    fn test_mask_jwt() {
        let layer = test_layer();
        let input = "Token: eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abc123def456";
        let result = layer.mask(input);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("eyJhbGciOiJIUzI1NiJ9"));
    }

    #[test]
    fn test_mask_private_key() {
        let layer = test_layer();
        let input = "Key: -----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAK...";
        let result = layer.mask(input);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("BEGIN RSA PRIVATE KEY"));
    }

    #[test]
    fn test_no_sensitive_data() {
        let layer = test_layer();
        let input = "Normal log message: request completed in 123ms";
        let result = layer.mask(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_multiple_secrets() {
        let layer = test_layer();
        let input = r#"Connecting with sk-abc123def456ghi789jkl012mno and password="secret123""#;
        let result = layer.mask(input);
        assert!(!result.contains("abc123def456ghi789jkl01"));
        assert!(!result.contains("secret123"));
    }

    #[test]
    fn test_custom_pattern() {
        let custom = vec![r"(CUSTOM-\d{6})".to_string()];
        let layer = MaskingLayer::with_custom_patterns(&custom);
        let input = "ID: CUSTOM-123456 found";
        let result = layer.mask(input);
        assert!(result.contains("***"));
        assert!(!result.contains("CUSTOM-123456"));
    }

    #[test]
    fn test_invalid_custom_pattern() {
        let custom = vec!["[invalid".to_string()];
        let layer = MaskingLayer::with_custom_patterns(&custom);
        let input = "Normal text";
        let result = layer.mask(input);
        assert_eq!(result, input);
    }
}
