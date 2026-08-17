//! UI consumption surface for the extension HostView contract.
//!
//! Renderer and zone capabilities are owned by the extension contribution
//! model. The UI layer only consumes that contract when building projections.

use super::dto::UiExtensionViewDescriptor;
use crate::extension::host_view::{
    effective_view_zone, validate_extension_view, HTML_SANDBOX_RENDERER,
};
use crate::extension::models::ViewRegistration;
use crate::extension::resource::resolve_extension_manifest_entry;
use std::path::Path;

pub(crate) fn extension_resource_path(extension_root: &Path, entry: &str) -> Result<std::path::PathBuf, String> {
    resolve_extension_manifest_entry(extension_root, entry).map_err(|error| {
        format!("Failed to resolve extension resource entry '{}': {error}", entry)
    })
}

pub(crate) fn ui_extension_view_descriptor(
    view: &ViewRegistration,
    extension_root: &Path,
) -> Result<UiExtensionViewDescriptor, String> {
    validate_extension_view(view).map_err(|error| error.to_string())?;
    let zone = effective_view_zone(view)
        .ok_or_else(|| format!("View '{}' must declare zone", view.id))?
        .to_string();

    let resource_path = if view.renderer == HTML_SANDBOX_RENDERER {
        let entry = view
            .entry
            .as_deref()
            .ok_or_else(|| format!("View '{}' HTML renderer requires entry", view.id))?;
        let resource = extension_resource_path(extension_root, entry)?;
        Some(resource.display().to_string())
    } else {
        None
    };

    Ok(UiExtensionViewDescriptor {
        view_id: view.id.clone(),
        name: view.name.clone(),
        icon: view.icon.clone(),
        zone: zone.clone(),
        placement: zone,
        renderer: view.renderer.clone(),
        entry: view.entry.clone(),
        resource_path,
        config: view.config.clone(),
        allow_close: view.allow_close.unwrap_or(true),
        default_visible: view.default_visible.unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::host_view::{HOST_PANEL_RENDERER, HOST_VIEW_ZONE_RIGHT_WORKSPACE};
    use std::fs;

    fn test_view(renderer: &str, entry: Option<&str>) -> ViewRegistration {
        ViewRegistration {
            id: "test.view".to_string(),
            name: "Test View".to_string(),
            icon: Some("panel".to_string()),
            entry: entry.map(str::to_string),
            zone: Some(HOST_VIEW_ZONE_RIGHT_WORKSPACE.to_string()),
            placement: None,
            renderer: renderer.to_string(),
            config: Some(serde_json::json!({"density": "compact"})),
            activation_events: Vec::new(),
            allow_close: None,
            default_visible: None,
        }
    }

    #[test]
    fn projects_host_view_with_normalized_defaults() {
        let root = tempfile::tempdir().unwrap();
        let descriptor =
            ui_extension_view_descriptor(&test_view(HOST_PANEL_RENDERER, None), root.path())
                .unwrap();

        assert_eq!(descriptor.view_id, "test.view");
        assert_eq!(descriptor.zone, HOST_VIEW_ZONE_RIGHT_WORKSPACE);
        assert_eq!(descriptor.renderer, HOST_PANEL_RENDERER);
        assert_eq!(descriptor.entry, None);
        assert_eq!(descriptor.resource_path, None);
        assert!(descriptor.allow_close);
        assert!(!descriptor.default_visible);
    }

    #[test]
    fn projects_html_view_resource_path_once_and_rejects_missing_resources() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("ExtensionUI")).unwrap();
        fs::write(root.path().join("ExtensionUI/index.html"), "<main />").unwrap();

        let descriptor = ui_extension_view_descriptor(
            &test_view(HTML_SANDBOX_RENDERER, Some("ExtensionUI/index.html")),
            root.path(),
        )
        .unwrap();
        assert_eq!(descriptor.entry.as_deref(), Some("ExtensionUI/index.html"));
        let resource_path = descriptor.resource_path.as_deref().unwrap();
        assert!(
            std::path::Path::new(resource_path)
                .ends_with(std::path::Path::new("ExtensionUI/index.html")),
            "unexpected resource path: {resource_path}"
        );

        let error = ui_extension_view_descriptor(
            &test_view(HTML_SANDBOX_RENDERER, Some("ExtensionUI/missing.html")),
            root.path(),
        )
        .unwrap_err();
        assert!(error.contains("Failed to resolve extension resource entry"));
    }
}
