//! Tauri 应用启动壳
//!
//! 框架层：Tauri bootstrap + 扩展加载 + 状态装配
//! 业务代码通过扩展机制加载，不在这里硬编码

pub mod infra;

use std::fs;
use std::sync::{Arc, Mutex};
use chrono::Utc;
use tauri::Manager;
use crate::extension;
use crate::foundation::{config, stream};
use crate::kernel;
use crate::security::{auth, sandbox};

/// 扫描扩展源目录
fn scan_extension_source(
    extension_loader: &extension::ExtensionLoader,
    extension_store: &extension::ExtensionStore,
    source_dir: &std::path::Path,
    source_label: &str,
) -> anyhow::Result<()> {
    // ... 扫描逻辑
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 框架启动：EventBus → Storage → Auth → Sandbox → ExtensionLifecycle
    // 业务代码通过扩展加载，不在这里硬编码
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // 框架初始化
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
