//! AuditLayer - tracing 审计观察日志通道
//!
//! 基于设计文档 §7 实现，将标记为 `audit=true` 的 tracing 事件独立写入日志文件。
//! 该 Layer 只服务实时观察与日志落盘，不是结构化业务审计事实源；
//! 结构化事实统一由 `kernel::AuditRecorder` / `AuditSink` 写入。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

/// tracing 审计日志操作类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    /// 文件读取
    FileRead,
    /// 文件写入
    FileWrite,
    /// 文件删除
    FileDelete,
    /// 命令执行
    CommandExecute,
    /// 网络请求
    NetworkRequest,
    /// 密钥访问
    KeyAccess,
    /// Sandbox 绕过
    SandboxBypass,
}

impl fmt::Display for AuditAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditAction::FileRead => write!(f, "FileRead"),
            AuditAction::FileWrite => write!(f, "FileWrite"),
            AuditAction::FileDelete => write!(f, "FileDelete"),
            AuditAction::CommandExecute => write!(f, "CommandExecute"),
            AuditAction::NetworkRequest => write!(f, "NetworkRequest"),
            AuditAction::KeyAccess => write!(f, "KeyAccess"),
            AuditAction::SandboxBypass => write!(f, "SandboxBypass"),
        }
    }
}

/// tracing 审计日志操作结果
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditResult {
    /// 操作被允许执行
    Allowed,
    /// 操作被拒绝
    Denied,
    /// 操作执行失败
    Failed,
}

impl fmt::Display for AuditResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditResult::Allowed => write!(f, "allowed"),
            AuditResult::Denied => write!(f, "denied"),
            AuditResult::Failed => write!(f, "failed"),
        }
    }
}

/// tracing 审计观察日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// 操作发生的时间戳（UTC，精确到毫秒）
    pub timestamp: DateTime<Utc>,
    /// 操作类型
    pub action: AuditAction,
    /// 操作者标识（如 "user"、"agent"、"extension:xxx"）
    pub actor: String,
    /// 操作目标（文件路径、命令、密钥名称等）
    pub target: String,
    /// 操作结果
    pub result: AuditResult,
    /// 原因说明（可选，如被 Sandbox 拒绝的具体原因）
    pub reason: Option<String>,
    /// 关联会话 ID（可选）
    pub session_id: Option<String>,
}

impl AuditEntry {
    /// 创建新的 tracing 审计观察日志条目
    pub fn new(
        action: AuditAction,
        actor: impl Into<String>,
        target: impl Into<String>,
        result: AuditResult,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            action,
            actor: actor.into(),
            target: target.into(),
            result,
            reason: None,
            session_id: None,
        }
    }

    /// 设置原因说明
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// 设置关联会话 ID
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// 格式化为 tracing 审计观察日志文本行
    pub fn format_line(&self) -> String {
        let mut parts = vec![
            format!("actor={}", self.actor),
            format!("action={}", self.action),
            format!("target=\"{}\"", self.target),
            format!("result={}", self.result),
        ];

        if let Some(ref session_id) = self.session_id {
            parts.push(format!("session={}", session_id));
        }

        if let Some(ref reason) = self.reason {
            parts.push(format!("reason=\"{}\"", reason));
        }

        format!(
            "[{}] [AUDIT] {}",
            self.timestamp
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            parts.join(" ")
        )
    }
}

/// AuditLayer - tracing Layer，将标记为审计观察的事件独立写入日志文件。
///
/// 这不是 `kernel::AuditSink`，也不产生可查询的结构化业务事实源。
pub struct AuditLayer {
    /// tracing 审计观察日志目录
    audit_dir: PathBuf,
    /// tracing 审计观察日志文件写入器（append-only）
    writer: Mutex<Option<std::fs::File>>,
}

impl AuditLayer {
    /// 创建新的 AuditLayer
    ///
    /// # Arguments
    /// * `audit_dir` - tracing 审计观察日志目录
    pub fn new(audit_dir: &Path) -> anyhow::Result<Self> {
        // 确保日志目录存在
        std::fs::create_dir_all(audit_dir)?;

        // 获取今天的观察日志文件路径
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let audit_file = audit_dir.join(format!("audit.{}.log", today));

        // 打开文件（append 模式）
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&audit_file)?;

