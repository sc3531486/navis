//! Extension UI contribution contract.
//!
//! This module owns the built-in renderer and open zone capabilities accepted
//! from extension manifests. It contains no Tauri DTO or frontend rendering code.

use std::collections::HashSet;

use anyhow::{bail, Result};

use super::models::ViewRegistration;

/// Built-in renderer for views rendered by Navis Go itself.
pub(crate) const HOST_PANEL_RENDERER: &str = "host:panel";
/// Sandboxed static HTML renderer for extension-provided UI.
pub(crate) const HTML_SANDBOX_RENDERER: &str = "html:sandbox";

pub(crate) const HOST_VIEW_ZONE_RIGHT_WORKSPACE: &str = "rightWorkspace";
pub(crate) const HOST_VIEW_ZONE_CHAT_ASIDE: &str = "chatAside";
pub(crate) const HOST_VIEW_ZONE_BOTTOM_DRAWER: &str = "bottomDrawer";
pub(crate) const HOST_VIEW_ZONE_SETTINGS_SECTION: &str = "settingsSection";
pub(crate) const HOST_VIEW_ZONE_DIALOG: &str = "dialog";

pub(crate) fn effective_view_zone(view: &ViewRegistration) -> Option<&str> {
    view.zone
        .as_deref()
        .filter(|zone| !zone.is_empty())
        .or_else(|| view.placement.as_deref().filter(|placement| !placement.is_empty()))
}

pub(crate) fn is_supported_extension_view_renderer(renderer: &str) -> bool {
    matches!(renderer, HOST_PANEL_RENDERER | HTML_SANDBOX_RENDERER)
}

pub(crate) fn is_builtin_extension_view_zone(zone: &str) -> bool {
    matches!(
        zone,
        HOST_VIEW_ZONE_RIGHT_WORKSPACE
            | HOST_VIEW_ZONE_CHAT_ASIDE
            | HOST_VIEW_ZONE_BOTTOM_DRAWER
            | HOST_VIEW_ZONE_SETTINGS_SECTION
            | HOST_VIEW_ZONE_DIALOG
    )
}

pub(crate) fn is_open_extension_view_zone(zone: &str) -> bool {
    let zone = zone.trim();
    if zone.is_empty() {
        return false;
    }
    is_builtin_extension_view_zone(zone)
        || zone.split_once(':').is_some_and(|(owner, local)| {
            !owner.trim().is_empty()
                && !local.trim().is_empty()
                && !owner.contains(char::is_whitespace)
                && !local.contains(char::is_whitespace)
        })
}

pub(crate) fn validate_extension_view(view: &ViewRegistration) -> Result<()> {
    let zone = effective_view_zone(view)
        .ok_or_else(|| anyhow::anyhow!("View '{}' must declare a zone", view.id))?;
    if !is_open_extension_view_zone(zone) {
        bail!("View '{}' uses invalid zone '{}'", view.id, zone);
    }

    if !is_supported_extension_view_renderer(&view.renderer) {
        bail!(
            "View '{}' uses unsupported renderer '{}'",
            view.id,
            view.renderer
        );
    }

    match view.renderer.as_str() {
        HOST_PANEL_RENDERER if view.entry.is_some() => {
            bail!("View '{}' host renderer must not declare entry", view.id)
        }
        HTML_SANDBOX_RENDERER => {
            let entry = view
                .entry
                .as_deref()
                .filter(|entry| !entry.is_empty())
                .ok_or_else(|| {
                    anyhow::anyhow!("View '{}' HTML renderer requires entry", view.id)
                })?;
            validate_relative_entry(entry)
                .map_err(|error| anyhow::anyhow!("View '{}': {error}", view.id))?;
        }
        _ => {}
    }

    Ok(())
}

