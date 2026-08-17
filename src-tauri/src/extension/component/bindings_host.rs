//! navis:host 世界绑定（wasmtime bindgen 生成代码，勿手改）。
//!
//! 生成内容：`navis_host::{types, operation, context, storage, network, event, log}`
//! 接口模块（含 `Host` trait），以及世界结构 `Host`（`add_to_linker` / `instantiate`）。
//! 契约定义见 `src-tauri/wit/navis.wit`。

#![allow(warnings)]

wasmtime::component::bindgen!({
    path: "wit/host",
    world: "navis:host/host",
});
