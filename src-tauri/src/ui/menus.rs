use super::dto::{
    UiCommandRegistration, UiExtensionKeybinding, UiHostViewTarget, UiMenuBuiltinAction,
    UiMenuRegistration, UiSlashCommandRegistration,
};
use super::host_view::ui_extension_view_descriptor;
use crate::extension::models::{
    menu_targets, BuiltinAction, CommandRegistration, ExtensionState, ExtensionStatus,
    KeybindingRegistration, KeybindingScope, MenuRegistration, MenuRisk,
};
use crate::extension::skills::Skills;
use crate::extension::ExtensionStore;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::State;

fn menu(
    id: &str,
    label: &str,
    target: &str,
    command: &str,
    risk: Option<MenuRisk>,
) -> MenuRegistration {
    menu_with_meta(id, label, target, command, risk, None, None)
}

fn menu_with_meta(
    id: &str,
    label: &str,
    target: &str,
    command: &str,
    risk: Option<MenuRisk>,
    group: Option<&str>,
    shortcut: Option<&str>,
) -> MenuRegistration {
    MenuRegistration {
        id: id.to_string(),
        label: label.to_string(),
        target: target.to_string(),
        command: command.to_string(),
        group: group.map(str::to_string),
        when: None,
        icon: None,
        shortcut: shortcut.map(str::to_string),
        risk,
    }
}

fn menu_with_icon(
    id: &str,
    label: &str,
    target: &str,
    command: &str,
    icon: &str,
) -> MenuRegistration {
    MenuRegistration {
        id: id.to_string(),
        label: label.to_string(),
        target: target.to_string(),
        command: command.to_string(),
        group: None,
        when: None,
        icon: Some(icon.to_string()),
        shortcut: None,
        risk: None,
    }
}