/// 在 `validate_extension_view` 基础上追加声明语义：`{extId}:{zoneId}`
/// 形式的 view.zone 必须由本扩展声明（owner 命中本扩展 id 且 zoneId 在已声明集内），
/// 否则 fail-closed。
pub(crate) fn validate_extension_view_with_declared_zones(
    view: &ViewRegistration,
    extension_id: &str,
    declared_zone_ids: &HashSet<&str>,
) -> Result<()> {
    validate_extension_view(view)?;
    let Some(zone) = effective_view_zone(view) else {
        return Ok(());
    };
    if is_builtin_extension_view_zone(zone) {
        return Ok(());
    }
    let Some((owner, zone_id)) = zone.split_once(':') else {
        bail!(
            "View '{}' uses non-namespaced extension zone '{}'",
            view.id,
            zone
        );
    };
    if owner != extension_id {
        bail!(
            "View '{}' zone '{}' must reference this extension's zone namespace",
            view.id,
            zone
        );
    }
    if !declared_zone_ids.contains(zone_id) {
        bail!("View '{}' references undeclared zone '{}'", view.id, zone);
    }
    Ok(())
}

/// 校验扩展 zone 锚定 parent：必须命中内置 zone 常量集，或 `{extId}:{zoneId}`
/// 且 owner 为本扩展、zoneId 为本扩展已声明的 zone。
pub(crate) fn validate_extension_zone_anchor_parent(
    parent: &str,
    extension_id: &str,
    declared_zone_ids: &HashSet<&str>,
) -> Result<()> {
    let parent = parent.trim();
    if parent.is_empty() {
        bail!("anchor parent must not be empty");
    }
    if is_builtin_extension_view_zone(parent) {
        return Ok(());
    }
    let Some((owner, zone_id)) = parent.split_once(':') else {
        bail!("anchor parent '{parent}' must be a builtin zone or '{{extId}}:{{zoneId}}'");
    };
    if owner != extension_id {
        bail!("anchor parent '{parent}' must reference this extension's zone namespace");
    }
    if !declared_zone_ids.contains(zone_id) {
        bail!("anchor parent '{parent}' references undeclared zone");
    }
    Ok(())
}

