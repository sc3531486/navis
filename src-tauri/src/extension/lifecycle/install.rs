//! 扩展安装/更新相关逻辑
//!
//! 包含 `update` 等高级生命周期操作。

use anyhow::Result;

use super::ExtensionLifecycle;
use crate::extension::models::ExtensionStatus;
use crate::kernel::{EventEnvelope, KernelContext, KernelScope};
use triomphe::Arc as SharedArc;

impl ExtensionLifecycle {
    /// 热更新扩展
    ///
    /// 流程：disable 旧版本 → 更新 manifest → enable 新版本 → 发送 extension.updated 事件
    ///
    /// 调用方负责在调用此方法前完成文件拷贝和 ExtensionStore manifest 更新。
    /// 本方法仅处理生命周期状态转换和运行时能力的重新注册。
    ///
    /// # Arguments
    /// * `extension_id` - 扩展 ID
    /// * `from_version` - 更新前版本号
    /// * `to_version` - 更新后版本号
    pub fn update(&self, extension_id: &str, from_version: &str, to_version: &str) -> Result<()> {
        tracing::info!(
            extension_id = %extension_id,
            from_version = %from_version,
            to_version = %to_version,
            "Updating extension"
        );

        let current = self
            .store
            .get(extension_id)
            .ok_or_else(|| anyhow::anyhow!("Extension '{}' not found for update", extension_id))?;

        if current.status != ExtensionStatus::Enabled {
            return Err(anyhow::anyhow!(
                "Extension '{}' must be enabled before update (current status: {:?})",
                extension_id,
                current.status
            ));
        }

        let _contributes = current.manifest.contributes.clone();

        // 1. Disable 旧版本
        self.disable(extension_id)?;

        // 2. 发送 extension.updated 事件（含版本信息）
        if let Err(error) = self.event_bus.emit(EventEnvelope::new(
            "extension.updated",
            KernelContext::new("extension", KernelScope::global()),
            Some(SharedArc::new(serde_json::json!({
                "extensionId": extension_id,
                "fromVersion": from_version,
                "toVersion": to_version
            }))),
        )) {
            tracing::warn!(
                event = "extension.updated",
                error = %error,
                "Failed to emit extension.updated event"
            );
        }

        // 3. 重新启用新版本
        self.enable(extension_id)?;

        tracing::info!(
            extension_id = %extension_id,
            from_version = %from_version,
            to_version = %to_version,
            "Extension updated successfully"
        );

        Ok(())
    }
}