fn extension_host_view_targets(extension: &ExtensionState) -> HashMap<String, UiHostViewTarget> {
    extension
        .manifest
        .contributes
        .views
        .as_ref()
        .map(|views| {
            views
                .iter()
                .filter_map(|view| {
                    let descriptor =
                        ui_extension_view_descriptor(view, &extension.install_path).ok()?;
                    let view_id = descriptor.view_id.clone();
                    Some((view_id, UiHostViewTarget { view: descriptor }))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn ui_menu_builtin_action(
    action: BuiltinAction,
    view_targets: &HashMap<String, UiHostViewTarget>,
) -> Option<UiMenuBuiltinAction> {
    match action {
        BuiltinAction::OpenView { view_id } => view_targets
            .get(&view_id)
            .cloned()
            .map(|view| UiMenuBuiltinAction::OpenView { view }),
        BuiltinAction::ToggleView { view_id } => view_targets
            .get(&view_id)
            .cloned()
            .map(|view| UiMenuBuiltinAction::ToggleView { view }),
        BuiltinAction::OpenDialog { view_id, size, position, modal } => view_targets
            .get(&view_id)
            .cloned()
            .map(|view| UiMenuBuiltinAction::OpenDialog { view, size, position, modal }),
        BuiltinAction::RunScript { script_id, args } => {
            Some(UiMenuBuiltinAction::RunScript { script_id, args })
        }
        BuiltinAction::SendMessage { target, payload } => {
            Some(UiMenuBuiltinAction::SendMessage { target, payload })
        }
    }
}

fn ui_extension_command(
    command: CommandRegistration,
    extension_id: &str,
    extension_name: &str,
    view_targets: &HashMap<String, UiHostViewTarget>,
) -> Option<UiCommandRegistration> {
    let action = ui_menu_builtin_action(command.action, view_targets)?;

    Some(UiCommandRegistration {
        id: format!("{}/{}", extension_id, command.id),
        label: command.label,
        description: command.description,
        category: command
            .category
            .unwrap_or_else(|| extension_name.to_string()),
        icon: command.icon,
        extension_id: extension_id.to_string(),
        extension_name: extension_name.to_string(),
        action,
    })
}

fn ui_extension_keybinding(
    keybinding: KeybindingRegistration,
    command: CommandRegistration,
    extension_id: &str,
    extension_name: &str,
    view_targets: &HashMap<String, UiHostViewTarget>,
) -> Option<UiExtensionKeybinding> {
    if keybinding.key.trim().is_empty() || keybinding.when.is_some() || command.when.is_some() {
        return None;
    }

    let ui_command = ui_extension_command(command, extension_id, extension_name, view_targets)?;
    let command_id = ui_command.id;
    let description = ui_command
        .description
        .unwrap_or_else(|| ui_command.label.clone());

    Some(UiExtensionKeybinding {
        id: format!("{command_id}:{}", keybinding.key),
        keybinding: keybinding.key,
        scope: match keybinding.scope {
            KeybindingScope::App => "app".to_string(),
        },
        command: command_id,
        description,
        category: ui_command.category,
        extension_id: ui_command.extension_id,
        extension_name: ui_command.extension_name,
        action: ui_command.action,
    })
}

fn sorted_enabled_extensions(extension_store: &ExtensionStore) -> Vec<ExtensionState> {
    let mut extensions = extension_store.list_by_status(&ExtensionStatus::Enabled);
    extensions.sort_by(|a, b| {
        a.manifest
            .name
            .to_lowercase()
            .cmp(&b.manifest.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    extensions
}

fn ui_extension_keybindings_for_extension(
    extension: &ExtensionState,
) -> Vec<UiExtensionKeybinding> {
    let Some(keybindings) = extension.manifest.contributes.keybindings.as_ref() else {
        return Vec::new();
    };

    let commands = extension
        .manifest
        .contributes
        .commands
        .as_ref()
        .map(|commands| {
            commands
                .iter()
                .map(|command| (command.id.clone(), command.clone()))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let view_targets = extension_host_view_targets(extension);

    keybindings
        .iter()
        .filter_map(|keybinding| {
            let command = commands.get(&keybinding.command)?.clone();
            ui_extension_keybinding(
                keybinding.clone(),
                command,
                &extension.id,
                &extension.manifest.name,
                &view_targets,
            )
        })
        .collect()
}

fn ui_menu(
    menu: MenuRegistration,
    extension_id: Option<String>,
    action: Option<UiMenuBuiltinAction>,
) -> UiMenuRegistration {
    let id = extension_id
        .as_ref()
        .map(|source| format!("{}/{}", source, menu.id))
        .unwrap_or(menu.id);

    UiMenuRegistration {
        id,
        label: menu.label,
        target: menu.target,
        command: menu.command,
        group: menu.group,
        when: menu.when,
        icon: menu.icon,
        shortcut: menu.shortcut,
        risk: menu.risk,
        extension_id,
        action,
    }
}

fn builtin_menus() -> Vec<MenuRegistration> {
    vec![
        menu_with_meta(
            "tools.command-palette",
            "Command palette",
            menu_targets::TOOLS,
            "tools.commandPalette",
            None,
            Some("global"),
            Some(">"),
        ),
        menu_with_meta(
            "tools.settings",
            "Settings",
            menu_targets::TOOLS,
            "tools.settings",
            None,
            Some("global"),
            None,
        ),
        menu_with_meta(
            "tools.gateway",
            "Gateway",
            menu_targets::TOOLS,
            "tools.gateway",
            None,
            Some("settings"),
            None,
        ),
        menu_with_meta(
            "tools.coding-editor",
            "Coding",
            menu_targets::TOOLS,
            "tools.codingEditor",
            None,
            Some("settings"),
            None,
        ),
        menu_with_meta(
            "tools.extensions",
            "Extensions",
            menu_targets::TOOLS,
            "tools.extensions",
            None,
            Some("settings"),
            None,
        ),
        menu_with_meta(
            "chat-title.open-in",
            "Open in",
            menu_targets::CHAT_TITLE,
            "session.openIn",
            None,
            Some("open"),
            Some("›"),
        ),
        menu_with_meta(
            "chat-title.rename",
            "Rename",
            menu_targets::CHAT_TITLE,
            "session.rename",
            None,
            Some("manage"),
            None,
        ),
        menu_with_meta(
            "chat-title.transcript-view",
            "Transcript view",
            menu_targets::CHAT_TITLE,
            "session.transcriptView",
            None,
            Some("manage"),
            Some("›"),
        ),
        menu_with_meta(
            "chat-title.fork",
            "Fork",
            menu_targets::CHAT_TITLE,
            "session.fork",
            Some(MenuRisk::Medium),
            Some("manage"),
            None,
        ),
        menu_with_meta(
            "chat-title.archive",
            "Archive",
            menu_targets::CHAT_TITLE,
            "session.archive",
            Some(MenuRisk::Medium),
            Some("lifecycle"),
            None,
        ),
        menu_with_meta(
            "chat-title.delete",
            "Delete",
            menu_targets::CHAT_TITLE,
            "session.delete",
            Some(MenuRisk::High),
            Some("lifecycle"),
            None,
        ),
        menu_with_icon(
            "right-panel.diff",
            "Diff",
            menu_targets::RIGHT_PANEL,
            "rightWorkspace.open.diff",
            "diff",
        ),
        menu_with_icon(
            "right-panel.background-tasks",
            "Background tasks",
            menu_targets::RIGHT_PANEL,
            "rightWorkspace.open.backgroundTasks",
            "background-tasks",
        ),
        menu_with_icon(
            "right-panel.plan",
            "Plan",
            menu_targets::RIGHT_PANEL,
            "rightWorkspace.open.plan",
            "plan",
        ),
        menu_with_icon(
            "right-panel.design",
            "Design",
            menu_targets::RIGHT_PANEL,
            "rightWorkspace.open.design",
            "design",
        ),
        menu(
            "input-plus.files",
            "Add files or photos",
            menu_targets::INPUT_PLUS,
            "composer.addFiles",
            None,
        ),
        menu(
            "input-plus.folder",
            "Add folder",
            menu_targets::INPUT_PLUS,
            "composer.addFolder",
            None,
        ),
        menu(
            "input-plus.slash",
            "Slash commands",
            menu_targets::INPUT_PLUS,
            "composer.insertSlashCommand",
            None,
        ),
        menu(
            "input-plus.connectors",
            "Add connectors",
            menu_targets::INPUT_PLUS,
            "composer.addConnectors",
            None,
        ),
        menu(
            "input-plus.extensions",
            "Add extensions...",
            menu_targets::INPUT_PLUS,
            "composer.addExtensions",
            None,
        ),
        menu_with_meta(
            "input-plus.plan-mode",
            "Plan mode",
            menu_targets::INPUT_PLUS,
            "composer.togglePlanMode",
            None,
            Some("mode"),
            None,
        ),
        menu_with_meta(
            "input-plus.multi-agent",
            "Multi-agent",
            menu_targets::INPUT_PLUS,
            "composer.toggleMultiAgent",
            None,
            Some("mode"),
            None,
        ),
        menu_with_meta(
            "input-plus.pursue-goal",
            "Pursue goal",
            menu_targets::INPUT_PLUS,
            "composer.toggleGoalTracking",
            None,
            Some("mode"),
            None,
        ),
        menu_with_meta(
            "gateway.settings",
            "Settings",
            menu_targets::GATEWAY,
            "gateway.settings",
            None,
            Some("settings"),
            None,
        ),
        menu_with_meta(
            "gateway.language",
            "Language",
            menu_targets::GATEWAY,
            "gateway.language",
            None,
            Some("settings"),
            None,
        ),
        menu_with_meta(
            "worktree.rename",
            "Rename worktree",
            menu_targets::WORKTREE_CONTEXT,
            "worktree.rename",
            None,
            Some("manage"),
            None,
        ),
        menu_with_meta(
            "worktree.delete",
            "Delete worktree",
            menu_targets::WORKTREE_CONTEXT,
            "worktree.delete",
            Some(MenuRisk::High),
            Some("danger"),
            None,
        ),
        menu_with_meta(
            "session.pin",
            "Pin",
            menu_targets::SESSION_CONTEXT,
            "session.pin",
            None,
            Some("manage"),
            Some("P"),
        ),
        menu_with_meta(
            "session.mark-unread",
            "Mark as unread",
            menu_targets::SESSION_CONTEXT,
            "session.markUnread",
            None,
            Some("manage"),
            Some("U"),
        ),
        menu_with_meta(
            "session.rename",
            "Rename",
            menu_targets::SESSION_CONTEXT,
            "session.rename",
            None,
            Some("manage"),
            Some("R"),
        ),
        menu_with_meta(
            "session.fork",
            "Fork",
            menu_targets::SESSION_CONTEXT,
            "session.fork",
            Some(MenuRisk::Medium),
            Some("manage"),
            Some("F"),
        ),
        menu_with_meta(
            "session.move-to-worktree",
            "Move to worktree",
            menu_targets::SESSION_CONTEXT,
            "session.moveToWorktree",
            Some(MenuRisk::Medium),
            Some("organize"),
            Some("›"),
        ),
        menu_with_meta(
            "session.archive",
            "Archive",
            menu_targets::SESSION_CONTEXT,
            "session.archive",
            Some(MenuRisk::Medium),
            Some("lifecycle"),
            Some("A"),
        ),
        menu_with_meta(
            "session.delete",
            "Delete",
            menu_targets::SESSION_CONTEXT,
            "session.delete",
            Some(MenuRisk::High),
            Some("lifecycle"),
            Some("D"),
        ),
    ]
}

#[tauri::command]
pub fn ui_list_menus(extension_store: State<'_, Arc<ExtensionStore>>) -> Vec<UiMenuRegistration> {
    let mut menus: Vec<UiMenuRegistration> = builtin_menus()
        .into_iter()
        .map(|menu| ui_menu(menu, None, None))
        .collect();

    let extensions = sorted_enabled_extensions(extension_store.inner().as_ref());

    for extension in extensions {
        let view_targets = extension_host_view_targets(&extension);
        let action_by_command: HashMap<String, crate::extension::models::BuiltinAction> = extension
            .manifest
            .contributes
            .commands
            .as_ref()
            .map(|commands| {
                commands
                    .iter()
                    .map(|command| (command.id.clone(), command.action.clone()))
                    .collect()
            })
            .unwrap_or_default();

        if let Some(extension_menus) = extension.manifest.contributes.menus {
            menus.extend(extension_menus.into_iter().filter_map(|menu| {
                if menu.target.trim().is_empty() {
                    return None;
                }

                let action = action_by_command.get(&menu.command).cloned()?;
                let action = ui_menu_builtin_action(action, &view_targets)?;

                let extension_id = extension.id.clone();
                Some(ui_menu(menu, Some(extension_id), Some(action)))
            }));
        }
    }

    menus
}

#[tauri::command]
pub fn ui_list_extension_commands(
    extension_store: State<'_, Arc<ExtensionStore>>,
) -> Vec<UiCommandRegistration> {
    let mut commands = Vec::new();
    let extensions = sorted_enabled_extensions(extension_store.inner().as_ref());

    for extension in extensions {
        let extension_id = extension.id.clone();
        let extension_name = extension.manifest.name.clone();
        let view_targets = extension_host_view_targets(&extension);
        if let Some(extension_commands) = extension.manifest.contributes.commands {
            commands.extend(extension_commands.into_iter().filter_map(|command| {
                ui_extension_command(command, &extension_id, &extension_name, &view_targets)
            }));
        }
    }

    commands
}

#[tauri::command]
pub fn ui_list_extension_keybindings(
    extension_store: State<'_, Arc<ExtensionStore>>,
) -> Vec<UiExtensionKeybinding> {
    sorted_enabled_extensions(extension_store.inner().as_ref())
        .iter()
        .flat_map(ui_extension_keybindings_for_extension)
        .collect()
}

#[tauri::command]
pub fn ui_list_slash_commands(
    skills: State<'_, Arc<Mutex<Skills>>>,
) -> Result<Vec<UiSlashCommandRegistration>, String> {
    let skills = skills.lock().map_err(|error| error.to_string())?;
    Ok(skills
        .get_trigger_candidates()
        .into_iter()
        .map(|candidate| UiSlashCommandRegistration {
            trigger: candidate.trigger,
            name: candidate.name,
            description: candidate.description,
            trigger_type: candidate.trigger_type,
            source: candidate.source,
            source_label: candidate.source_label,
            extension_id: candidate.extension_id,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::models::ViewRegistration;

    fn test_view(placement: Option<&str>, renderer: &str) -> ViewRegistration {
        ViewRegistration {
            id: "test.view".to_string(),
            name: "Test View".to_string(),
            icon: None,
            entry: None,
            zone: None,
            placement: placement.map(str::to_string),
            renderer: renderer.to_string(),
            config: None,
            activation_events: Vec::new(),
            allow_close: Some(true),
            default_visible: Some(false),
        }
    }
    fn test_command(action: BuiltinAction) -> CommandRegistration {
        CommandRegistration {
            id: "panel.open".to_string(),
            label: "Open panel".to_string(),
            description: Some("Open the panel".to_string()),
            icon: None,
            category: Some("Testing".to_string()),
            when: None,
            action,
        }
    }

    fn test_keybinding_with_key(
        command: &str,
        key: &str,
        when: Option<&str>,
    ) -> KeybindingRegistration {
        KeybindingRegistration {
            command: command.to_string(),
            key: key.to_string(),
            when: when.map(str::to_string),
            scope: KeybindingScope::App,
        }
    }

    fn test_keybinding(command: &str, when: Option<&str>) -> KeybindingRegistration {
        test_keybinding_with_key(command, "Ctrl+Shift+P", when)
    }

    fn test_extension(
        commands: Vec<CommandRegistration>,
        keybindings: Vec<KeybindingRegistration>,
    ) -> ExtensionState {
        ExtensionState {
            id: "sample.extension".to_string(),
            status: ExtensionStatus::Enabled,
            manifest: crate::extension::models::ExtensionManifest {
                id: "sample.extension".to_string(),
                name: "Sample Extension".to_string(),
                version: "1.0.0".to_string(),
                description: String::new(),
                author: String::new(),
                permissions: Default::default(),
                contributes: crate::extension::models::ExtensionContributes {
                    commands: Some(commands),
                    keybindings: Some(keybindings),
                    views: Some(vec![test_view(Some("rightWorkspace"), "host:panel")]),
                    ..Default::default()
                },
            },
            install_path: Default::default(),
            installed_at: chrono::Utc::now(),
            enabled_at: None,
            error: None,
        }
    }

    #[test]
    fn extension_keybinding_projection_namespaces_command_and_reuses_ui_action() {
        let extension = test_extension(
            vec![test_command(BuiltinAction::OpenView {
                view_id: "test.view".to_string(),
            })],
            vec![test_keybinding("panel.open", None)],
        );

        let projections = ui_extension_keybindings_for_extension(&extension);
        assert_eq!(projections.len(), 1);
        assert_eq!(
            projections[0].id,
            "sample.extension/panel.open:Ctrl+Shift+P"
        );
        assert_eq!(projections[0].command, "sample.extension/panel.open");
        assert_eq!(projections[0].scope, "app");
        assert_eq!(projections[0].description, "Open the panel");
        assert!(matches!(
            projections[0].action,
            UiMenuBuiltinAction::OpenView { .. }
        ));
    }

    #[test]
    fn extension_keybinding_projection_fails_closed_for_when() {
        let extension = test_extension(
            vec![test_command(BuiltinAction::OpenView {
                view_id: "test.view".to_string(),
            })],
            vec![test_keybinding("panel.open", Some("editorFocus"))],
        );
        assert!(ui_extension_keybindings_for_extension(&extension).is_empty());

        let mut command = test_command(BuiltinAction::OpenView {
            view_id: "test.view".to_string(),
        });
        command.when = Some("chatVisible".to_string());
        let extension = test_extension(vec![command], vec![test_keybinding("panel.open", None)]);
        assert!(ui_extension_keybindings_for_extension(&extension).is_empty());
    }

    #[test]
    fn extension_keybinding_projection_reuses_toggle_view_action() {
        let extension = test_extension(
            vec![test_command(BuiltinAction::ToggleView {
                view_id: "test.view".to_string(),
            })],
            vec![test_keybinding("panel.open", None)],
        );

        let projections = ui_extension_keybindings_for_extension(&extension);
        assert_eq!(projections.len(), 1);
        assert_eq!(projections[0].command, "sample.extension/panel.open");
        assert!(matches!(
            projections[0].action,
            UiMenuBuiltinAction::ToggleView { .. }
        ));
    }

    #[test]
    fn extension_keybinding_projection_rejects_empty_key() {
        let extension = test_extension(
            vec![test_command(BuiltinAction::OpenView {
                view_id: "test.view".to_string(),
            })],
            vec![test_keybinding_with_key("panel.open", "  ", None)],
        );

        assert!(ui_extension_keybindings_for_extension(&extension).is_empty());
    }

    #[test]
    fn extension_keybinding_projection_excludes_missing_commands() {
        let extension = test_extension(
            vec![test_command(BuiltinAction::OpenView {
                view_id: "test.view".to_string(),
            })],
            vec![
                test_keybinding("panel.open", None),
                test_keybinding("missing.command", None),
            ],
        );
        let projections = ui_extension_keybindings_for_extension(&extension);
        assert_eq!(projections.len(), 1);
        assert_eq!(projections[0].command, "sample.extension/panel.open");
    }

    #[test]
    fn extension_view_projection_fails_closed_for_invalid_contracts() {
        for (placement, renderer, entry) in [
            (Some("rightWorkspace"), "extension:dynamic", None),
            (Some("floatingDock"), "host:panel", None),
            (
                Some("rightWorkspace"),
                "html:sandbox",
                Some("../index.html"),
            ),
        ] {
            let mut extension = test_extension(
                vec![test_command(BuiltinAction::OpenView {
                    view_id: "test.view".to_string(),
                })],
                vec![test_keybinding("panel.open", None)],
            );
            let mut view = test_view(placement, renderer);
            view.entry = entry.map(str::to_string);
            extension.manifest.contributes.views = Some(vec![view]);

            let view_targets = extension_host_view_targets(&extension);
            assert!(view_targets.is_empty(), "invalid view was projected");
            assert!(ui_extension_command(
                test_command(BuiltinAction::OpenView {
                    view_id: "test.view".to_string(),
                }),
                &extension.id,
                &extension.manifest.name,
                &view_targets,
            )
            .is_none());
            assert!(ui_extension_keybindings_for_extension(&extension).is_empty());
        }
    }
}
