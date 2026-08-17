//! Extension static resource boundary.
//!
//! This module resolves only files below an extension's `ExtensionUI` directory. It is
//! intentionally independent from manifest parsing and rendering so resource
//! authorization can be reused by an asset URL producer or a future protocol
//! adapter without widening the extension domain contract.

use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};

const EXTENSION_UI_DIRECTORY: &str = "ExtensionUI";

/// Resolve a manifest entry relative to the extension root. Static UI entries
/// are deliberately restricted to the extension's `ExtensionUI` directory.
pub fn resolve_extension_manifest_entry(extension_root: &Path, entry: &str) -> Result<PathBuf> {
    let path = Path::new(entry);
    let ui_relative = path
        .strip_prefix(EXTENSION_UI_DIRECTORY)
        .map_err(|_| anyhow::anyhow!("Extension entry must be located below the ExtensionUI directory"))?;
    resolve_extension_ui_resource(extension_root, ui_relative)
}

/// Resolve a static extension UI resource below `<extension_root>/ExtensionUI`.
///
/// The returned path is canonicalized and guaranteed to remain below the
/// canonical UI root. Every path component is checked for symlinks so the
/// helper does not turn a seemingly local resource into an escape hatch.
pub fn resolve_extension_ui_resource(
    extension_root: &Path,
    relative_path: &Path,
) -> Result<PathBuf> {
    validate_relative_resource_path(relative_path)?;

    if fs::symlink_metadata(extension_root)
        .with_context(|| {
            format!(
                "Failed to inspect extension root: {}",
                extension_root.display()
            )
        })?
        .file_type()
        .is_symlink()
    {
        bail!("Extension root cannot be a symbolic link");
    }

    let extension_root = extension_root.canonicalize().with_context(|| {
        format!(
            "Failed to canonicalize extension root: {}",
            extension_root.display()
        )
    })?;
    let ui_root = extension_root.join(EXTENSION_UI_DIRECTORY);
    reject_symlink_components(&extension_root, &ui_root)?;
    let ui_root = ui_root.canonicalize().with_context(|| {
        format!(
            "Extension UI directory does not exist: {}",
            ui_root.display()
        )
    })?;

    let candidate = ui_root.join(relative_path);
    reject_symlink_components(&ui_root, &candidate)?;
    let candidate = candidate.canonicalize().with_context(|| {
        format!(
            "Extension UI resource does not exist: {}",
            candidate.display()
        )
    })?;

    if !candidate.starts_with(&ui_root) {
        bail!("Extension UI resource escapes the UI directory");
    }
    if !candidate.is_file() {
        bail!(
            "Extension UI resource is not a file: {}",
            candidate.display()
        );
    }

    Ok(candidate)
}

fn validate_relative_resource_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        bail!("Extension UI resource path is empty");
    }

    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            bail!("Extension UI resource path must contain only relative normal components");
        }
    }

    Ok(())
}

fn reject_symlink_components(root: &Path, target: &Path) -> Result<()> {
    let relative = target
        .strip_prefix(root)
        .with_context(|| format!("Resource is outside the UI root: {}", target.display()))?;
    let mut current = root.to_path_buf();

    for component in relative.components() {
        let Component::Normal(name) = component else {
            bail!("Extension UI resource path contains an invalid component");
        };
        current.push(name);
        let metadata = fs::symlink_metadata(&current).with_context(|| {
            format!(
                "Failed to inspect extension UI resource: {}",
                current.display()
            )
        })?;
        if metadata.file_type().is_symlink() {
            bail!(
                "Extension UI resources cannot contain symbolic links: {}",
                current.display()
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_ui_tree() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("temp directory");
        fs::create_dir_all(root.path().join("ExtensionUI/assets")).expect("ExtensionUI directory");
        fs::write(
            root.path().join("ExtensionUI/index.html"),
            "<link rel=\"stylesheet\" href=\"./styles.css\"><script src=\"./jquery.js\"></script><script src=\"./app.js\"></script>",
        )
        .expect("entry file");
        fs::write(root.path().join("ExtensionUI/assets/app.js"), "console.log('ok')").expect("asset file");
        fs::write(root.path().join("ExtensionUI/jquery.js"), "window.jQuery = {};\n").expect("jquery file");
        fs::write(
            root.path().join("ExtensionUI/styles.css"),
            "body { color: white; }\n",
        )
        .expect("style file");
        root
    }

    #[test]
    fn resolves_only_files_below_extension_ui_directory() {
        let root = setup_ui_tree();

        let resolved = resolve_extension_ui_resource(root.path(), Path::new("assets/app.js"))
            .expect("resource should resolve");

        assert_eq!(
            resolved,
            root.path().join("ExtensionUI/assets/app.js").canonicalize().unwrap()
        );
    }

    #[test]
    fn resolves_html_entry_and_relative_static_dependencies() {
        let root = setup_ui_tree();

        let html = resolve_extension_manifest_entry(root.path(), "ExtensionUI/index.html")
            .expect("HTML entry should resolve");
        let html_source = fs::read_to_string(html).expect("HTML source");
        assert!(html_source.contains("./styles.css"));
        assert!(html_source.contains("./jquery.js"));
        assert!(html_source.contains("./app.js"));

        for resource in ["styles.css", "jquery.js", "assets/app.js"] {
            assert!(
                resolve_extension_manifest_entry(root.path(), &format!("ExtensionUI/{resource}")).is_ok(),
                "static resource should resolve: {resource}"
            );
        }
    }

    #[test]
    fn resolves_manifest_entry_only_under_extension_ui() {
        let root = setup_ui_tree();

        let resolved = resolve_extension_manifest_entry(root.path(), "ExtensionUI/index.html")
            .expect("manifest entry should resolve");
        assert_eq!(
            resolved,
            root.path().join("ExtensionUI/index.html").canonicalize().unwrap()
        );

        assert!(resolve_extension_manifest_entry(root.path(), "index.html").is_err());
        assert!(resolve_extension_manifest_entry(root.path(), "ExtensionUI/../extension.json").is_err());
    }

    #[test]
    fn rejects_parent_and_absolute_paths() {
        let root = setup_ui_tree();

        for path in [Path::new("../extension.json"), Path::new("/etc/passwd")] {
            assert!(resolve_extension_ui_resource(root.path(), path).is_err());
        }
    }

    #[test]
    fn rejects_directories() {
        let root = setup_ui_tree();

        assert!(resolve_extension_ui_resource(root.path(), Path::new("assets")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_resources() {
        use std::os::unix::fs::symlink;

        let root = setup_ui_tree();
        symlink(
            root.path().join("ExtensionUI/index.html"),
            root.path().join("ExtensionUI/link.html"),
        )
        .expect("symlink resource");

        assert!(resolve_extension_ui_resource(root.path(), Path::new("link.html")).is_err());
    }
}
