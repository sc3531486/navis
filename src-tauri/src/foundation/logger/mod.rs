//! Logger 日志系统模块
//!
//! 基于 Rust tracing 生态构建，为所有模块提供统一的日志记录、分级、轮转、脱敏、导出能力。
//!
//! # 技术选型
//! - 基础框架：tracing + tracing-subscriber + tracing-appender
//! - 自研扩展：4 个自定义 Layer（Masking/Audit/Query/Export）
//!
//! `AuditLayer` 是 tracing 观察日志通道，只把 `audit=true` 事件写入独立日志文件；
//! 结构化业务审计事实源统一走 `kernel::AuditRecorder` / `AuditSink`。
//!
//! # 子模块
//! - masking.rs - MaskingLayer（敏感信息脱敏）
//! - audit.rs - AuditLayer（tracing 审计观察日志通道）
//! - query.rs - QueryLayer（前端查询）
//! - export.rs - 日志导出（JSON/Text/CSV）

pub mod audit;
pub mod export;
pub mod masking;
pub mod query;

pub use audit::AuditLayer;
pub use export::{ExportError, ExportFormat};
pub use masking::MaskingLayer;
pub use query::QueryLayer;
pub use query::{LogEntry, LogFilter};

use anyhow::Result;
use std::path::PathBuf;
use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// 日志配置
#[derive(Debug, Clone)]
pub struct LoggerConfig {
    /// 全局过滤级别（默认 "info"）
    /// 支持："info", "gateway=debug,agent=trace"
    pub level: String,

    /// 是否输出到控制台（默认 true）
    pub console_enabled: bool,

    /// 是否输出到文件（默认 true）
    pub file_enabled: bool,

    /// 日志目录（默认 ~/.navis/logs/）
    pub file_dir: PathBuf,

    /// 文件名前缀（默认 "navis"）
    pub file_prefix: String,

    /// 轮转策略（默认 Daily）
    pub rotation: RotationStrategy,

    /// 是否启用脱敏（默认 true）
    pub masking_enabled: bool,

    /// 是否启用 tracing 审计观察日志通道（默认 true）
    pub audit_enabled: bool,

    /// tracing 审计观察日志目录（默认 ~/.navis/logs/audit/）
    pub audit_dir: PathBuf,
}

/// 轮转策略
#[derive(Debug, Clone, Copy)]
pub enum RotationStrategy {
    /// 按天轮转
    Daily,
    /// 按小时轮转
    Hourly,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let navis_dir = home_dir.join(".navis");

        Self {
            level: "info".to_string(),
            console_enabled: true,
            file_enabled: true,
            file_dir: navis_dir.join("logs"),
            file_prefix: "navis".to_string(),
            rotation: RotationStrategy::Daily,
            masking_enabled: true,
            audit_enabled: true,
            audit_dir: navis_dir.join("logs").join("audit"),
        }
    }
}

/// 初始化日志系统
///
/// # Arguments
/// * `config` - 日志配置
///
/// # Returns
/// 返回 QueryLayer 的引用，供前端查询使用
pub fn init(config: LoggerConfig) -> Result<QueryLayer> {
    // 1. 创建过滤器
    let env_filter = EnvFilter::try_new(&config.level).unwrap_or_else(|_| EnvFilter::new("info"));

    // 2. 创建格式化 Layer（控制台输出）
    let fmt_layer = if config.console_enabled {
        Some(
            fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true),
        )
    } else {
        None
    };

    // 3. 创建文件 Layer
    let file_layer = if config.file_enabled {
        // 确保日志目录存在
        std::fs::create_dir_all(&config.file_dir)?;

        let file_appender = match config.rotation {
            RotationStrategy::Daily => {
                tracing_appender::rolling::daily(&config.file_dir, &config.file_prefix)
            }
            RotationStrategy::Hourly => {
                tracing_appender::rolling::hourly(&config.file_dir, &config.file_prefix)
            }
        };

        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        Some(fmt::layer().json().with_writer(non_blocking))
    } else {
        None
    };

    // 4. 创建自研 Layers
    let masking_layer = if config.masking_enabled {
        Some(MaskingLayer::new())
    } else {
        None
    };

    let audit_layer = if config.audit_enabled {
        Some(AuditLayer::new(&config.audit_dir)?)
    } else {
        None
    };

    let query_layer = QueryLayer::new(1000); // 环形缓冲区 1000 条

    // 5. 叠加所有 Layers
    let subscriber = tracing_subscriber::registry().with(env_filter);

    // 动态添加 Layer（使用 Option 处理可选 Layer）
    let subscriber = subscriber.with(fmt_layer);

    let subscriber = subscriber.with(file_layer);

    let subscriber = subscriber.with(masking_layer);

    let subscriber = subscriber.with(audit_layer);

    let subscriber = subscriber.with(query_layer.clone());

    // 6. 初始化全局订阅者
    subscriber.init();

    Ok(query_layer)
}

/// 清理旧日志文件
///
/// # Arguments
/// * `log_dir` - 日志目录
/// * `max_days` - 保留天数
///
/// # Returns
/// 返回清理的文件数量
pub fn cleanup(log_dir: &std::path::Path, max_days: u32) -> Result<u64> {
    let mut cleaned_count = 0;

    if !log_dir.exists() {
        return Ok(0);
    }

    let now = chrono::Utc::now();
    let threshold = now - chrono::Duration::days(max_days as i64);

    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            // 检查文件修改时间
            if let Ok(metadata) = path.metadata() {
                if let Ok(modified) = metadata.modified() {
                    let modified_time: chrono::DateTime<chrono::Utc> = modified.into();

                    if modified_time < threshold {
                        std::fs::remove_file(&path)?;
                        cleaned_count += 1;
                    }
                }
            }
        }
    }

    Ok(cleaned_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_logger_config_default() {
        let config = LoggerConfig::default();
        assert_eq!(config.level, "info");
        assert!(config.console_enabled);
        assert!(config.file_enabled);
        assert_eq!(config.file_prefix, "navis");
        assert!(config.masking_enabled);
        assert!(config.audit_enabled);
    }

    #[test]
    fn test_cleanup() {
        let temp_dir = tempdir().unwrap();
        let log_dir = temp_dir.path().join("logs");
        std::fs::create_dir_all(&log_dir).unwrap();

        // 创建测试文件
        let file1 = log_dir.join("test1.log");
        let file2 = log_dir.join("test2.log");
        std::fs::write(&file1, "test").unwrap();
        std::fs::write(&file2, "test").unwrap();

        // 清理（保留 0 天，应该删除所有文件）
        let cleaned = cleanup(&log_dir, 0).unwrap();
        assert_eq!(cleaned, 2);
        assert!(!file1.exists());
        assert!(!file2.exists());
    }
}
