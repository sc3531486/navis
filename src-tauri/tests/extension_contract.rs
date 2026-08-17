//! 扩展固定目录契约集成测试（design/35 §三、design/36 §一A）。
//!
//! 校验仓库 `extensions/` 下每个扩展包：
//! - 目录名必须等于 manifest `id`
//! - `ExtensionUI/` 与 `ExtensionBackend/` 固定目录必须存在
//! - 前端入口（views/scripts）必须位于 `ExtensionUI/`，后端入口（backendServices）
//!   必须位于 `ExtensionBackend/`，组件轨（components）必须位于二者之一
//! - 声明的入口文件必须真实存在

use std::path::{Path, PathBuf};

use navis::extension::loader::{is_valid_extension_id, ExtensionLoader};

/// 仓库扩展统一根目录：`<repo>/extensions`。
fn repo_extensions_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap().join("extensions")
}

#[test]
fn all_repo_extensions_follow_fixed_directory_contract() {
    let root = repo_extensions_root();
    assert!(
        root.is_dir(),
        "仓库扩展根目录不存在: {}",
        root.display()
    );

    let loader = ExtensionLoader::new();
    let mut extension_dirs = std::fs::read_dir(&root)
        .expect("read extensions root")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_type()
                .map(|file_type| file_type.is_dir())
                .unwrap_or(false)
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    extension_dirs.sort();
    assert!(
        !extension_dirs.is_empty(),
        "extensions/ 下没有扩展包: {}",
        root.display()
    );

    for dir in extension_dirs {
        let manifest_path = dir.join("extension.json");
        if !manifest_path.is_file() {
            // 非扩展包目录（如辅助目录），跳过。
            continue;
        }

        let manifest = loader
            .load_manifest(&dir)
            .unwrap_or_else(|error| panic!("manifest 无效 {}: {error:#}", manifest_path.display()));
        let dir_name = dir
            .file_name()
            .expect("extension dir name")
            .to_string_lossy()
            .into_owned();

        // 1. 目录名必须等于 manifest id。
        assert_eq!(
            dir_name, manifest.id,
            "扩展目录名必须等于 manifest id: {}",
            dir.display()
        );
        assert!(
            is_valid_extension_id(&manifest.id),
            "非法扩展 id: {}",
            manifest.id
        );

        // 2. 固定目录必须存在。
        assert!(
            dir.join("ExtensionUI").is_dir(),
            "缺少 ExtensionUI/ 固定目录: {}",
            dir.display()
        );
        assert!(
            dir.join("ExtensionBackend").is_dir(),
            "缺少 ExtensionBackend/ 固定目录: {}",
            dir.display()
        );

        let contributes = &manifest.contributes;

        // 3. html:sandbox 视图入口必须位于 ExtensionUI/ 下且文件存在。
        if let Some(views) = &contributes.views {
            for view in views {
                if view.renderer == "html:sandbox" {
                    let entry = view.entry.as_deref().unwrap_or_default();
                    assert!(
                        entry.starts_with("ExtensionUI/"),
                        "html:sandbox 视图入口必须在 ExtensionUI/ 下: {entry} ({})",
                        dir.display()
                    );
                    assert!(
                        dir.join(entry).is_file(),
                        "视图入口文件缺失: {entry} ({})",
                        dir.display()
                    );
                }
            }
        }

        // 4. 脚本入口必须位于 ExtensionUI/ 下。
        if let Some(scripts) = &contributes.scripts {
            for script in scripts {
                assert!(
                    script.entry.starts_with("ExtensionUI/"),
                    "脚本入口必须在 ExtensionUI/ 下: {} ({})",
                    script.entry,
                    dir.display()
                );
            }
        }

        // 5. 组件轨入口必须位于 ExtensionUI/ 或 ExtensionBackend/ 下。
        if let Some(components) = &contributes.components {
            for component in components {
                assert!(
                    component.entry.starts_with("ExtensionUI/")
                        || component.entry.starts_with("ExtensionBackend/"),
                    "组件入口必须在 ExtensionUI/ 或 ExtensionBackend/ 下: {} ({})",
                    component.entry,
                    dir.display()
                );
            }
        }

        // 6. 后端服务入口必须位于 ExtensionBackend/ 下。
        if let Some(services) = &contributes.backend_services {
            for service in services {
                assert!(
                    service.entry.starts_with("ExtensionBackend/"),
                    "后端服务入口必须在 ExtensionBackend/ 下: {} ({})",
                    service.entry,
                    dir.display()
                );
            }
        }
    }
}