//! 扩展网络代理 IPC。
//!
//! 扩展 iframe 没有直接网络权限；所有 fetch 经宿主代理并按
//! contributes.network / capabilities.network 策略校验。未声明网络能力时 fail-closed。
//!
//! 安全模型：host allowlist（声明校验）为第一道防线；SSRF 防护（IP 字面量
//! 内网/环回拦截）与拒绝路径审计作为纵深防御。域名不做 DNS 解析，域名
//! 指向内网解析的逃逸属于已知限制（见 `is_blocked_private_ip`）。

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::State;
use reqwest::Url;

use crate::extension::models::NetworkPolicy;
use crate::extension::{ExtensionStatus, ExtensionStore};
use crate::kernel::{AuditRecord, AuditRecorder, AuditStatus, KernelContext, KernelScope};
// use [REMOVED: MCP reference]

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionNetworkRequest {
    pub extension_id: String,
    pub url: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    #[serde(default)]
    pub body: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionNetworkResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

fn default_method() -> String {
    "GET".to_string()
}

fn policy_for_extension(store: &ExtensionStore, extension_id: &str) -> Result<NetworkPolicy, String> {
    let state = store
        .get(extension_id)
        .ok_or_else(|| format!("Extension '{extension_id}' is not installed"))?;
    if state.status != ExtensionStatus::Enabled {
        return Err(format!("Extension '{extension_id}' is not enabled"));
    }
    Ok(state
        .manifest
        .contributes
        .network
        .clone()
        .or_else(|| state.manifest.contributes.capabilities.as_ref().and_then(|caps| caps.network.clone()))
        .unwrap_or(NetworkPolicy::None))
}

fn host_allowed(policy: &NetworkPolicy, url: &Url) -> bool {
    match policy {
        NetworkPolicy::None => false,
        NetworkPolicy::Proxy => matches!(url.scheme(), "http" | "https"),
        NetworkPolicy::Allowlist { hosts } => {
            let Some(host) = url.host_str() else { return false; };
            hosts.iter().any(|allowed| {
                let protocol_ok = allowed.protocols.is_empty()
                    || allowed.protocols.iter().any(|protocol| protocol.trim_end_matches(':') == url.scheme());
                if !protocol_ok { return false; }
                host == allowed.host || (allowed.allow_subdomains && host.ends_with(&format!(".{}", allowed.host)))
            })
        }
    }
}

fn safe_header(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    !matches!(lower.as_str(), "host" | "cookie" | "authorization" | "proxy-authorization" | "connection" | "content-length")
}

/// 扩展网络代理审计的 action 命名空间。
const NETWORK_AUDIT_ACTION: &str = "extension.network.request";

/// 记录扩展网络请求审计（允许/拒绝路径）。
fn record_network_audit(
    audit: &AuditRecorder,
    extension_id: &str,
    url: &str,
    status: AuditStatus,
    reason: Option<&str>,
) {
    let ctx = KernelContext::new("extension.network", KernelScope::global());
    let decision = json!({
        "extensionId": extension_id,
        "url": url,
        "allowed": status == AuditStatus::Success,
        "reason": reason,
    });
    let record = AuditRecord::new(
        &ctx,
        uuid::Uuid::new_v4().to_string(),
        NETWORK_AUDIT_ACTION,
        status,
    )
    .with_policy_decision(decision);
    if let Err(error) = audit.record_owned(record) {
        tracing::warn!(error = %error, "Failed to record extension network audit");
    }
}

/// SSRF 防护：拦截 IP 字面量指向的私有/环回/链路本地/未指定地址。
///
/// 仅检查 IP 字面量，不做 DNS 解析（避免引入异步 DNS 复杂性）；域名经解析
/// 指向内网地址的逃逸属于已知限制。命中即拒绝，作为 host allowlist 的纵深防御。
fn is_blocked_private_ip(url: &Url) -> bool {
    let Some(host) = url.host_str() else { return false; };
    // host_str 对 IPv6 字面量返回带方括号形式（`[::1]`），先剥离再解析
    let host = host
        .strip_prefix('[')
        .and_then(|inner| inner.strip_suffix(']'))
        .unwrap_or(host);
    match host.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(ip)) => {
            // 0.0.0.0 / 127.0.0.0/8 / 10.0.0.0/8+172.16.0.0/12+192.168.0.0/16 / 169.254.0.0/16
            ip.is_unspecified() || ip.is_loopback() || ip.is_private() || ip.is_link_local()
        }
        Ok(std::net::IpAddr::V6(ip)) => {
            // :: / ::1 / fc00::/7 / fe80::/10
            ip.is_unspecified() || ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local()
        }
        Err(_) => false, // 域名不做 DNS 解析，保留现有 host 匹配语义
    }
}

