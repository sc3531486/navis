//! 扩展流订阅 IPC（`extension.stream.subscribeSource` 后端出口）。
//!
//! 34（§2.8 / §4.3）+ 02b-stream（§3.8）：实时感知流（Agent 动作、任务状态、
//! Gateway 流式内容）必须逐条实时投递，禁止节流/合并；性能靠按需订阅
//! （Stream 是推模型，无订阅者不产生转发）。
//!
//! 本模块在扩展 **Enabled 且声明 `capabilities.provides: ["stream"]`** 后建立一条
//! 命名流（StreamChannel + StreamIndex track），并把一个 broadcast 订阅登记进
//! StreamIndex。宿主发布点（如 Agent 流推送，见 `session_message_stream.rs`）调用
//! `StreamIndex::publish(kind, session_id, data)` 时，命中 filter 的数据经广播
//! 投递到订阅者传入的 Tauri Channel。
//!
//! 命令不经过 `extension_bridge.rs` 的桥注册表，是独立 Tauri 命令：host:panel
//! 与扩展 iframe 的宿主桥 dispatcher 直接 invoke（宿主侧可信，已校验来源）。

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{ipc::Channel, State};

use crate::extension::{ExtensionStatus, ExtensionStore};
use crate::foundation::stream::{
    send_channel_value, StreamChannel, StreamIndex, StreamSource, StreamSubscriptionFilter,
};

/// `extension.stream.subscribeSource(filter)` 的 filter。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionStreamFilter {
    /// 流类型（如 "agent"）
    pub kind: String,
    /// 可选会话 ID
    #[serde(default)]
    pub session_id: Option<String>,
}

/// 流订阅请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionStreamSubscribeRequest {
    /// 发起订阅的扩展 ID（Enabled + capabilities 校验）
    pub extension_id: String,
    /// 流来源过滤器
    pub filter: ExtensionStreamFilter,
}

/// 流取消请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionStreamUnsubscribeRequest {
    /// 订阅时返回的 stream ID
    pub stream_id: String,
}

/// 校验扩展具备流订阅能力：Enabled + `capabilities.provides` 含 `stream`。
fn ensure_stream_subscribe_allowed(
    store: &ExtensionStore,
    extension_id: &str,
) -> Result<(), String> {
    let state = store
        .get(extension_id)
        .ok_or_else(|| format!("Extension '{extension_id}' is not installed"))?;
    if state.status != ExtensionStatus::Enabled {
        return Err(format!("Extension '{extension_id}' is not enabled"));
    }
    let Some(capabilities) = &state.manifest.contributes.capabilities else {
        return Err(format!(
            "Extension '{extension_id}' did not declare a capabilities block"
        ));
    };
    if !capabilities.provides.iter().any(|label| label == "stream") {
        return Err(format!(
            "Extension '{extension_id}' did not declare the 'stream' capability"
        ));
    }
    Ok(())
}

