// 通用多路复用 IPC 网关：前端只通过 core_route_ipc / core_route_stream 通信，
// 本模块负责把 JSON-RPC 包路由到对应插件进程的 stdin，并将其 stdout 解析为
// 应答/通知，分别回送请求方或流式 Channel。不绑定任何业务方法。
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Mutex;

use crate::core::process_supervisor::ProcessSupervisor;

/// 通知订阅者：plugin_id -> 订阅集合
#[derive(Default)]
pub struct TransportRouter {
    supervisor: ProcessSupervisor,
    subscribers: Arc<Mutex<HashMap<String, Vec<tauri::ipc::Channel<Value>>>>>,
    default_timeout_ms: u64,
}

impl TransportRouter {
    pub fn new() -> Self {
        Self {
            supervisor: ProcessSupervisor::new(),
            subscribers: Arc::new(Mutex::new(HashMap::new())),
            default_timeout_ms: 30_000,
        }
    }

    /// 按清单声明拉起插件后端进程，并启动 stdout 读取任务（async 版，供测试/命令复用）
    pub async fn ensure_plugin_process_async(
        &self,
        plugin_id: &str,
        main: &str,
        cwd: Option<&Path>,
    ) -> Result<(), String> {
        self.supervisor.spawn_plugin_process(plugin_id, main, cwd).await?;
        // 启动 stdout 读取任务
        let sup = self.supervisor.clone();
        let this = self.clone();
        let pid = plugin_id.to_string();
        tauri::async_runtime::spawn(async move {
            let stdout = {
                let mut lock = sup.processes.lock().await;
                match lock.get_mut(&pid) {
                    Some(p) => match p.child.stdout.take() {
                        Some(s) => s,
                        None => return,
                    },
                    None => return,
                }
            };
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let trimmed = line.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }
                // 通知（带 method 字段）广播给订阅者；应答（带 id）回送请求方
                let value: Value = match serde_json::from_str(&trimmed) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if value.get("method").is_some() {
                    let subs = this.subscribers.lock().await;
                    if let Some(channels) = subs.get(&pid) {
                        for ch in channels {
                            let _ = ch.send(value.clone());
                        }
                    }
                } else if value.get("id").is_some() {
                    let _ = sup.resolve_line(&pid, &trimmed).await;
                }
            }
            // 进程退出：清订阅
            this.subscribers.lock().await.remove(&pid);
        });
        Ok(())
    }

    /// 按清单声明拉起插件后端进程（setup 阶段同步入口）
    pub fn ensure_plugin_process(
        &self,
        plugin_id: &str,
        main: &str,
        cwd: Option<&Path>,
    ) -> Result<(), String> {
        tauri::async_runtime::block_on(self.ensure_plugin_process_async(plugin_id, main, cwd))
    }

    /// 同步 JSON-RPC 调用：写入请求并等待应答
    pub async fn send_rpc(
        &self,
        plugin_id: &str,
        method: &str,
        params: Value,
    ) -> Result<Value, String> {
        let request_id = format!("r{}-{}", method, uuid_lite());
        let request = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params,
        });
        self.supervisor.write_line(plugin_id, &request.to_string()).await?;
        self.supervisor.await_response(plugin_id, request_id, self.default_timeout_ms).await
    }

    /// 流式 JSON-RPC 调用：订阅通知并将事件推送到 Channel
    pub async fn stream_rpc(
        &self,
        plugin_id: &str,
        method: &str,
        params: Value,
        on_event: tauri::ipc::Channel<Value>,
    ) -> Result<(), String> {
        self.subscribers.lock().await
            .entry(plugin_id.to_string())
            .or_default()
            .push(on_event.clone());

        let request_id = format!("s{}-{}", method, uuid_lite());
        let request = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params,
        });
        self.supervisor.write_line(plugin_id, &request.to_string()).await?;
        // 等待最终应答（期间的 method 通知已被推送到 Channel）
        let _ = self.supervisor.await_response(plugin_id, request_id, self.default_timeout_ms).await;

        // 移除订阅（按 channel id 比较，克隆共享同一 id）
        let channel_id = on_event.id();
        let mut subs = self.subscribers.lock().await;
        if let Some(list) = subs.get_mut(plugin_id) {
            list.retain(|ch| ch.id() != channel_id);
            if list.is_empty() {
                subs.remove(plugin_id);
            }
        }
        Ok(())
    }

    pub async fn kill(&self, plugin_id: &str) -> Result<(), String> {
        self.supervisor.kill_plugin_process(plugin_id).await
    }

    pub async fn list_running(&self) -> Vec<String> {
        self.supervisor.list_running().await
    }

    pub async fn shutdown(&self) {
        self.supervisor.shutdown().await;
    }
}

impl Clone for TransportRouter {
    fn clone(&self) -> Self {
        Self {
            supervisor: self.supervisor.clone(),
            subscribers: self.subscribers.clone(),
            default_timeout_ms: self.default_timeout_ms,
        }
    }
}

/// 简易唯一 id 生成（避免引入 uuid 依赖）
fn uuid_lite() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:x}-{:x}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0), n)
}