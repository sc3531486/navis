//! Extension 安装/卸载
//!
//! 基于设计文档 §07 实现扩展的安装与卸载逻辑。
//!
//! 职责：
//! - 从指定路径安装扩展（读取 extension.json，校验，复制到扩展目录）
//! - 卸载扩展（禁用、删除文件、清理数据）

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::Utc;

use super::loader::{is_valid_extension_id, ExtensionLoader};
use super::models::{ExtensionState, ExtensionStatus};
use super::store::ExtensionStore;

/// 扩展安装器
///
/// 负责扩展的安装（从路径读取、校验、复制）和卸载（删除、清理）。
pub struct ExtensionInstaller {
    /// 扩展安装根目录
    extensions_dir: PathBuf,
    /// 扩展状态存储
    store: std::sync::Arc<ExtensionStore>,
    /// 扩展加载器
    loader: ExtensionLoader,
    /// 事件总线
    #[allow(dead_code)]
    event_bus: std::sync::Arc<dyn crate::kernel::EventBus>,
}

impl ExtensionInstaller {
    /// 创建新的扩展安装器
    ///
    /// # Arguments
    /// * `extensions_dir` - 扩展安装根目录
    /// * `store` - 扩展状态存储
    /// * `event_bus` - 事件总线
    pub fn new(
        extensions_dir: PathBuf,
        store: std::sync::Arc<ExtensionStore>,
        event_bus: std::sync::Arc<dyn crate::kernel::EventBus>,
    ) -> Self {
        tracing::info!(extensions_dir = %extensions_dir.display(), "Creating ExtensionInstaller");
        Self {
            extensions_dir,
            store,
            loader: ExtensionLoader::new(),
            event_bus,
        }
    }

    fn install_path_for(&self, extension_id: &str) -> Result<PathBuf> {
        if !is_valid_extension_id(extension_id) {
            bail!("Extension ID contains unsupported characters");
        }

        fs::create_dir_all(&self.extensions_dir)?;
        let root = self.extensions_dir.canonicalize()?;
        let path = root.join(extension_id);
        if path.parent() != Some(root.as_path()) {
            bail!("Extension install path is outside the extension directory");
        }
        Ok(path)
    }

    /// 安装扩展
    ///
    /// 从指定路径安装扩展：
    /// 1. 读取 extension.json
    /// 2. 校验清单
    /// 3. 检查 ID 冲突
    /// 4. 复制到扩展目录
    /// 5. 写入扩展状态存储
    ///
    /// # Arguments
    /// * `source_path` - 扩展源路径（包含 extension.json 的目录或 .zip 文件）
    pub fn install(&self, source_path: &Path) -> Result<ExtensionState> {
        tracing::info!(source = %source_path.display(), "Installing extension");

        // 加载并校验清单
        let manifest = self
            .loader
            .load_manifest(source_path)
            .context("Failed to load extension manifest")?;

        let extension_id = manifest.id.clone();
        tracing::debug!(extension_id = %extension_id, "Manifest loaded and validated");

        // 检查是否已注册
        if self.store.contains(&extension_id) {
            return Err(anyhow::anyhow!(
                "Extension '{}' is already installed",
                extension_id
            ));
        }

        // 创建扩展安装目录
        let install_path = self.install_path_for(&extension_id)?;
        if install_path.exists() {
            fs::remove_dir_all(&install_path)
                .context("Failed to remove existing extension directory")?;
        }

        // 复制扩展文件
        self.copy_extension(source_path, &install_path)
            .context("Failed to copy extension files")?;

        // 写入扩展状态存储
        let state = ExtensionState {
            id: extension_id.clone(),
            status: ExtensionStatus::Installed,
            manifest,
            install_path: install_path.clone(),
            installed_at: Utc::now(),
            enabled_at: None,
            error: None,
        };

        self.store.register(state.clone())?;

        tracing::info!(extension_id = %extension_id, "Extension installed successfully");
        Ok(state)
    }

    /// 卸载扩展
    ///
    /// 1. 从扩展状态存储中获取扩展状态
    /// 2. 如果已启用，先禁用（lifecycle 负责）
    /// 3. 删除扩展文件
    /// 4. 从扩展状态存储移除
    ///
    /// # Arguments
    /// * `extension_id` - 扩展 ID
    pub fn uninstall(&self, extension_id: &str) -> Result<()> {
        tracing::info!(extension_id = %extension_id, "Uninstalling extension");

        let state = self
            .store
            .get(extension_id)
            .ok_or_else(|| anyhow::anyhow!("Extension '{}' not found", extension_id))?;

        match state.status {
            ExtensionStatus::Installed | ExtensionStatus::Disabled => {}
            ExtensionStatus::Enabled => {
                return Err(anyhow::anyhow!(
                    "Extension '{}' is currently enabled. Disable it first before uninstalling.",
                    extension_id
                ));
            }
            status => {
                return Err(anyhow::anyhow!(
                    "Extension '{}' cannot be uninstalled while it is {:?}",
                    extension_id,
                    status
                ));
            }
        }

        let expected_path = self.install_path_for(extension_id)?;
        if state.install_path != expected_path {
            bail!("Extension install path does not match the extension directory");
        }

        // 删除扩展文件
        if expected_path.exists() {
            fs::remove_dir_all(&expected_path).context("Failed to remove extension directory")?;
            tracing::debug!(
                extension_id = %extension_id,
                path = %expected_path.display(),
                "Extension files removed"
            );
        }

        // 从扩展状态存储移除
        self.store.unregister(extension_id)?;

        tracing::info!(extension_id = %extension_id, "Extension uninstalled successfully");
        Ok(())
    }

