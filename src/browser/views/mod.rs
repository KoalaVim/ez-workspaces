//! Top-level view dispatcher for the interactive browser.
//!
//! Each view renders an fzf selector for a different "type" of top-level item
//! (tree, workspace, repo, owner, label). A set of view-switch keybinds is
//! registered with `--expect`; pressing one exits the current fzf instance and
//! the dispatch loop re-enters the chosen view.
//!
//! Plugin views are registered via manifest `[[views]]` entries and appear
//! alongside the core views in the header and keybind list.

mod label;
mod owner;
mod plugin_view;
mod repo;
mod tree;
mod workspace;

use std::path::Path;

use crate::config;
use crate::error::{EzError, Result};
use crate::plugin;

use super::selector::InteractiveSelector;

/// Top-level view modes selectable via keybinds.
#[derive(Clone, Debug)]
pub enum ViewMode {
    Tree,
    Workspace,
    Repo,
    Owner,
    Label,
    /// A view provided by a plugin.
    Plugin {
        view_name: String,
        plugin_name: String,
    },
}

impl ViewMode {
    pub fn from_flag(v: &str, config: &config::model::EzConfig) -> Result<Self> {
        match v.to_ascii_lowercase().as_str() {
            "tree" | "t" => Ok(ViewMode::Tree),
            "workspace" | "ws" | "w" => Ok(ViewMode::Workspace),
            "repo" | "repos" | "r" => Ok(ViewMode::Repo),
            "owner" | "owners" | "o" => Ok(ViewMode::Owner),
            "label" | "labels" | "l" => Ok(ViewMode::Label),
            other => {
                // Look up plugin views by name
                if let Some(pv) = plugin::find_plugin_view(other, config)? {
                    Ok(ViewMode::Plugin {
                        view_name: pv.view_name,
                        plugin_name: pv.plugin_name,
                    })
                } else {
                    Err(EzError::Config(format!(
                        "unknown view '{other}' — expected one of: tree, workspace, repo, owner, label (or a plugin view)"
                    )))
                }
            }
        }
    }

}

/// Dispatch loop that renders views and handles view-switch keybinds.
pub fn run(
    initial: ViewMode,
    selector: &dyn InteractiveSelector,
    config: &config::model::EzConfig,
    workspace_override: Option<&str>,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
) -> Result<()> {
    let mut mode = initial;
    // --workspace short-circuits the root picker for the Workspace view only.
    let mut workspace_jump = workspace_override.map(|s| s.to_string());

    loop {
        let outcome = match mode {
            ViewMode::Tree => tree::run(selector, config, cd_file, post_cmd_file)?,
            ViewMode::Workspace => {
                let jump = workspace_jump.take();
                workspace::run(selector, config, cd_file, post_cmd_file, jump.as_deref())?
            }
            ViewMode::Repo => repo::run(selector, config, cd_file, post_cmd_file)?,
            ViewMode::Owner => owner::run(selector, config, cd_file, post_cmd_file)?,
            ViewMode::Label => label::run(selector, config, cd_file, post_cmd_file)?,
            ViewMode::Plugin {
                ref view_name,
                ref plugin_name,
            } => plugin_view::run(
                selector,
                config,
                cd_file,
                post_cmd_file,
                plugin_name,
                view_name,
            )?,
        };

        match outcome {
            Outcome::Done => return Ok(()),
            Outcome::Switch(next) => {
                mode = next;
            }
        }
    }
}

/// Result of a single view render.
pub(super) enum Outcome {
    Done,
    Switch(ViewMode),
}

/// Keys registered with `select_with_actions` for view-switching; the caller
/// interprets an `Action` whose key matches any of these as a view switch.
pub(super) fn view_switch_keys<'a>(
    kb: &'a config::model::KeybindsConfig,
    plugin_views: &'a [plugin::PluginViewInfo],
) -> Vec<&'a str> {
    let mut keys = vec![
        kb.view_tree.as_str(),
        kb.view_workspace.as_str(),
        kb.view_repo.as_str(),
        kb.view_owner.as_str(),
        kb.view_label.as_str(),
    ];
    for pv in plugin_views {
        keys.push(pv.key.as_str());
    }
    keys
}

/// Translate a pressed key into a view switch, if it matches any view keybind.
pub(super) fn match_view_switch(
    kb: &config::model::KeybindsConfig,
    plugin_views: &[plugin::PluginViewInfo],
    key: &str,
) -> Option<ViewMode> {
    if key == kb.view_tree {
        Some(ViewMode::Tree)
    } else if key == kb.view_workspace {
        Some(ViewMode::Workspace)
    } else if key == kb.view_repo {
        Some(ViewMode::Repo)
    } else if key == kb.view_owner {
        Some(ViewMode::Owner)
    } else if key == kb.view_label {
        Some(ViewMode::Label)
    } else {
        for pv in plugin_views {
            if key == pv.key {
                return Some(ViewMode::Plugin {
                    view_name: pv.view_name.clone(),
                    plugin_name: pv.plugin_name.clone(),
                });
            }
        }
        None
    }
}

pub(super) fn view_header(
    current: &str,
    kb: &config::model::KeybindsConfig,
    plugin_views: &[plugin::PluginViewInfo],
) -> String {
    let mut header = format!(
        "view: {} │ {}:tree {}:workspace {}:repo {}:owner {}:label",
        current,
        kb.view_tree,
        kb.view_workspace,
        kb.view_repo,
        kb.view_owner,
        kb.view_label,
    );
    for pv in plugin_views {
        header.push_str(&format!(" {}:{}", pv.key, pv.label));
    }
    header
}
