// 统一扩展协议：extension.json 清单解析。
// 字段对齐通用运行时外壳规范：id/name/version/main/ui + contributes 贡献点。
use serde::Deserialize;
use serde::ser::SerializeStruct;
use std::path::Path;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ExtensionManifest {
    /// 唯一扩展 ID；缺省回退到 name
    pub id: Option<String>,
    pub name: String,
    pub version: String,
    /// 后端进程入口（.mjs/.js/.cjs/.py 或可执行文件）
    pub main: Option<String>,
    /// 前端 UI 入口（打包后的 ESM 模块路径）
    pub ui: Option<String>,
    #[serde(default)]
    pub contributes: ContributionPoints,
    #[serde(default)]
    pub permissions: serde_json::Value,
}

// 兼容旧清单：只有 name 时视为 id
impl ExtensionManifest {
    pub fn plugin_id(&self) -> String {
        self.id.clone().unwrap_or_else(|| self.name.clone())
    }

    /// 加载目录下所有 extension.json；目录直接子级各为一个扩展
    pub fn load_from_dir(dir: &Path) -> Vec<Self> {
        let mut manifests = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let manifest_path = entry.path().join("extension.json");
                if manifest_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                        if let Ok(manifest) = serde_json::from_str::<Self>(&content) {
                            manifests.push(manifest);
                        }
                    }
                }
            }
        }
        manifests
    }
}

// 手动实现 Serialize 以让 navis_list_extensions 返回完整清单
impl serde::Serialize for ExtensionManifest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("ExtensionManifest", 6)?;
        s.serialize_field("id", &self.plugin_id())?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("version", &self.version)?;
        s.serialize_field("main", &self.main)?;
        s.serialize_field("ui", &self.ui)?;
        s.serialize_field("contributes", &self.contributes)?;
        s.end()
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ContributionPoints {
    /// 声明挂载到宿主插槽的条目（组件名由 UI 插件 apply() 绑定）
    #[serde(default)]
    pub slots: Vec<SlotContribution>,
    /// 扩展向系统发布的新插槽名（供其他扩展挂载）
    #[serde(default, rename = "providesSlots")]
    pub provides_slots: Vec<String>,
    /// 传统命令贡献点
    #[serde(default)]
    pub commands: Vec<CommandContribution>,
    /// 工具能力声明（供 Agent 网关注册）
    #[serde(default)]
    pub tools: Vec<ToolContribution>,
    /// Agent 管线拦截钩子声明
    #[serde(default, rename = "pipelineHooks")]
    pub pipeline_hooks: Vec<PipelineHookContribution>,
}

impl serde::Serialize for ContributionPoints {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("ContributionPoints", 5)?;
        s.serialize_field("slots", &self.slots)?;
        s.serialize_field("providesSlots", &self.provides_slots)?;
        s.serialize_field("commands", &self.commands)?;
        s.serialize_field("tools", &self.tools)?;
        s.serialize_field("pipelineHooks", &self.pipeline_hooks)?;
        s.end()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct SlotContribution {
    pub id: String,
    pub target: String,
    /// 组件名，由 ExtensionUI 的组件注册表解析
    pub component: Option<String>,
    #[serde(default = "default_priority")]
    pub priority: u32,
}

fn default_priority() -> u32 { 100 }

impl serde::Serialize for SlotContribution {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("SlotContribution", 4)?;
        s.serialize_field("id", &self.id)?;
        s.serialize_field("target", &self.target)?;
        s.serialize_field("component", &self.component)?;
        s.serialize_field("priority", &self.priority)?;
        s.end()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct CommandContribution {
    pub id: String,
    pub title: String,
}

impl serde::Serialize for CommandContribution {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("CommandContribution", 2)?;
        s.serialize_field("id", &self.id)?;
        s.serialize_field("title", &self.title)?;
        s.end()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ToolContribution {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<serde_json::Value>,
}

impl serde::Serialize for ToolContribution {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("ToolContribution", 3)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("description", &self.description)?;
        s.serialize_field("parameters", &self.parameters)?;
        s.end()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct PipelineHookContribution {
    pub hook: String,
    pub handler: String,
}

impl serde::Serialize for PipelineHookContribution {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("PipelineHookContribution", 2)?;
        s.serialize_field("hook", &self.hook)?;
        s.serialize_field("handler", &self.handler)?;
        s.end()
    }
}