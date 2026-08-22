// 核心机制集成测试：进程管理器 + IPC 网关 + 沙箱 + 清单解析 + 产品装配闭环。
use navis_lib::core::ipc_bridge::TransportRouter;
use navis_lib::core::sandbox::{Capability, PermissionToken, Sandbox};
use navis_lib::kernel::manifest::ExtensionManifest;
use navis_lib::kernel::product::ProductConfig;
use serde_json::json;
use std::path::PathBuf;

fn extensions_root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("extensions")
}

fn demo_extension_dir() -> PathBuf {
    extensions_root().join("shared").join("navis-demo")
}

#[test]
fn manifest_parses_new_protocol() {
    let dir = extensions_root();
    // 递归目录下应能发现 navis-demo（含泛型清单字段）
    let manifests = ExtensionManifest::load_from_dir(&dir);
    let m = manifests
        .iter()
        .find(|m| m.plugin_id() == "navis-demo")
        .expect("navis-demo manifest not found");
    assert!(m.main.as_deref().is_some());
    assert_eq!(m.contributes["tools"].as_array().unwrap().len(), 2);
    assert_eq!(m.slots().len(), 1);
    assert_eq!(m.contributes["pipelineHooks"].as_array().unwrap().len(), 1);
    assert_eq!(m.contributes["providesSlots"].as_array().unwrap().len(), 1);

    // Serialize 应输出完整清单结构（前端发现依赖完整清单）
    let json = serde_json::to_value(m).unwrap();
    assert_eq!(json["id"].as_str().unwrap(), "navis-demo");
    assert!(json["contributes"]["tools"].as_array().is_some());
    assert_eq!(json["contributes"]["pipelineHooks"].as_array().unwrap().len(), 1);

    // 套件下的业务扩展清单也应能递归解析（navis-editor）
    let editor = manifests
        .iter()
        .find(|m| m.plugin_id() == "navis-editor")
        .expect("navis-editor manifest not found");
    assert_eq!(editor.contributes["tools"].as_array().unwrap().len(), 4);
    assert_eq!(editor.slots().len(), 1);
    assert_eq!(editor.contributes["providesSlots"].as_array().unwrap().len(), 2);
}

#[test]
fn product_config_parses() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let cfg = ProductConfig::load_from_file(&root.join("navis-code.json")).unwrap();
    assert_eq!(cfg.id, "navis-code");
    assert_eq!(cfg.shell.as_deref(), Some("navis-code"));
    let active = cfg.active_extension_ids();
    assert!(active.contains(&"navis-code".to_string()));
    assert!(active.contains(&"navis-session".to_string()));

    // teller-system 示例：新产品只需一个配置 + extensions/ 下新增扩展，宿主零改动
    let teller = ProductConfig::load_from_file(&root.join("teller-system.json")).unwrap();
    assert_eq!(teller.id, "teller-system");
    assert_eq!(teller.active_extension_ids(), vec!["teller-system-shell".to_string()]);
}

#[test]
fn sandbox_dynamic_acl() {
    let sandbox = Sandbox::new();
    sandbox.grant("navis-demo", &["fs.read".to_string(), "network".to_string()]);
    let token = PermissionToken {
        plugin_id: "navis-demo".to_string(),
        capabilities: vec![],
    };
    // 已授予 -> 通过
    assert!(sandbox.authorize(&token, Capability::FsRead, "read config").is_ok());
    // 未授予 -> 拒绝并审计
    assert!(sandbox.authorize(&token, Capability::ShellExec, "run shell").is_err());
    let log = sandbox.audit_log();
    assert_eq!(log.len(), 2);
    assert!(log.iter().any(|e| !e.allowed && e.capability == "ShellExec"));
}

#[tokio::test]
async fn transport_router_routes_to_demo_backend() {
    let router = TransportRouter::new();
    let main = "./ExtensionBackend/main.mjs";
    let cwd = demo_extension_dir();
    router
        .ensure_plugin_process_async("navis-demo", main, Some(&cwd))
        .await
        .unwrap();

    // 等待子进程就绪
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;

    // 同步 RPC
    let res = router
        .send_rpc("navis-demo", "demo.ping", json!({}))
        .await
        .expect("demo.ping failed");
    assert_eq!(res["result"]["pong"], json!(true), "unexpected: {res}");

    // 工具路由（tool.add）
    let res = router
        .send_rpc("navis-demo", "tool.add", json!({ "a": 2, "b": 5 }))
        .await
        .expect("tool.add failed");
    assert_eq!(res["result"]["sum"], json!(7), "unexpected: {res}");

    // 进程存活清单
    let running = router.list_running().await;
    assert!(running.contains(&"navis-demo".to_string()));

    router.kill("navis-demo").await.unwrap();
}
