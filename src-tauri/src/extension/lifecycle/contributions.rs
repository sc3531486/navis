//! Runtime registrar for declarative UI contributions.
//!
//! The registrar owns the enabled-state projection for views, commands, menus,
//! and keybindings. It deliberately stores declarations only; execution stays
//! in the existing host contracts and frontend command dispatch.

use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};

use crate::extension::host_view::{
    validate_extension_view_with_declared_zones, validate_extension_zone_anchor_parent,
};
use crate::extension::models::{
    menu_targets, BuiltinAction, CommandRegistration, ExtensionContributes, KeybindingRegistration,
    InlineExtensionRegistration, MenuRegistration, ScriptRegistration, StatusBarItemRegistration,
    ToolbarItemRegistration, ViewRegistration, WorkModeRegistration, ZoneRegistration,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UiContributionRegistration {
    pub(crate) extension_id: String,
    pub(crate) views: Vec<ViewRegistration>,
    pub(crate) commands: Vec<CommandRegistration>,
    pub(crate) menus: Vec<MenuRegistration>,
    pub(crate) keybindings: Vec<KeybindingRegistration>,
    pub(crate) work_modes: Vec<WorkModeRegistration>,
    pub(crate) zones: Vec<ZoneRegistration>,
    pub(crate) scripts: Vec<ScriptRegistration>,
    pub(crate) toolbar_items: Vec<ToolbarItemRegistration>,
    pub(crate) statusbar_items: Vec<StatusBarItemRegistration>,
    pub(crate) inline_extensions: Vec<InlineExtensionRegistration>,
    pub(crate) configuration: Option<serde_json::Value>,
}

impl UiContributionRegistration {
    pub(crate) fn is_empty(&self) -> bool {
        self.views.is_empty()
            && self.commands.is_empty()
            && self.menus.is_empty()
            && self.keybindings.is_empty()
            && self.work_modes.is_empty()
            && self.zones.is_empty()
            && self.scripts.is_empty()
            && self.toolbar_items.is_empty()
            && self.statusbar_items.is_empty()
            && self.inline_extensions.is_empty()
            && self.configuration.is_none()
    }
}

#[derive(Debug, Default)]
pub(crate) struct UiContributionRegistrar {
    registrations: HashMap<String, UiContributionRegistration>,
}

impl UiContributionRegistrar {
    pub(crate) fn commit_registration(
        &mut self,
        registration: UiContributionRegistration,
    ) -> Result<()> {
        let extension_id = registration.extension_id.clone();
        if self.registrations.contains_key(&extension_id) {
            return Err(anyhow!(
                "Extension '{}' UI contributions are already registered",
                extension_id
            ));
        }

        self.registrations.insert(extension_id, registration);
        Ok(())
    }

    pub(crate) fn unregister_extension(
        &mut self,
        extension_id: &str,
    ) -> Option<UiContributionRegistration> {
        self.registrations.remove(extension_id)
    }

    pub(crate) fn contains_extension(&self, extension_id: &str) -> bool {
        self.registrations.contains_key(extension_id)
    }

    #[cfg(test)]
    pub(crate) fn get(&self, extension_id: &str) -> Option<UiContributionRegistration> {
        self.registrations.get(extension_id).cloned()
    }

    #[cfg(test)]
    pub(crate) fn list(&self) -> Vec<UiContributionRegistration> {
        self.registrations.values().cloned().collect()
    }
}

pub(crate) fn normalize_registration(
    extension_id: &str,
    contributes: &ExtensionContributes,
) -> UiContributionRegistration {
    UiContributionRegistration {
        extension_id: extension_id.to_string(),
        views: contributes.views.clone().unwrap_or_default(),
        commands: contributes.commands.clone().unwrap_or_default(),
        menus: contributes.menus.clone().unwrap_or_default(),
        keybindings: contributes.keybindings.clone().unwrap_or_default(),
        work_modes: contributes.work_modes.clone().unwrap_or_default(),
        zones: contributes.zones.clone().unwrap_or_default(),
        scripts: contributes.scripts.clone().unwrap_or_default(),
        toolbar_items: contributes.toolbar_items.clone().unwrap_or_default(),
        statusbar_items: contributes.statusbar_items.clone().unwrap_or_default(),
        inline_extensions: contributes.inline_extensions.clone().unwrap_or_default(),
        configuration: contributes.configuration.clone(),
    }
}

pub(crate) fn validate_registration(registration: &UiContributionRegistration) -> Result<()> {
    if registration.extension_id.trim().is_empty() {
        return Err(anyhow!("Extension ID must not be empty"));
    }

    let zones = &registration.zones;
    validate_unique_ids(
        "zone",
        &zones
            .iter()
            .map(|zone| zone.id.as_str())
            .collect::<Vec<_>>(),
    )?;
    let declared_zone_ids: HashSet<&str> = zones.iter().map(|zone| zone.id.as_str()).collect();
    for zone in zones {
        if zone.id.trim().is_empty() {
            return Err(anyhow!("Zone ID is empty"));
        }
        validate_extension_zone_anchor_parent(
            &zone.anchor.parent,
            &registration.extension_id,
            &declared_zone_ids,
        )
        .map_err(|error| anyhow!("Zone '{}' has an invalid anchor parent: {}", zone.id, error))?;
    }

    let views = &registration.views;
    validate_unique_ids(
        "view",
        &views
            .iter()
            .map(|view| view.id.as_str())
            .collect::<Vec<_>>(),
    )?;
    for view in views {
        if view.id.trim().is_empty() {
            return Err(anyhow!("View ID is empty"));
        }
        if let Err(error) = validate_extension_view_with_declared_zones(
            view,
            &registration.extension_id,
            &declared_zone_ids,
        ) {
            return Err(anyhow!(
                "View '{}' uses an unsupported HostView contract: {}",
                view.id,
                error
            ));
        }
    }

    let commands = &registration.commands;
    validate_unique_ids(
        "command",
        &commands
            .iter()
            .map(|command| command.id.as_str())
            .collect::<Vec<_>>(),
    )?;
    let view_ids: HashSet<&str> = views.iter().map(|view| view.id.as_str()).collect();
    for command in commands {
        if command.id.trim().is_empty() {
            return Err(anyhow!("Command ID is empty"));
        }
        match &command.action {
            BuiltinAction::OpenView { view_id }
            | BuiltinAction::ToggleView { view_id }
            | BuiltinAction::OpenDialog { view_id, .. } => {
                if !view_ids.contains(view_id.as_str()) {
                    return Err(anyhow!(
                        "Command '{}' references non-existent view '{}'",
                        command.id,
                        view_id
                    ));
                }
            }
            BuiltinAction::RunScript { script_id, .. } => {
                if script_id.trim().is_empty() {
                    return Err(anyhow!("Command '{}' references an empty script", command.id));
                }
            }
            BuiltinAction::SendMessage { target, .. } => {
                if target.trim().is_empty() {
                    return Err(anyhow!("Command '{}' references an empty message target", command.id));
                }
            }
        }
    }

    let command_ids: HashSet<&str> = commands.iter().map(|command| command.id.as_str()).collect();
    let menus = &registration.menus;
    validate_unique_ids(
        "menu",
        &menus
            .iter()
            .map(|menu| menu.id.as_str())
            .collect::<Vec<_>>(),
    )?;
    for menu in menus {
        if menu.id.trim().is_empty() {
            return Err(anyhow!("Menu ID is empty"));
        }
        if !is_valid_menu_target(&menu.target) {
            return Err(anyhow!(
                "Menu '{}' uses invalid target '{}'",
                menu.id,
                menu.target
            ));
        }
        if !command_ids.contains(menu.command.as_str()) {
            return Err(anyhow!(
                "Menu '{}' references non-existent command '{}'",
                menu.id,
                menu.command
            ));
        }
    }

    let keybindings = &registration.keybindings;
    let mut seen_keybindings = HashSet::new();
    for keybinding in keybindings {
        if keybinding.key.trim().is_empty() {
            return Err(anyhow!("Keybinding key must not be empty"));
        }
        if !command_ids.contains(keybinding.command.as_str()) {
            return Err(anyhow!(
                "Keybinding key '{}' references non-existent command '{}'",
                keybinding.key,
                keybinding.command
            ));
        }
        let identity = (
            keybinding.command.as_str(),
            keybinding.key.as_str(),
            keybinding.when.as_deref(),
        );
        if !seen_keybindings.insert(identity) {
            return Err(anyhow!(
                "Duplicate keybinding '{}' for command '{}'",
                keybinding.key,
                keybinding.command
            ));
        }
    }

    if let Some(configuration) = &registration.configuration {
        validate_configuration_schema(configuration)
            .map_err(|error| anyhow!("Invalid configuration schema: {error}"))?;
    }

    Ok(())
}

fn is_valid_menu_target(target: &str) -> bool {
    let target = target.trim();
    if is_builtin_menu_target(target) {
        return true;
    }
    target.split_once(':').is_some_and(|(owner, local)| {
        !owner.is_empty()
            && !local.is_empty()
            && !owner.contains(char::is_whitespace)
            && !local.contains(char::is_whitespace)
    })
}

fn is_builtin_menu_target(target: &str) -> bool {
    matches!(
        target,
        menu_targets::TOOLS
            | menu_targets::INPUT_PLUS
            | menu_targets::CHAT_TITLE
            | menu_targets::RIGHT_PANEL
            | menu_targets::GATEWAY
            | menu_targets::WORKTREE_CONTEXT
            | menu_targets::SESSION_CONTEXT
    )
}

const VALID_JSON_SCHEMA_TYPES: &[&str] = &[
    "string",
    "object",
    "array",
    "number",
    "boolean",
    "integer",
    "null",
];

/// 轻量校验 `contributes.configuration` 的 JSON Schema 自身合法性。
/// 只拒绝明显非法结构，不重造完整 JSON Schema 校验器。
fn validate_configuration_schema(schema: &serde_json::Value) -> Result<()> {
    let Some(object) = schema.as_object() else {
        return Err(anyhow!("configuration must be a JSON object schema"));
    };
    let Some(type_name) = object.get("type").and_then(serde_json::Value::as_str) else {
        return Err(anyhow!("configuration schema must declare a 'type'"));
    };
    if !VALID_JSON_SCHEMA_TYPES.contains(&type_name) {
        return Err(anyhow!(
            "configuration schema type '{type_name}' is not a valid JSON Schema type"
        ));
    }
    if type_name == "object" {
        if let Some(properties) = object.get("properties") {
            if !properties.is_object() {
                return Err(anyhow!("configuration schema 'properties' must be an object"));
            }
        }
    }
    if type_name == "array" {
        if let Some(items) = object.get("items") {
            if !items.is_object() {
                return Err(anyhow!("configuration schema 'items' must be an object"));
            }
        }
    }
    Ok(())
}

pub(crate) fn prepare_registration(
    normalized: &UiContributionRegistration,
) -> Result<UiContributionRegistration> {
    validate_registration(normalized)?;
    Ok(normalized.clone())
}

fn validate_unique_ids(kind: &str, ids: &[&str]) -> Result<()> {
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(*id) {
            return Err(anyhow!("Duplicate {} ID '{}'", kind, id));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::host_view::HOST_PANEL_RENDERER;
    use crate::extension::models::{KeybindingScope, ZoneAnchor};
    use serde_json::json;

    fn view(id: &str) -> ViewRegistration {
        ViewRegistration {
            id: id.to_string(),
            name: id.to_string(),
            icon: None,
            entry: None,
            zone: Some("rightWorkspace".to_string()),
            placement: None,
            renderer: HOST_PANEL_RENDERER.to_string(),
            config: None,
            activation_events: Vec::new(),
            allow_close: Some(true),
            default_visible: Some(false),
        }
    }

    fn contributes() -> ExtensionContributes {
        ExtensionContributes {
            views: Some(vec![view("view")]),
            commands: Some(vec![CommandRegistration {
                id: "open".to_string(),
                label: "Open".to_string(),
                description: None,
                icon: None,
                category: None,
                when: None,
                action: BuiltinAction::OpenView {
                    view_id: "view".to_string(),
                },
            }]),
            menus: Some(vec![MenuRegistration {
                id: "menu".to_string(),
                label: "Open".to_string(),
                target: "Tools".to_string(),
                command: "open".to_string(),
                group: None,
                when: None,
                icon: None,
                shortcut: None,
                risk: None,
            }]),
            keybindings: Some(vec![KeybindingRegistration {
                command: "open".to_string(),
                key: "Ctrl+K".to_string(),
                when: None,
                scope: KeybindingScope::App,
            }]),
            ..Default::default()
        }
    }

    #[test]
    fn registration_is_atomic_and_unregisters_as_one_unit() {
        let mut registrar = UiContributionRegistrar::default();
        let registration =
            prepare_registration(&normalize_registration("example", &contributes())).unwrap();
        registrar.commit_registration(registration.clone()).unwrap();

        assert_eq!(registration.views.len(), 1);
        assert_eq!(registration.commands.len(), 1);
        assert_eq!(registration.menus.len(), 1);
        assert_eq!(registration.keybindings.len(), 1);
        assert_eq!(registrar.list().len(), 1);

        let removed = registrar.unregister_extension("example").unwrap();
        assert_eq!(removed, registration);
        assert!(registrar.list().is_empty());
    }

    #[test]
    fn duplicate_or_unresolved_ui_contributions_leave_no_registration() {
        let registrar = UiContributionRegistrar::default();
        let mut invalid = contributes();
        invalid.commands.as_mut().unwrap()[0].action = BuiltinAction::ToggleView {
            view_id: "missing".to_string(),
        };

        let error =
            validate_registration(&normalize_registration("example", &invalid)).unwrap_err();
        assert!(error.to_string().contains("non-existent view"));
        assert!(registrar.list().is_empty());

        let mut duplicate = contributes();
        let first = duplicate.commands.as_ref().unwrap()[0].clone();
        duplicate.commands.as_mut().unwrap().push(first);
        let error =
            validate_registration(&normalize_registration("example", &duplicate)).unwrap_err();
        assert!(error.to_string().contains("Duplicate command ID"));
        assert!(registrar.list().is_empty());
    }

    #[test]
    fn duplicate_registration_is_rejected_without_replacing_existing_state() {
        let mut registrar = UiContributionRegistrar::default();
        let expected =
            prepare_registration(&normalize_registration("example", &contributes())).unwrap();
        registrar.commit_registration(expected.clone()).unwrap();

        let error = registrar
            .commit_registration(
                prepare_registration(&normalize_registration("example", &contributes())).unwrap(),
            )
            .unwrap_err();
        assert!(error.to_string().contains("already registered"));
        assert_eq!(registrar.get("example").unwrap(), expected);
    }

    #[test]
    fn zones_must_be_unique_and_anchor_to_known_zones() {
        let mut registration = normalize_registration("example", &contributes());
        registration.zones = vec![
            ZoneRegistration {
                id: "customPanel".into(),
                name: "Custom Panel".into(),
                anchor: ZoneAnchor {
                    parent: "rightWorkspace".into(),
                    position: None,
                    size: None,
                },
            },
            ZoneRegistration {
                id: "chatDock".into(),
                name: "Chat Dock".into(),
                anchor: ZoneAnchor {
                    parent: "example:customPanel".into(),
                    position: None,
                    size: None,
                },
            },
        ];
        assert!(validate_registration(&registration).is_ok());

        let mut duplicate = registration.clone();
        duplicate.zones.push(duplicate.zones[0].clone());
        let error = validate_registration(&duplicate).unwrap_err();
        assert!(error.to_string().contains("Duplicate zone ID"));

        let mut undeclared = registration.clone();
        undeclared.zones[1].anchor.parent = "example:missing".into();
        let error = validate_registration(&undeclared).unwrap_err();
        assert!(error.to_string().contains("undeclared zone"));

        let mut cross = registration.clone();
        cross.zones[1].anchor.parent = "other.ext:customPanel".into();
        let error = validate_registration(&cross).unwrap_err();
        assert!(error.to_string().contains("zone namespace"));

        let mut plain = registration.clone();
        plain.zones[1].anchor.parent = "floatingDock".into();
        let error = validate_registration(&plain).unwrap_err();
        assert!(error.to_string().contains("must be a builtin zone"));
    }

    #[test]
    fn view_zone_must_be_declared_by_the_extension() {
        let mut registration = normalize_registration("example", &contributes());
        registration.zones = vec![ZoneRegistration {
            id: "customPanel".into(),
            name: "Custom Panel".into(),
            anchor: ZoneAnchor {
                parent: "rightWorkspace".into(),
                position: None,
                size: None,
            },
        }];
        registration.views[0].zone = Some("example:customPanel".into());
        assert!(validate_registration(&registration).is_ok());

        let mut undeclared = registration.clone();
        undeclared.views[0].zone = Some("example:missing".into());
        let error = validate_registration(&undeclared).unwrap_err();
        assert!(error.to_string().contains("undeclared zone"));

        let mut cross = registration.clone();
        cross.views[0].zone = Some("other.ext:customPanel".into());
        let error = validate_registration(&cross).unwrap_err();
        assert!(error.to_string().contains("zone namespace"));
    }

    #[test]
    fn menu_target_accepts_builtin_and_namespaced_and_rejects_others() {
        let mut registration = normalize_registration("example", &contributes());
        let set_target = |target: &str, registration: &mut UiContributionRegistration| {
            registration.menus[0].target = target.to_string();
        };

        set_target("Tools", &mut registration);
        assert!(validate_registration(&registration).is_ok());
        set_target("example:custom", &mut registration);
        assert!(validate_registration(&registration).is_ok());

        for invalid in ["", "Foo Bar", "foo", "example:", ":custom", "exa mple:custom"] {
            let mut candidate = registration.clone();
            set_target(invalid, &mut candidate);
            assert!(
                validate_registration(&candidate).is_err(),
                "target: {invalid:?}"
            );
        }
    }

    #[test]
    fn configuration_schema_is_loosely_validated() {
        let mut registration = normalize_registration("example", &contributes());
        registration.configuration = Some(json!({
            "type": "object",
            "properties": {
                "theme": { "type": "string" },
                "count": { "type": "number" }
            }
        }));
        assert!(validate_registration(&registration).is_ok());

        let mut missing_type = registration.clone();
        missing_type.configuration = Some(json!({ "properties": {} }));
        let error = validate_registration(&missing_type).unwrap_err();
        assert!(error.to_string().contains("must declare a 'type'"));

        let mut bad_type = registration.clone();
        bad_type.configuration = Some(json!({ "type": "wat" }));
        let error = validate_registration(&bad_type).unwrap_err();
        assert!(error.to_string().contains("not a valid JSON Schema type"));

        let mut bad_properties = registration.clone();
        bad_properties.configuration = Some(json!({ "type": "object", "properties": [] }));
        let error = validate_registration(&bad_properties).unwrap_err();
        assert!(error.to_string().contains("'properties' must be an object"));

        let mut bad_items = registration.clone();
        bad_items.configuration = Some(json!({ "type": "array", "items": "string" }));
        let error = validate_registration(&bad_items).unwrap_err();
        assert!(error.to_string().contains("'items' must be an object"));
    }
}
