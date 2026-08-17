//! 日志导出模块
//!
//! 基于设计文档实现，支持将日志导出为 JSON/Text/CSV 格式

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use super::query::LogEntry;

/// 导出格式枚举
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// JSON 格式
    Json,
    /// 纯文本格式
    Text,
    /// CSV 格式
    Csv,
}

impl ExportFormat {
    /// 从字符串解析导出格式
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(ExportFormat::Json),
            "text" | "txt" => Some(ExportFormat::Text),
            "csv" => Some(ExportFormat::Csv),
            _ => None,
        }
    }
}

/// 导出错误类型
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}

/// 日志导出器
pub struct LogExporter;

impl LogExporter {
    /// 导出日志到文件
    ///
    /// # Arguments
    /// * `entries` - 待导出的日志条目列表
    /// * `format` - 导出格式
    /// * `path` - 输出文件路径
    pub fn export(
        entries: &[LogEntry],
        format: &ExportFormat,
        path: &Path,
    ) -> Result<(), ExportError> {
        // 确保输出目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = match format {
            ExportFormat::Json => Self::format_json(entries),
            ExportFormat::Text => Self::format_text(entries),
            ExportFormat::Csv => Self::format_csv(entries),
        };

        let mut file = fs::File::create(path)?;
        file.write_all(content.as_bytes())?;
        file.flush()?;

        Ok(())
    }

    /// 格式化为 JSON
    fn format_json(entries: &[LogEntry]) -> String {
        serde_json::to_string_pretty(entries).unwrap_or_else(|_| "[]".to_string())
    }

    /// 格式化为纯文本
    fn format_text(entries: &[LogEntry]) -> String {
        entries
            .iter()
            .map(|entry| {
                format!(
                    "{} [{}] [{}] {}",
                    entry
                        .timestamp
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    entry.level,
                    entry.target,
                    entry.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 格式化为 CSV
    fn format_csv(entries: &[LogEntry]) -> String {
        let mut output = String::new();

        // CSV 表头
        output.push_str("timestamp,level,target,message,session_id\n");

        for entry in entries {
            let row = format!(
                "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
                Self::csv_escape(
                    &entry
                        .timestamp
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
                ),
                entry.level,
                Self::csv_escape(&entry.target),
                Self::csv_escape(&entry.message),
                Self::csv_escape(entry.session_id.as_deref().unwrap_or(""))
            );
            output.push_str(&row);
        }

        output
    }

    /// CSV 字段转义
    fn csv_escape(s: &str) -> String {
        s.replace('"', "\"\"")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use std::collections::HashMap;

    /// 创建测试用的日志条目列表
    fn test_entries() -> Vec<LogEntry> {
        vec![
            LogEntry {
                timestamp: "2026-06-01T14:32:01.123Z".parse::<DateTime<Utc>>().unwrap(),
                level: "INFO".to_string(),
                target: "gateway".to_string(),
                message: "Request completed".to_string(),
                fields: HashMap::new(),
                session_id: Some("sess_001".to_string()),
            },
            LogEntry {
                timestamp: "2026-06-01T14:32:02.456Z".parse::<DateTime<Utc>>().unwrap(),
                level: "ERROR".to_string(),
                target: "agent".to_string(),
                message: "Tool call failed".to_string(),
                fields: HashMap::new(),
                session_id: Some("sess_001".to_string()),
            },
        ]
    }

    #[test]
    fn test_export_json() {
        let entries = test_entries();
        let output = LogExporter::format_json(&entries);
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 2);
        assert_eq!(parsed[0]["level"], "INFO");
        assert_eq!(parsed[1]["level"], "ERROR");
    }

    #[test]
    fn test_export_text() {
        let entries = test_entries();
        let output = LogExporter::format_text(&entries);
        assert!(output.contains("[INFO] [gateway] Request completed"));
        assert!(output.contains("[ERROR] [agent] Tool call failed"));
    }

    #[test]
    fn test_export_csv() {
        let entries = test_entries();
        let output = LogExporter::format_csv(&entries);
        let lines: Vec<&str> = output.lines().collect();
        // 表头 + 2 行数据
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("timestamp"));
        assert!(lines[0].contains("level"));
        assert!(lines[1].contains("INFO"));
        assert!(lines[1].contains("gateway"));
        assert!(lines[2].contains("ERROR"));
        assert!(lines[2].contains("agent"));
    }

    #[test]
    fn test_csv_escape() {
        assert_eq!(LogExporter::csv_escape("hello"), "hello");
        assert_eq!(LogExporter::csv_escape("say \"hi\""), "say \"\"hi\"\"");
        assert_eq!(LogExporter::csv_escape(""), "");
    }

    #[test]
    fn test_export_format_parse() {
        assert_eq!(ExportFormat::from_str("json"), Some(ExportFormat::Json));
        assert_eq!(ExportFormat::from_str("text"), Some(ExportFormat::Text));
        assert_eq!(ExportFormat::from_str("txt"), Some(ExportFormat::Text));
        assert_eq!(ExportFormat::from_str("csv"), Some(ExportFormat::Csv));
        assert_eq!(ExportFormat::from_str("JSON"), Some(ExportFormat::Json));
        assert_eq!(ExportFormat::from_str("xml"), None);
    }

    #[test]
    fn test_export_empty() {
        let entries: Vec<LogEntry> = vec![];
        assert_eq!(LogExporter::format_json(&entries), "[]");
        assert_eq!(LogExporter::format_text(&entries), "");
        let csv = LogExporter::format_csv(&entries);
        assert!(csv.contains("timestamp")); // 至少有表头
    }
}
