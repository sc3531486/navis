//! Root config version guard.
//!
//! Navis Go is not released yet, so config files do not have a migration
//! chain. Existing config files may omit a version, but if a file declares one
//! it must match this root config version exactly.

use anyhow::{bail, Result};
use serde_json::Value;
use std::path::Path;

pub const CURRENT_CONFIG_VERSION: u32 = 1;

pub fn ensure_current_config_version(config: &Value, path: &Path) -> Result<()> {
    let Some(version) = config
        .get("configVersion")
        .or_else(|| config.get("rootConfigVersion"))
        .or_else(|| config.get("version"))
    else {
        return Ok(());
    };

    let parsed = parse_version(version)?;
    if parsed != CURRENT_CONFIG_VERSION {
        bail!(
            "配置文件根版本不匹配: 当前文件版本为 {}, Navis Go 需要版本 {}。请重建或手动更新配置文件: {}",
            parsed,
            CURRENT_CONFIG_VERSION,
            path.display()
        );
    }

    Ok(())
}

fn parse_version(value: &Value) -> Result<u32> {
    if let Some(number) = value.as_u64() {
        return Ok(number as u32);
    }

    if let Some(text) = value.as_str() {
        return text
            .trim()
            .parse::<u32>()
            .map_err(|_| anyhow::anyhow!("配置版本必须是整数，收到 '{}'", text));
    }

    bail!("配置版本必须是整数，收到 {}", value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_config_without_version() {
        ensure_current_config_version(&json!({"gateway": {}}), Path::new("navis.json")).unwrap();
    }

    #[test]
    fn accepts_current_numeric_version() {
        ensure_current_config_version(
            &json!({"configVersion": CURRENT_CONFIG_VERSION}),
            Path::new("navis.json"),
        )
        .unwrap();
    }

    #[test]
    fn accepts_current_string_version() {
        ensure_current_config_version(&json!({"version": "1"}), Path::new("navis.json")).unwrap();
    }

    #[test]
    fn rejects_other_versions() {
        let err =
            ensure_current_config_version(&json!({"configVersion": 99}), Path::new("navis.json"))
                .unwrap_err();
        assert!(err.to_string().contains("配置文件根版本不匹配"));
    }
}