/// 订阅源流（`extension.stream.subscribeSource` 后端出口）。
///
/// 建立一条命名流（StreamChannel + StreamIndex track）并把 broadcast 订阅登记进
/// StreamIndex；宿主发布点 `publish` 命中时，数据经广播转发到传入的 Tauri Channel。
/// 返回 stream ID，供 `ui_extension_stream_unsubscribe` 取消。
#[tauri::command]
pub async fn ui_extension_stream_subscribe(
    extension_store: State<'_, Arc<ExtensionStore>>,
    stream_index: State<'_, Arc<StreamIndex>>,
    request: ExtensionStreamSubscribeRequest,
    channel: Channel,
) -> Result<String, String> {
    ensure_stream_subscribe_allowed(extension_store.inner().as_ref(), &request.extension_id)?;

    let mut filter = StreamSubscriptionFilter::new(request.filter.kind.clone());
    if let Some(session_id) = &request.filter.session_id {
        filter = filter.with_session_id(session_id);
    }
    let source = StreamSource::new(&request.filter.kind, &request.extension_id);
    let stream_channel = StreamChannel::builder(channel.clone())
        .source(source)
        .index((**stream_index.inner()).clone())
        .build();
    let stream_id = stream_channel.stream_id().to_string();

    let (tx, mut rx) = tokio::sync::broadcast::channel(64);
    stream_index
        .inner()
        .subscribe_for_stream(&stream_id, filter, tx);

    let forward_channel = channel;
    let log_stream_id = stream_id.clone();
    let forward_stream_id = stream_id.clone();
    tauri::async_runtime::spawn(async move {
        // 保持命名流存活到广播关闭：订阅者被移除 → rx 收到 Closed → StreamChannel
        // Drop → StreamIndex untrack。
        let _keepalive = stream_channel;
        loop {
            match rx.recv().await {
                Ok(data) => {
                    if let Err(error) = send_channel_value(&forward_channel, data) {
                        tracing::debug!(
                            stream_id = %forward_stream_id,
                            error = %error,
                            "扩展流订阅转发停止"
                        );
                        break;
                    }
                }
                // 慢消费者回退：跳过滞后的数据继续接收，不丢流
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    tracing::info!(
        extension_id = %request.extension_id,
        stream_id = %log_stream_id,
        kind = %request.filter.kind,
        "扩展 stream.subscribeSource 已订阅"
    );

    Ok(stream_id)
}

/// 取消流订阅：移除订阅者并解绑 StreamIndex 中的命名流。
#[tauri::command]
pub fn ui_extension_stream_unsubscribe(
    stream_index: State<'_, Arc<StreamIndex>>,
    payload: ExtensionStreamUnsubscribeRequest,
) -> Result<(), String> {
    let stream_id = payload.stream_id.trim();
    if stream_id.is_empty() {
        return Err("streamId 不能为空".to_string());
    }
    let removed = stream_index.inner().unsubscribe_by_stream(stream_id);
    stream_index.inner().untrack(stream_id);
    tracing::info!(stream_id, removed, "扩展 stream 订阅已取消");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::models::{ExtensionManifest, ExtensionState};
    use crate::extension::ExtensionContributes;
    use crate::kernel::InMemoryEventBus;
    use chrono::Utc;
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::OnceLock;
    use tokio::runtime::Runtime;

    fn test_runtime_handle() -> tokio::runtime::Handle {
        static RUNTIME: OnceLock<Runtime> = OnceLock::new();
        RUNTIME
            .get_or_init(|| Runtime::new().expect("test tokio runtime"))
            .handle()
            .clone()
    }

    fn test_store() -> ExtensionStore {
        ExtensionStore::new(Arc::new(InMemoryEventBus::new(1000, test_runtime_handle())))
    }

    fn test_extension_state(id: &str, enabled: bool, provides_stream: bool) -> ExtensionState {
        let mut contributes = ExtensionContributes::default();
        contributes.capabilities = Some(crate::extension::models::CapabilityDeclaration {
            provides: if provides_stream {
                vec!["stream".to_string()]
            } else {
                vec!["storage".to_string()]
            },
            ..Default::default()
        });
        ExtensionState {
            id: id.to_string(),
            status: if enabled {
                ExtensionStatus::Enabled
            } else {
                ExtensionStatus::Installed
            },
            manifest: ExtensionManifest {
                id: id.to_string(),
                name: format!("Extension {}", id),
                version: "1.0.0".into(),
                description: "test".into(),
                author: "test".into(),
                permissions: Default::default(),
                contributes,
            },
            install_path: PathBuf::from(format!("/extensions/{}", id)),
            installed_at: Utc::now(),
            enabled_at: enabled.then(Utc::now),
            error: None,
        }
    }

    #[test]
    fn subscribe_request_serializes_camel_case() {
        let request = ExtensionStreamSubscribeRequest {
            extension_id: "ext.alpha".into(),
            filter: ExtensionStreamFilter {
                kind: "agent".into(),
                session_id: Some("sess_001".into()),
            },
        };
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["extensionId"], "ext.alpha");
        assert_eq!(value["filter"]["kind"], "agent");
        assert_eq!(value["filter"]["sessionId"], "sess_001");
    }

    #[test]
    fn stream_subscribe_requires_enabled_extension_with_stream_capability() {
        let store = test_store();

        // 未安装 → 拒绝
        assert!(ensure_stream_subscribe_allowed(&store, "ext.missing").is_err());

        // 已安装但未启用 → 拒绝
        let disabled = test_extension_state("ext.disabled", false, true);
        store.register(disabled).unwrap();
        assert!(ensure_stream_subscribe_allowed(&store, "ext.disabled").is_err());

        // 已启用但未声明 capabilities / 未声明 stream 能力 → 拒绝
        let no_cap = test_extension_state("ext.nocap", true, false);
        store.register(no_cap).unwrap();
        assert!(ensure_stream_subscribe_allowed(&store, "ext.nocap").is_err());

        // 已启用且声明 stream 能力 → 放行
        let allowed = test_extension_state("ext.ok", true, true);
        store.register(allowed).unwrap();
        assert!(ensure_stream_subscribe_allowed(&store, "ext.ok").is_ok());
    }

    #[tokio::test]
    async fn stream_index_publish_reaches_subscriber() {
        let index = StreamIndex::new();
        let (tx, mut rx) = tokio::sync::broadcast::channel(16);
        index.subscribe_for_stream(
            "stream:agent:uuid-1",
            StreamSubscriptionFilter::new("agent"),
            tx,
        );

        // 命中 kind + session 的发布 → 订阅者收到
        index.publish("agent", Some("sess_001"), json!({ "delta": "hello" }));
        assert_eq!(rx.recv().await.unwrap(), json!({ "delta": "hello" }));

        // 不命中 kind → 不投递
        index.publish("terminal", Some("sess_001"), json!({ "delta": "no" }));
        assert!(rx.try_recv().is_err());
    }
}
