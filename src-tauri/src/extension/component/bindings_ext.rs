//! navis:ext 世界绑定（wasmtime bindgen 生成代码，勿手改）。
//!
//! 生成内容：`navis_ext::{lifecycle, message}` 导出接口模块（含 `Guest` 包装结构），
//! 以及世界结构 `Ext`（含 `navis_ext_lifecycle()` / `navis_ext_message()` 访问器）。
//! 契约定义见 `src-tauri/wit/navis.wit`。

#![allow(warnings)]

wasmtime::component::bindgen!({
    path: "wit/ext",
    world: "navis:ext/ext",
});
