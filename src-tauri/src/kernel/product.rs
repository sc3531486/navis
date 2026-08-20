// 产品组装配置：根级 <product>.json 声明当前运行时装配的扩展清单。
// 产品即形态（Profile）——同一 Navis 宿主可按配置装配不同客户端（navis-code / teller-system 等），
// 新增产品仅需新增 extensions/ 下的扩展与一个 <product>.json，宿主零改动。
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::manifest::ExtensionManifest;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ProductConfig {
    pub id: String,
    pub name: String,
    /// 产品壳扩展 id（负责 root/overlay 布局）
    #[serde(default)]
    pub shell: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// 本产品装配的业务扩展 id 列表
    #[serde(default)]
    pub extensions: Vec<String>,
}

impl ProductConfig {
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    /// 本产品装配的扩展 id 集合（含产品壳）
    pub fn active_extension_ids(&self) -> Vec<String> {
        let mut ids = self.extensions.clone();
        if let Some(shell) = &self.shell {
            if !ids.iter().any(|i| i == shell) {
                ids.push(shell.clone());
            }
        }
        ids
    }

    /// 判断清单是否属于本产品
    pub fn includes(&self, manifest: &ExtensionManifest) -> bool {
        self.active_extension_ids().contains(&manifest.plugin_id())
    }
}