        Ok(Self {
            audit_dir: audit_dir.to_path_buf(),
            writer: Mutex::new(Some(file)),
        })
    }

    /// 获取今天的 tracing 审计观察日志文件路径
    fn get_today_audit_path(&self) -> PathBuf {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        self.audit_dir.join(format!("audit.{}.log", today))
    }

    /// 写入 tracing 审计观察日志条目
    fn write_audit_entry(&self, entry: &AuditEntry) {
        let mut writer_guard = self.writer.lock().unwrap();

        // 检查是否需要切换到新的日志文件（日期变更）
        let today_path = self.get_today_audit_path();
        if let Some(ref file) = *writer_guard {
            // 如果文件路径不同，需要重新打开
            if file.metadata().ok().map(|m| m.len()).unwrap_or(0) == 0 || !today_path.exists() {
                // 重新打开文件
                match OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&today_path)
                {
                    Ok(new_file) => {
                        *writer_guard = Some(new_file);
                    }
                    Err(e) => {
                        // 使用 eprintln 而非 tracing：AuditLayer 本身就是 tracing Layer，
                        // 此处是该 Layer 内部的文件操作失败，用 tracing 会递归调用自身
                        eprintln!("[audit] Failed to open audit file: {}", e);
                        return;
                    }
                }
            }
        }

        // 写入 tracing 审计观察日志
        if let Some(ref mut file) = *writer_guard {
            let line = entry.format_line();
            if let Err(e) = writeln!(file, "{}", line) {
                // 同上：AuditLayer 内部写入失败，不能用 tracing（会递归）
                eprintln!("[audit] Failed to write audit entry: {}", e);
            }
        }
    }

    /// 清理超过指定天数的 tracing 审计观察日志
    pub fn cleanup(&self, max_days: u32) -> anyhow::Result<u64> {
        let mut cleaned_count = 0;
        let now = Utc::now();
        let threshold = now - chrono::Duration::days(max_days as i64);

        for entry in std::fs::read_dir(&self.audit_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    // 解析文件名中的日期（audit.YYYY-MM-DD.log）
                    if file_name.starts_with("audit.") && file_name.ends_with(".log") {
                        let date_str = &file_name[6..file_name.len() - 4];
                        if let Ok(file_date) =
                            chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                        {
                            let file_datetime = file_date.and_hms_opt(0, 0, 0).unwrap();
                            let file_datetime_utc =
                                DateTime::<Utc>::from_naive_utc_and_offset(file_datetime, Utc);

                            if file_datetime_utc < threshold {
                                std::fs::remove_file(&path)?;
                                cleaned_count += 1;
                            }
                        }
                    }
                }
            }
        }

        Ok(cleaned_count)
    }
}

/// tracing Layer 实现
impl<S: tracing::Subscriber> Layer<S> for AuditLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        // 只处理带有 audit=true 标记的 tracing 事件
        let mut visitor = AuditFieldVisitor::new();
        event.record(&mut visitor);

        if !visitor.is_audit_event {
            return;
        }

        // 提取观察日志字段
        let action = visitor.action.unwrap_or_else(|| "unknown".to_string());
        let actor = visitor.actor.unwrap_or_else(|| "unknown".to_string());
        let target = visitor.target.unwrap_or_else(|| "unknown".to_string());
        let result = visitor.result.unwrap_or_else(|| "unknown".to_string());
        let session_id = visitor.session_id;
        let reason = visitor.reason;

        let Some(audit_action) = parse_audit_action(&action) else {
            tracing::warn!(
                action = %action,
                "Skipping tracing audit observation with unknown action"
            );
            return;
        };

        let Some(audit_result) = parse_audit_result(&result) else {
            tracing::warn!(
                result = %result,
                "Skipping tracing audit observation with unknown result"
            );
            return;
        };

        // 创建观察日志条目
        let entry = AuditEntry {
            timestamp: Utc::now(),
            action: audit_action,
            actor,
            target,
            result: audit_result,
            reason,
            session_id,
        };

        // 写入观察日志文件
        self.write_audit_entry(&entry);
    }
}

