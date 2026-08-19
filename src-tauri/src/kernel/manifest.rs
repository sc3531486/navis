use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct ExtensionManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub contributes: ContributionPoints,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ContributionPoints {
    #[serde(default)]
    pub slots: Vec<SlotContribution>,
    #[serde(default)]
    pub commands: Vec<CommandContribution>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SlotContribution {
    pub id: String,
    pub target: String,
    #[serde(default = "default_priority")]
    pub priority: u32,
}

fn default_priority() -> u32 { 100 }

#[derive(Debug, Deserialize, Clone)]
pub struct CommandContribution {
    pub id: String,
    pub title: String,
}

impl ExtensionManifest {
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