    /// 获取扩展安装根目录
    pub fn extensions_dir(&self) -> &Path {
        &self.extensions_dir
    }

    /// 获取已安装的扩展路径
    pub fn get_extension_path(&self, extension_id: &str) -> PathBuf {
        self.extensions_dir.join(extension_id)
    }

    /// 复制扩展文件到安装目录
    fn copy_extension(&self, source: &Path, dest: &Path) -> Result<()> {
        if !source.exists() {
            return Err(anyhow::anyhow!(
                "Source path does not exist: {}",
                source.display()
            ));
        }

        if source.is_dir() {
            Self::copy_dir_recursive(source, dest)?;
        } else {
            return Err(anyhow::anyhow!(
                "Source path is not a directory: {}",
                source.display()
            ));
        }

        Ok(())
    }

    /// 递归复制目录
    fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            let dest_path = dst.join(entry.file_name());

            if ty.is_symlink() {
                bail!(
                    "Extension files cannot contain symbolic links: {}",
                    entry.path().display()
                );
            }
            if ty.is_dir() {
                Self::copy_dir_recursive(&entry.path(), &dest_path)?;
            } else if ty.is_file() {
                fs::copy(entry.path(), &dest_path)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::OnceLock;
    use tokio::runtime::Runtime;

    fn test_runtime_handle() -> tokio::runtime::Handle {
        static RUNTIME: OnceLock<Runtime> = OnceLock::new();
        RUNTIME
            .get_or_init(|| Runtime::new().expect("test tokio runtime"))
            .handle()
            .clone()
    }

    fn create_test_manifest_json(id: &str) -> String {
        serde_json::json!({
            "id": id,
            "name": format!("Extension {}", id),
            "version": "1.0.0",
            "description": "test extension",
            "author": "tester",
            "permissions": {
                "filesystem": [],
                "terminal": [],
                "network": [],
                "ipc": [],
                "events": [],
                "resources": { "max_memory_mb": 256, "max_cpu_percent": 30.0, "timeout_ms": 10000 }
            },
            "contributes": {}
        })
        .to_string()
    }

    fn setup_test_extension(dir: &Path, id: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("extension.json"), create_test_manifest_json(id)).unwrap();
    }

    #[test]
    fn test_install_extension() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        let extensions_dir = temp_dir.path().join("extensions");

        setup_test_extension(&source_dir, "com.test.install");

        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let installer =
            ExtensionInstaller::new(extensions_dir.clone(), Arc::clone(&store), event_bus);

        let state = installer.install(&source_dir).unwrap();
        assert_eq!(state.id, "com.test.install");
        assert_eq!(state.status, ExtensionStatus::Installed);

        // 验证文件被复制
        let installed_manifest = extensions_dir
            .join("com.test.install")
            .join("extension.json");
        assert!(installed_manifest.exists());

        // 验证注册表
        assert!(store.contains("com.test.install"));
    }

    #[test]
    fn test_install_duplicate_fails() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        let extensions_dir = temp_dir.path().join("extensions");

        setup_test_extension(&source_dir, "com.test.dup");

        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let installer = ExtensionInstaller::new(extensions_dir, Arc::clone(&store), event_bus);

        installer.install(&source_dir).unwrap();
        let result = installer.install(&source_dir);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("already installed"));
    }

    #[test]
    fn test_uninstall_extension() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        let extensions_dir = temp_dir.path().join("extensions");

        setup_test_extension(&source_dir, "com.test.uninstall");

        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let installer =
            ExtensionInstaller::new(extensions_dir.clone(), Arc::clone(&store), event_bus);

        installer.install(&source_dir).unwrap();
        assert!(store.contains("com.test.uninstall"));

        installer.uninstall("com.test.uninstall").unwrap();
        assert!(!store.contains("com.test.uninstall"));

        // 验证文件被删除
        let extension_dir = extensions_dir.join("com.test.uninstall");
        assert!(!extension_dir.exists());
    }

    #[test]
    fn test_uninstall_enabled_extension_fails() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        let extensions_dir = temp_dir.path().join("extensions");

        setup_test_extension(&source_dir, "com.test.enabled");

        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let installer =
            ExtensionInstaller::new(extensions_dir, Arc::clone(&store), Arc::clone(&event_bus));

        installer.install(&source_dir).unwrap();

        // 模拟启用
        store
            .update_status("com.test.enabled", ExtensionStatus::Enabled, None)
            .unwrap();

        let result = installer.uninstall("com.test.enabled");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Disable it first"));
    }

    #[test]
    fn test_uninstall_nonexistent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let installer =
            ExtensionInstaller::new(temp_dir.path().join("extensions"), store, event_bus);

        let result = installer.uninstall("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_install_source_not_found() {
        let temp_dir = tempfile::tempdir().unwrap();
        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let installer =
            ExtensionInstaller::new(temp_dir.path().join("extensions"), store, event_bus);

        let result = installer.install(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_install_missing_manifest() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source_dir = temp_dir.path().join("empty_extension");
        fs::create_dir_all(&source_dir).unwrap();

        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let installer =
            ExtensionInstaller::new(temp_dir.path().join("extensions"), store, event_bus);

        let result = installer.install(&source_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_install_invalid_manifest() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source_dir = temp_dir.path().join("bad_extension");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("extension.json"), "{invalid json!!!}").unwrap();

        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let installer =
            ExtensionInstaller::new(temp_dir.path().join("extensions"), store, event_bus);

        let result = installer.install(&source_dir);
        assert!(result.is_err());
    }
}
