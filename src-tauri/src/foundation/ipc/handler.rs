//! IPC 命令分发器
//!
//! 基于设计文档 §4.1 实现，提供 IPC 命令处理器登记和分发能力。
//! 这里维护的是 Tauri IPC command -> handler 的进程内分发表，不是 Kernel Registry。
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::codec::{IpcCommand, IpcResponse};
use super::error::IpcError;

/// IPC 命令处理器类型
pub type IpcHandler = Arc<dyn Fn(IpcCommand) -> Result<Value, IpcError> + Send + Sync>;

/// IPC 命令分发器
///
/// 管理 Tauri IPC 命令处理器映射。它不是能力注册表，不参与 Kernel Registry 生命周期。
pub struct IpcDispatcher {
    /// 命令处理器映射（command_name -> handler）
    handlers: Arc<Mutex<HashMap<String, IpcHandler>>>,
}

impl IpcDispatcher {
    /// 创建新的命令分发器
    pub fn new() -> Self {
        tracing::debug!("Creating new IpcDispatcher");

        Self {
            handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 绑定命令处理器到分发表。
    ///
    /// 这只是进程内 command -> handler 绑定，不声明 Kernel Capability，
    /// 也不参与能力生命周期管理。
    ///
    /// # Arguments
    /// * `name` - 命令名（如 "agent.cancelTask"）
    /// * `handler` - 命令处理器
    pub fn register(
        &self,
        name: &str,
        handler: impl Fn(IpcCommand) -> Result<Value, IpcError> + Send + Sync + 'static,
    ) {
        let mut handlers = self.handlers.lock().unwrap();

        tracing::info!(
            command = %name,
            "Registering IPC command handler"
        );

        handlers.insert(name.to_string(), Arc::new(handler));
    }

    /// 分发 IPC 命令
    ///
    /// # Arguments
    /// * `command` - IPC 命令
    ///
    /// # Returns
    /// IPC 响应
    pub fn dispatch(&self, command: IpcCommand) -> IpcResponse {
        let request_id = command.request_id.clone();
        let command_name = command.name.clone();
        let session_id = command.session_id.clone();

        tracing::debug!(
            request_id = %request_id,
            command = %command_name,
            session_id = ?session_id,
            "Handling IPC command"
        );

        // 只在查表时持有分发表锁，handler 执行不属于 Registry/索引维护路径。
        let handler = {
            let handlers = self.handlers.lock().unwrap();
            handlers.get(&command_name).cloned()
        };

        match handler {
            Some(handler) => {
                // 执行命令处理器。具体业务事实源由对应领域模块负责。
                let start_time = std::time::Instant::now();

                match handler(command) {
                    Ok(result) => {
                        let duration = start_time.elapsed();
                        tracing::debug!(
                            request_id = %request_id,
                            command = %command_name,
                            duration_ms = duration.as_millis(),
                            "IPC command executed successfully"
                        );
                        IpcResponse::success(request_id, result)
                    }
                    Err(error) => {
                        let duration = start_time.elapsed();
                        tracing::error!(
                            request_id = %request_id,
                            command = %command_name,
                            duration_ms = duration.as_millis(),
                            error = %error,
                            "IPC command execution failed"
                        );
                        IpcResponse::error(request_id, error)
                    }
                }
            }
            None => {
                tracing::warn!(
                    request_id = %request_id,
                    command = %command_name,
                    "IPC command handler not found"
                );
                IpcResponse::error(request_id, IpcError::command_not_found(&command_name))
            }
        }
    }

    /// 检查命令是否已注册
    ///
    /// # Arguments
    /// * `name` - 命令名
    pub fn has_handler(&self, name: &str) -> bool {
        let handlers = self.handlers.lock().unwrap();
        handlers.contains_key(name)
    }

    /// 获取已注册的命令列表
    pub fn get_registered_commands(&self) -> Vec<String> {
        let handlers = self.handlers.lock().unwrap();
        handlers.keys().cloned().collect()
    }

    /// 获取注册的命令数量
    pub fn handler_count(&self) -> usize {
        let handlers = self.handlers.lock().unwrap();
        handlers.len()
    }
}

impl Default for IpcDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for IpcDispatcher {
    fn clone(&self) -> Self {
        Self {
            handlers: Arc::clone(&self.handlers),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_ipc_dispatcher_register_and_dispatch() {
        let dispatcher = IpcDispatcher::new();

        // 注册处理器
        dispatcher.register("test.echo", |cmd| Ok(cmd.payload));

        // 创建命令
        let command = IpcCommand::new("test.echo", json!({"message": "hello"}), None);

        // 处理命令
        let response = dispatcher.dispatch(command);

        assert!(response.is_success());
        assert_eq!(response.data(), Some(&json!({"message": "hello"})));
    }

    #[test]
    fn test_ipc_dispatcher_command_not_found() {
        let dispatcher = IpcDispatcher::new();

        // 创建命令
        let command = IpcCommand::new("nonexistent.command", json!({}), None);

        // 处理命令
        let response = dispatcher.dispatch(command);

        assert!(!response.is_success());
        assert!(response.get_error().is_some());

        if let Some(error) = response.get_error() {
            assert_eq!(error.code(), "COMMAND_NOT_FOUND");
        }
    }

    #[test]
    fn test_ipc_dispatcher_handler_error() {
        let dispatcher = IpcDispatcher::new();

        // 注册会返回错误的处理器
        dispatcher.register("test.error", |_cmd| {
            Err(IpcError::execution_failed("test.error", "test error"))
        });

        // 创建命令
        let command = IpcCommand::new("test.error", json!({}), None);

        // 处理命令
        let response = dispatcher.dispatch(command);

        assert!(!response.is_success());
        assert!(response.get_error().is_some());
    }

    #[test]
    fn test_ipc_dispatcher_has_handler() {
        let dispatcher = IpcDispatcher::new();

        assert!(!dispatcher.has_handler("test.command"));

        dispatcher.register("test.command", |cmd| Ok(cmd.payload));

        assert!(dispatcher.has_handler("test.command"));
    }

    #[test]
    fn test_ipc_dispatcher_get_registered_commands() {
        let dispatcher = IpcDispatcher::new();

        dispatcher.register("command1", |cmd| Ok(cmd.payload));
        dispatcher.register("command2", |cmd| Ok(cmd.payload));
        dispatcher.register("command3", |cmd| Ok(cmd.payload));

        let commands = dispatcher.get_registered_commands();
        assert_eq!(commands.len(), 3);
        assert!(commands.contains(&"command1".to_string()));
        assert!(commands.contains(&"command2".to_string()));
        assert!(commands.contains(&"command3".to_string()));
    }

    #[test]
    fn test_ipc_dispatcher_handler_count() {
        let dispatcher = IpcDispatcher::new();

        assert_eq!(dispatcher.handler_count(), 0);

        dispatcher.register("command1", |cmd| Ok(cmd.payload));
        dispatcher.register("command2", |cmd| Ok(cmd.payload));

        assert_eq!(dispatcher.handler_count(), 2);
    }

    #[test]
    fn test_ipc_dispatcher_with_session() {
        let dispatcher = IpcDispatcher::new();

        // 注册处理器，检查会话 ID
        dispatcher.register("test.session", |cmd| {
            if let Some(session_id) = cmd.session_id {
                Ok(json!({"session_id": session_id}))
            } else {
                Err(IpcError::invalid_params(
                    "test.session",
                    "session_id required",
                ))
            }
        });

        // 创建带会话的命令
        let command = IpcCommand::with_session("test.session", json!({}), "sess_001");

        // 处理命令
        let response = dispatcher.dispatch(command);

        assert!(response.is_success());
        assert_eq!(response.data(), Some(&json!({"session_id": "sess_001"})));
    }
}