pub(crate) fn validate_relative_entry(entry: &str) -> Result<()> {
    if entry.is_empty()
        || entry.contains('\\')
        || entry.contains("://")
        || entry.starts_with('/')
        || entry.starts_with('\0')
        || std::path::Path::new(entry).is_absolute()
    {
        bail!("entry must be a relative path using '/' separators");
    }

    let segments: Vec<&str> = entry.split('/').collect();
    // 仅 ExtensionUI 规范（设计 35 §3.2）。
    match segments.first().copied() {
        Some("ExtensionUI") => {}
        _ => bail!("entry must be located below the ExtensionUI directory"),
    }
    for segment in segments {
        if segment.is_empty() || segment == "." || segment == ".." || segment.contains(':') {
            bail!("entry contains an unsafe path segment");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_view(zone: Option<&str>, renderer: &str) -> ViewRegistration {
        ViewRegistration {
            id: "test.view".to_string(),
            name: "Test View".to_string(),
            icon: None,
            entry: None,
            zone: zone.map(str::to_string),
            placement: None,
            renderer: renderer.to_string(),
            config: None,
            activation_events: Vec::new(),
            allow_close: Some(true),
            default_visible: Some(false),
        }
    }

    #[test]
    fn view_without_zone_is_not_renderable() {
        let view = test_view(None, HOST_PANEL_RENDERER);
        assert_eq!(effective_view_zone(&view), None);
        assert!(validate_extension_view(&view).is_err());
    }

    #[test]
    fn supported_zones_render_host_and_html_views() {
        for placement in [
            HOST_VIEW_ZONE_RIGHT_WORKSPACE,
            HOST_VIEW_ZONE_CHAT_ASIDE,
            HOST_VIEW_ZONE_BOTTOM_DRAWER,
            HOST_VIEW_ZONE_SETTINGS_SECTION,
        ] {
            let host_view = test_view(Some(placement), HOST_PANEL_RENDERER);
            assert!(validate_extension_view(&host_view).is_ok(), "{placement}");

            let mut html_view = test_view(Some(placement), HTML_SANDBOX_RENDERER);
            html_view.entry = Some("ExtensionUI/index.html".into());
            assert!(validate_extension_view(&html_view).is_ok(), "{placement}");
        }
    }

    #[test]
    fn unknown_renderer_and_invalid_zone_are_rejected() {
        let renderer = test_view(
            Some(HOST_VIEW_ZONE_RIGHT_WORKSPACE),
            "extension:custom",
        );
        assert!(validate_extension_view(&renderer).is_err());

        let dynamic_zone = test_view(Some("sample.ext:floatingDock"), HOST_PANEL_RENDERER);
        assert!(validate_extension_view(&dynamic_zone).is_ok());

        let invalid_zone = test_view(Some("floating Dock"), HOST_PANEL_RENDERER);
        assert!(validate_extension_view(&invalid_zone).is_err());
    }

    #[test]
    fn html_renderer_requires_safe_relative_entry() {
        let mut view = test_view(
            Some(HOST_VIEW_ZONE_RIGHT_WORKSPACE),
            HTML_SANDBOX_RENDERER,
        );
        for entry in [
            None,
            Some("../index.html"),
            Some("/index.html"),
            Some("C:/index.html"),
            Some("https://example.com/index.html"),
            Some("ExtensionUI\\index.html"),
        ] {
            view.entry = entry.map(str::to_string);
            assert!(
                validate_extension_view(&view).is_err(),
                "unsafe entry: {entry:?}"
            );
        }

        view.entry = Some("ExtensionUI/index.html".into());
        assert!(validate_extension_view(&view).is_ok());
    }

    #[test]
    fn host_renderer_does_not_accept_entry() {
        let mut view = test_view(
            Some(HOST_VIEW_ZONE_RIGHT_WORKSPACE),
            HOST_PANEL_RENDERER,
        );
        view.entry = Some("index.html".into());
        assert!(validate_extension_view(&view).is_err());
    }

    #[test]
    fn namespaced_view_zone_must_be_declared_by_the_extension() {
        let declared: HashSet<&str> = ["customPanel", "chatDock"].into_iter().collect();

        let view = test_view(Some("sample.ext:customPanel"), HOST_PANEL_RENDERER);
        assert!(validate_extension_view_with_declared_zones(&view, "sample.ext", &declared).is_ok());

        let builtin = test_view(Some(HOST_VIEW_ZONE_RIGHT_WORKSPACE), HOST_PANEL_RENDERER);
        assert!(validate_extension_view_with_declared_zones(&builtin, "sample.ext", &declared).is_ok());

        let undeclared = test_view(Some("sample.ext:floatingDock"), HOST_PANEL_RENDERER);
        assert!(validate_extension_view_with_declared_zones(&undeclared, "sample.ext", &declared).is_err());

        let other_owner = test_view(Some("other.ext:customPanel"), HOST_PANEL_RENDERER);
        assert!(validate_extension_view_with_declared_zones(&other_owner, "sample.ext", &declared).is_err());
    }

    #[test]
    fn zone_anchor_parent_must_resolve_to_builtin_or_declared_zone() {
        let declared: HashSet<&str> = ["customPanel"].into_iter().collect();

        assert!(validate_extension_zone_anchor_parent(
            HOST_VIEW_ZONE_RIGHT_WORKSPACE,
            "sample.ext",
            &declared
        )
        .is_ok());
        assert!(validate_extension_zone_anchor_parent("sample.ext:customPanel", "sample.ext", &declared)
            .is_ok());

        assert!(validate_extension_zone_anchor_parent("sample.ext:missing", "sample.ext", &declared).is_err());
        assert!(validate_extension_zone_anchor_parent("other.ext:customPanel", "sample.ext", &declared).is_err());
        assert!(validate_extension_zone_anchor_parent("right Workspace", "sample.ext", &declared).is_err());
        assert!(validate_extension_zone_anchor_parent("", "sample.ext", &declared).is_err());
    }
}
