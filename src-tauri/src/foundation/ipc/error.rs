//! IPC 错误类型
//!
//! 基于设计文档 §7 实现，定义 IPC 通信中的错误类型
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

use serde::{Deserialize, Serialize};
use std::fmt;

/// IPC 错误类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcError {
    /// 命令未找到
    CommandNotFound { command: String },
    /// 参数验证失败
    InvalidParams { command: String, message: String },
    /// 执行超时
    Timeout { command: String, timeout_ms: u64 },
    /// 执行失败
    ExecutionFailed { command: String, message: String },
    /// 序列化/反序列化失败
    SerializationError { message: String },
    /// 会话不存在
    SessionNotFound { session_id: String },
    /// 权限不足
    PermissionDenied { command: String, reason: String },
    /// 内部错误
    InternalError { message: String },
}

impl fmt::Display for IpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpcError::CommandNotFound { command } => {
                write!(f, "Command not found: {}", command)
            }
            IpcError::InvalidParams { command, message } => {
                write!(f, "Invalid params for command '{}': {}", command, message)
            }
            IpcError::Timeout {
                command,
                timeout_ms,
            } => {
                write!(f, "Command '{}' timed out after {}ms", command, timeout_ms)
            }
            IpcError::ExecutionFailed { command, message } => {
                write!(f, "Command '{}' execution failed: {}", command, message)
            }
            IpcError::SerializationError { message } => {
                write!(f, "Serialization error: {}", message)
            }
            IpcError::SessionNotFound { session_id } => {
                write!(f, "Session not found: {}", session_id)
            }
            IpcError::PermissionDenied { command, reason } => {
                write!(f, "Permission denied for command '{}': {}", command, reason)
            }
            IpcError::InternalError { message } => {
                write!(f, "Internal error: {}", message)
            }
        }
    }
}

impl std::error::Error for IpcError {}

impl IpcError {
    /// 获取错误代码
    pub fn code(&self) -> &'static str {
        match self {
            IpcError::CommandNotFound { .. } => "COMMAND_NOT_FOUND",
            IpcError::InvalidParams { .. } => "INVALID_PARAMS",
            IpcError::Timeout { .. } => "TIMEOUT",
            IpcError::ExecutionFailed { .. } => "EXECUTION_FAILED",
            IpcError::SerializationError { .. } => "SERIALIZATION_ERROR",
            IpcError::SessionNotFound { .. } => "SESSION_NOT_FOUND",
            IpcError::PermissionDenied { .. } => "PERMISSION_DENIED",
            IpcError::InternalError { .. } => "INTERNAL_ERROR",
        }
    }

    /// 创建命令未找到错误
    pub fn command_not_found(command: impl Into<String>) -> Self {
        let command = command.into();
        tracing::warn!(command = %command, "Command not found");
        Self::CommandNotFound { command }
    }

    /// 创建参数验证失败错误
    pub fn invalid_params(command: impl Into<String>, message: impl Into<String>) -> Self {
        let command = command.into();
        let message = message.into();
        tracing::warn!(command = %command, message = %message, "Invalid params");
        Self::InvalidParams { command, message }
    }

    /// 创建超时错误
    pub fn timeout(command: impl Into<String>, timeout_ms: u64) -> Self {
        let command = command.into();
        tracing::warn!(command = %command, timeout_ms = timeout_ms, "Command timeout");
        Self::Timeout {
            command,
            timeout_ms,
        }
    }

    /// 创建执行失败错误
    pub fn execution_failed(command: impl Into<String>, message: impl Into<String>) -> Self {
        let command = command.into();
        let message = message.into();
        tracing::error!(command = %command, message = %message, "Command execution failed");
        Self::ExecutionFailed { command, message }
    }

    /// 创建序列化错误
    pub fn serialization_error(message: impl Into<String>) -> Self {
        let message = message.into();
        tracing::error!(message = %message, "Serialization error");
        Self::SerializationError { message }
    }

    /// 创建会话不存在错误
    pub fn session_not_found(session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        tracing::warn!(session_id = %session_id, "Session not found");
        Self::SessionNotFound { session_id }
    }

    /// 创建权限不足错误
    pub fn permission_denied(command: impl Into<String>, reason: impl Into<String>) -> Self {
        let command = command.into();
        let reason = reason.into();
        tracing::warn!(command = %command, reason = %reason, "Permission denied");
        Self::PermissionDenied { command, reason }
    }

    /// 创建内部错误
    pub fn internal_error(message: impl Into<String>) -> Self {
        let message = message.into();
        tracing::error!(message = %message, "Internal error");
        Self::InternalError { message }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = IpcError::command_not_found("test.command");
        assert_eq!(err.to_string(), "Command not found: test.command");
        assert_eq!(err.code(), "COMMAND_NOT_FOUND");
    }

    #[test]
    fn test_error_invalid_params() {
        let err = IpcError::invalid_params("test.command", "missing required field");
        assert!(err.to_string().contains("Invalid params"));
        assert_eq!(err.code(), "INVALID_PARAMS");
    }

    #[test]
    fn test_error_timeout() {
        let err = IpcError::timeout("test.command", 30000);
        assert!(err.to_string().contains("timed out"));
        assert_eq!(err.code(), "TIMEOUT");
    }

    #[test]
    fn test_error_execution_failed() {
        let err = IpcError::execution_failed("test.command", "connection failed");
        assert!(err.to_string().contains("execution failed"));
        assert_eq!(err.code(), "EXECUTION_FAILED");
    }

    #[test]
    fn test_error_serialization_error() {
        let err = IpcError::serialization_error("invalid JSON");
        assert!(err.to_string().contains("Serialization error"));
        assert_eq!(err.code(), "SERIALIZATION_ERROR");
    }

    #[test]
    fn test_error_session_not_found() {
        let err = IpcError::session_not_found("sess_001");
        assert!(err.to_string().contains("Session not found"));
        assert_eq!(err.code(), "SESSION_NOT_FOUND");
    }

    #[test]
    fn test_error_permission_denied() {
        let err = IpcError::permission_denied("test.command", "insufficient permissions");
        assert!(err.to_string().contains("Permission denied"));
        assert_eq!(err.code(), "PERMISSION_DENIED");
    }

    #[test]
    fn test_error_internal_error() {
        let err = IpcError::internal_error("something went wrong");
        assert!(err.to_string().contains("Internal error"));
        assert_eq!(err.code(), "INTERNAL_ERROR");
    }
}