fn parse_audit_action(action: &str) -> Option<AuditAction> {
    match action {
        "FileRead" => Some(AuditAction::FileRead),
        "FileWrite" => Some(AuditAction::FileWrite),
        "FileDelete" => Some(AuditAction::FileDelete),
        "CommandExecute" => Some(AuditAction::CommandExecute),
        "NetworkRequest" => Some(AuditAction::NetworkRequest),
        "KeyAccess" => Some(AuditAction::KeyAccess),
        "SandboxBypass" => Some(AuditAction::SandboxBypass),
        _ => None,
    }
}

fn parse_audit_result(result: &str) -> Option<AuditResult> {
    match result {
        "allowed" => Some(AuditResult::Allowed),
        "denied" => Some(AuditResult::Denied),
        "failed" => Some(AuditResult::Failed),
        _ => None,
    }
}

/// tracing 审计观察字段访问者
struct AuditFieldVisitor {
    is_audit_event: bool,
    action: Option<String>,
    actor: Option<String>,
    target: Option<String>,
    result: Option<String>,
    session_id: Option<String>,
    reason: Option<String>,
}

impl AuditFieldVisitor {
    fn new() -> Self {
        Self {
            is_audit_event: false,
            action: None,
            actor: None,
            target: None,
            result: None,
            session_id: None,
            reason: None,
        }
    }
}

impl tracing::field::Visit for AuditFieldVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        match field.name() {
            "audit" => {
                // 检查 audit 标记
                let value_str = format!("{:?}", value);
                self.is_audit_event = value_str == "true" || value_str == "1";
            }
            "action" => self.action = Some(format!("{:?}", value)),
            "actor" => self.actor = Some(format!("{:?}", value)),
            "target" => self.target = Some(format!("{:?}", value)),
            "result" => self.result = Some(format!("{:?}", value)),
            "session_id" => self.session_id = Some(format!("{:?}", value)),
            "reason" => self.reason = Some(format!("{:?}", value)),
            _ => {}
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "audit" => {
                self.is_audit_event = value == "true" || value == "1";
            }
            "action" => self.action = Some(value.to_string()),
            "actor" => self.actor = Some(value.to_string()),
            "target" => self.target = Some(value.to_string()),
            "result" => self.result = Some(value.to_string()),
            "session_id" => self.session_id = Some(value.to_string()),
            "reason" => self.reason = Some(value.to_string()),
            _ => {}
        }
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        if field.name() == "audit" {
            self.is_audit_event = value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_audit_entry_format_line() {
        let entry = AuditEntry {
            timestamp: "2026-06-01T14:32:01.123Z".parse::<DateTime<Utc>>().unwrap(),
            action: AuditAction::FileWrite,
            actor: "agent".to_string(),
            target: "./src/main.ts".to_string(),
            result: AuditResult::Allowed,
            reason: None,
            session_id: Some("sess_xyz".to_string()),
        };
        let line = entry.format_line();
        assert!(line.contains("[AUDIT]"));
        assert!(line.contains("actor=agent"));
        assert!(line.contains("action=FileWrite"));
        assert!(line.contains("result=allowed"));
        assert!(line.contains("session=sess_xyz"));
    }

    #[test]
    fn test_audit_layer_creation() {
        let temp_dir = tempdir().unwrap();
        let audit_dir = temp_dir.path().join("audit");
        std::fs::create_dir_all(&audit_dir).unwrap();

        let layer = AuditLayer::new(&audit_dir).unwrap();
        assert_eq!(layer.audit_dir, audit_dir);
    }

    #[test]
    fn test_audit_action_display() {
        assert_eq!(AuditAction::FileRead.to_string(), "FileRead");
        assert_eq!(AuditAction::CommandExecute.to_string(), "CommandExecute");
        assert_eq!(AuditAction::SandboxBypass.to_string(), "SandboxBypass");
    }

    #[test]
    fn test_audit_result_display() {
        assert_eq!(AuditResult::Allowed.to_string(), "allowed");
        assert_eq!(AuditResult::Denied.to_string(), "denied");
        assert_eq!(AuditResult::Failed.to_string(), "failed");
    }

    #[test]
    fn test_unknown_audit_observation_fields_are_not_defaulted() {
        assert_eq!(parse_audit_action("FileRead"), Some(AuditAction::FileRead));
        assert_eq!(parse_audit_result("allowed"), Some(AuditResult::Allowed));
        assert_eq!(parse_audit_action("Unknown"), None);
        assert_eq!(parse_audit_result("unknown"), None);
    }
}
