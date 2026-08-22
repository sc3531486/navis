// 通用进程管理器：根据扩展声明拉起 Node/Python/可执行文件进程，
// 建立双向 stdio 通道，支持监控与回收。宿主仅保留该通用机制，不涉及业务逻辑。
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{Mutex, oneshot};
use tokio::time::{Duration, timeout};
use tracing::info;

use serde_json::Value;

/// 单个受管插件进程
pub struct ManagedProcess {
    pub plugin_id: String,
    pub child: Child,
    pub stdin: ChildStdin,
    pub started_at: Instant,
    /// 进行中的 JSON-RPC 请求（id -> 应答通道）
    pub pending: Arc<Mutex<HashMap<String, oneshot::Sender<Value>>>>,
}

/// 通用进程管理器
#[derive(Clone, Default)]
pub struct ProcessSupervisor {
    pub(crate) processes: Arc<Mutex<HashMap<String, ManagedProcess>>>,
}

impl ProcessSupervisor {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 根据 backend 入口解析本机可执行命令
    /// .mjs/.js/.cjs -> node；.py -> python；其余视为可执行文件
    pub fn resolve_command(main: &str) -> (String, Vec<String>) {
        let lower = main.to_lowercase();
        if lower.ends_with(".mjs") || lower.ends_with(".js") || lower.ends_with(".cjs") {
            ("node".to_string(), vec![main.to_string()])
        } else if lower.ends_with(".py") {
            ("python".to_string(), vec![main.to_string()])
        } else {
            (main.to_string(), Vec::new())
        }
    }

    /// 拉起插件进程；main 为 backend 入口，cwd 为扩展目录
    pub async fn spawn_plugin_process(
        &self,
        plugin_id: &str,
        main: &str,
        cwd: Option<&Path>,
    ) -> Result<(), String> {
        let mut lock = self.processes.lock().await;
        if lock.contains_key(plugin_id) {
            return Ok(());
        }

        let (command, args) = Self::resolve_command(main);
        let mut cmd = Command::new(&command);
        cmd.args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .env("NAVIS_PLUGIN_ID", plugin_id);
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        let mut child = cmd.spawn().map_err(|e| {
            format!("[Navis Process] spawn '{}' for '{}' failed: {e}", command, plugin_id)
        })?;
        let stdin = child.stdin.take().ok_or("[Navis Process] stdin unavailable")?;
        child.stderr.take(); // stderr 丢弃或后续接入日志通道

        lock.insert(
            plugin_id.to_string(),
            ManagedProcess {
                plugin_id: plugin_id.to_string(),
                child,
                stdin,
                started_at: Instant::now(),
                pending: Arc::new(Mutex::new(HashMap::new())),
            },
        );
        info!("[Navis Process] spawned plugin process: {plugin_id} ({main})");
        Ok(())
    }

    /// 向插件进程写入一行数据（JSON-RPC 包）
    pub async fn write_line(&self, plugin_id: &str, line: &str) -> Result<(), String> {
        let mut lock = self.processes.lock().await;
        let proc = lock.get_mut(plugin_id).ok_or_else(|| {
            format!("[Navis Process] no process for plugin '{plugin_id}'")
        })?;
        proc.stdin
            .write_all(format!("{line}\n").as_bytes())
            .await
            .map_err(|e| format!("[Navis Process] write to '{plugin_id}' failed: {e}"))?;
        proc.stdin.flush().await.map_err(|e| e.to_string())
    }

    /// 注册一个待应答的请求；返回应答接收端
    pub async fn await_response(
        &self,
        plugin_id: &str,
        request_id: String,
        timeout_ms: u64,
    ) -> Result<Value, String> {
        let pending = {
            let lock = self.processes.lock().await;
            lock.get(plugin_id)
                .ok_or_else(|| format!("[Navis Process] no process for plugin '{plugin_id}'"))?
                .pending
                .clone()
        };
        let (tx, rx) = oneshot::channel();
        pending.lock().await.insert(request_id, tx);

        timeout(Duration::from_millis(timeout_ms), rx)
            .await
            .map_err(|_| format!("[Navis Process] rpc timeout for plugin '{plugin_id}'"))?
            .map_err(|_| format!("[Navis Process] rpc channel closed for plugin '{plugin_id}'"))
    }

    /// 解析一行 stdout 为应答并写入 pending（由读取任务调用）
    pub async fn resolve_line(&self, plugin_id: &str, line: &str) -> Option<Value> {
        let value: Value = serde_json::from_str(line).ok()?;
        let id = value.get("id").and_then(|v| v.as_str()).map(String::from)?;
        let pending = {
            let lock = self.processes.lock().await;
            lock.get(plugin_id)?.pending.clone()
        };
        if let Some(tx) = pending.lock().await.remove(&id) {
            let _ = tx.send(value.clone());
        }
        Some(value)
    }

    pub async fn is_running(&self, plugin_id: &str) -> bool {
        let lock = self.processes.lock().await;
        lock.get(plugin_id).map(|p| p.child.id().is_some()).unwrap_or(false)
    }

    /// 终止并回收插件进程
    pub async fn kill_plugin_process(&self, plugin_id: &str) -> Result<(), String> {
        let mut lock = self.processes.lock().await;
        if let Some(mut proc) = lock.remove(plugin_id) {
            let _ = proc.child.kill().await;
            let _ = proc.child.wait().await;
            info!("[Navis Process] killed plugin process: {plugin_id}");
        }
        Ok(())
    }

    /// 停止所有进程（宿主退出时调用）
    pub async fn shutdown(&self) {
        let mut lock = self.processes.lock().await;
        for (id, mut proc) in lock.drain() {
            let _ = proc.child.kill().await;
            info!("[Navis Process] shutdown plugin process: {id}");
        }
    }

    /// 存活进程清单
    pub async fn list_running(&self) -> Vec<String> {
        let lock = self.processes.lock().await;
        lock.keys().cloned().collect()
    }
}