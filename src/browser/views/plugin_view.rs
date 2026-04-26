use std::path::Path;

use crate::config;
use crate::error::Result;
use crate::plugin;

use super::super::selector::{ActionResult, InteractiveSelector, SelectItem};
use super::super::{write_cd_target, write_post_commands};
use super::{match_view_switch, view_header, view_switch_keys, Outcome};

/// Run a plugin-provided view. Calls the plugin's OnView hook to get items,
/// renders them in fzf, then calls OnViewSelect with the user's choice.
pub(super) fn run(
    selector: &dyn InteractiveSelector,
    config: &config::model::EzConfig,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
    plugin_name: &str,
    view_name: &str,
) -> Result<Outcome> {
    let plugin_views = plugin::collect_plugin_views("plugin", config).unwrap_or_default();

    // Call plugin's OnView hook to get items
    let response = plugin::run_view_hook(plugin_name, view_name, config)?;

    let view_items = match response.view_items {
        Some(items) if !items.is_empty() => items,
        _ => {
            eprintln!("No items from plugin view '{view_name}'.");
            return Ok(Outcome::Done);
        }
    };

    let items: Vec<SelectItem> = view_items
        .iter()
        .map(|vi| SelectItem {
            display: vi.display.clone(),
            value: vi.value.clone(),
        })
        .collect();

    let prompt = response
        .view_prompt
        .as_deref()
        .unwrap_or(view_name);

    let header = view_header(view_name, &config.keybinds, &plugin_views);

    let action = selector.select_with_actions(
        &items,
        prompt,
        response.view_preview_cmd.as_deref(),
        &view_switch_keys(&config.keybinds, &plugin_views),
        Some(&header),
    )?;

    match action {
        ActionResult::Cancel => Ok(Outcome::Done),
        ActionResult::Action(key, _) => {
            match match_view_switch(&config.keybinds, &plugin_views, &key) {
                Some(next) => Ok(Outcome::Switch(next)),
                None => Ok(Outcome::Done),
            }
        }
        ActionResult::Select(idx) => {
            let selected = &view_items[idx];

            // Call plugin's OnViewSelect hook
            let select_response = plugin::run_view_select_hook(
                plugin_name,
                view_name,
                &selected.value,
                &selected.display,
                config,
            )?;

            // Write cd target if plugin specified one
            if let Some(ref cd) = select_response.cd_target {
                write_cd_target(cd_file, cd)?;
            }

            // Write post-exit shell commands for the shell wrapper
            if !select_response.post_shell_commands.is_empty() {
                write_post_commands(post_cmd_file, &select_response.post_shell_commands)?;
            }

            // Execute any immediate shell commands
            if !select_response.shell_commands.is_empty() {
                plugin::runner::run_shell_commands(&select_response.shell_commands)?;
            }

            Ok(Outcome::Done)
        }
    }
}