#[tauri::command]
pub async fn ui_extension_network_proxy(
    extension_store: State<'_, Arc<ExtensionStore>>,
    mcp: State<'_, Arc<MCP>>,
    request: ExtensionNetworkRequest,
) -> Result<ExtensionNetworkResponse, String> {
    let url = Url::parse(&request.url).map_err(|error| format!("Invalid URL: {error}"))?;
    let policy = policy_for_extension(extension_store.inner().as_ref(), &request.extension_id)?;
    let audit = mcp.sandbox().audit_recorder();

    if !host_allowed(&policy, &url) {
        tracing::warn!(extension_id = %request.extension_id, url = %request.url, "Extension network request denied by network policy");
        record_network_audit(
            &audit,
            &request.extension_id,
            &request.url,
            AuditStatus::Failed,
            Some("not allowed by network policy"),
        );
        return Err("Extension network request denied by network policy".to_string());
    }

    // SSRF 防护（Allowlist 与 Proxy 模式都生效）
    if is_blocked_private_ip(&url) {
        tracing::warn!(
            extension_id = %request.extension_id,
            url = %request.url,
            host = %url.host_str().unwrap_or(""),
            "Extension network request denied: private/loopback/link-local IP literal"
        );
        record_network_audit(
            &audit,
            &request.extension_id,
            &request.url,
            AuditStatus::Failed,
            Some("private/loopback/link-local IP literal is not allowed"),
        );
        return Err("Extension network request denied: private or loopback address".to_string());
    }

    let method = reqwest::Method::from_bytes(request.method.as_bytes())
        .map_err(|error| format!("Invalid HTTP method: {error}"))?;
    if !matches!(method, reqwest::Method::GET | reqwest::Method::POST | reqwest::Method::PUT | reqwest::Method::PATCH | reqwest::Method::DELETE) {
        return Err("HTTP method is not allowed for extension network proxy".to_string());
    }

    let client = reqwest::Client::new();
    let mut builder = client.request(method, url);
    for (name, value) in request.headers {
        if safe_header(&name) {
            builder = builder.header(name, value);
        }
    }
    if let Some(body) = request.body {
        builder = builder.json(&body);
    }
    let response = builder.send().await.map_err(|error| format!("Network proxy failed: {error}"))?;
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .filter_map(|(name, value)| value.to_str().ok().map(|value| (name.to_string(), value.to_string())))
        .collect();
    let body = response.text().await.map_err(|error| format!("Failed to read proxy response: {error}"))?;
    record_network_audit(&audit, &request.extension_id, &request.url, AuditStatus::Success, None);
    Ok(ExtensionNetworkResponse { status, headers, body })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssrf_blocks_private_loopback_and_link_local_ip_literals() {
        for url in [
            "http://127.0.0.1:8080/admin",
            "http://10.0.0.1/x",
            "http://172.16.0.1/x",
            "http://192.168.1.1/x",
            "http://169.254.169.254/latest/meta-data",
            "http://0.0.0.0/x",
            "http://[::1]:3000/x",
            "http://[fc00::1]/x",
            "http://[fe80::1]/x",
        ] {
            assert!(
                is_blocked_private_ip(&Url::parse(url).unwrap()),
                "expected {url} to be blocked as a private/loopback IP literal"
            );
        }
    }

    #[test]
    fn ssrf_does_not_block_public_ips_or_domains() {
        for url in ["http://8.8.8.8/x", "http://api.example.com/x", "http://example.com:8080/x"] {
            assert!(
                !is_blocked_private_ip(&Url::parse(url).unwrap()),
                "expected {url} to be allowed (domain is not DNS-resolved)"
            );
        }
    }
}
