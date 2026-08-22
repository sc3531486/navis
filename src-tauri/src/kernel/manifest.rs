// 通用扩展协议：extension.json 泛型清单解析。
// 字段对齐通用运行时规范：id/name/version/main/ui + contributes 泛型贡献点。
// 框架核心不做业务字段硬编码，所有贡献点通过 JSON Map 泛型透传给前端或运行时。
use serde::{Deserialize, Serialize};
use serde::ser::SerializeStruct;
use std::path::Path;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ExtensionManifest {
    /// 唯一扩展 ID；缺省回退到 name
    pub id: Option<String>,
    pub name: String,
    pub version: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub publisher: Option<String>,
    pub description: Option<String>,
    /// 后端进程入口（.mjs/.js/.cjs/.py 或可执行文件）
    pub main: Option<String>,
    /// 前端 UI 入口（打包后的 ESM 模块路径）
    pub ui: Option<String>,
    #[serde(default)]
    pub contributes: serde_json::Value,
    #[serde(default)]
    pub permissions: serde_json::Value,
}

impl ExtensionManifest {
    pub fn plugin_id(&self) -> String {
        self.id.clone().unwrap_or_else(|| self.name.clone())
    }

    /// 获取清单中声明的命令列表（通用机制）
    pub fn commands(&self) -> Vec<CommandContribution> {
        if let Some(commands_val) = self.contributes.get("commands") {
            if let Ok(cmds) = serde_json::from_value::<Vec<CommandContribution>>(commands_val.clone()) {
                return cmds;
            }
        }
        Vec::new()
    }

    /// 获取清单中声明的插槽列表（通用机制）
    pub fn slots(&self) -> Vec<SlotContribution> {
        if let Some(slots_val) = self.contributes.get("slots") {
            if let Ok(slots) = serde_json::from_value::<Vec<SlotContribution>>(slots_val.clone()) {
                return slots;
            }
        }
        Vec::new()
    }

    /// 递归加载目录下所有 extension.json（支持单层扁平与多层套件分组目录结构）
    pub fn load_from_dir(dir: &Path) -> Vec<Self> {
        let mut manifests = Vec::new();
        Self::collect_manifests(dir, 0, 3, &mut manifests);
        manifests
    }

    fn collect_manifests(dir: &Path, depth: usize, max_depth: usize, out: &mut Vec<Self>) {
        if depth > max_depth || !dir.exists() {
            return;
        }
        let manifest_path = dir.join("extension.json");
        if manifest_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                if let Ok(manifest) = serde_json::from_str::<Self>(&content) {
                    out.push(manifest);
                    return;
                }
            }
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    Self::collect_manifests(&path, depth + 1, max_depth, out);
                }
            }
        }
    }
}

// 手动实现 Serialize 以让 navis_list_extensions 返回完整清单
impl Serialize for ExtensionManifest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("ExtensionManifest", 9)?;
        s.serialize_field("id", &self.plugin_id())?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("version", &self.version)?;
        s.serialize_field("displayName", &self.display_name)?;
        s.serialize_field("publisher", &self.publisher)?;
        s.serialize_field("description", &self.description)?;
        s.serialize_field("main", &self.main)?;
        s.serialize_field("ui", &self.ui)?;
        s.serialize_field("contributes", &self.contributes)?;
        s.serialize_field("permissions", &self.permissions)?;
        s.end()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SlotContribution {
    pub id: String,
    pub target: String,
    pub component: Option<String>,
    #[serde(default = "default_priority")]
    pub priority: u32,
}

fn default_priority() -> u32 {
    100
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CommandContribution {
    pub id: String,
    pub title: String,
}