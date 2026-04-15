//! Top-level view dispatcher for the interactive browser.
//!
//! Each view renders an fzf selector for a different "type" of top-level item
//! (tree, workspace, repo, owner, label). A set of view-switch keybinds is
//! registered with `--expect`; pressing one exits the current fzf instance and
//! the dispatch loop re-enters the chosen view.

mod label;
mod owner;
mod repo;
mod tree;
mod workspace;

use std::path::Path;

use crate::config;
use crate::error::{EzError, Result};

use super::selector::InteractiveSelector;

/// Top-level view modes selectable via keybinds.
#[derive(Clone, Copy, Debug)]
pub enum ViewMode {
    Tree,
    Workspace,
    Repo,
    Owner,
    Label,
}

impl ViewMode {
    pub fn from_flag(v: &str) -> Result<Self> {
        match v.to_ascii_lowercase().as_str() {
            "tree" | "t" => Ok(ViewMode::Tree),
            "workspace" | "ws" | "w" => Ok(ViewMode::Workspace),
            "repo" | "repos" | "r" => Ok(ViewMode::Repo),
            "owner" | "owners" | "o" => Ok(ViewMode::Owner),
            "label" | "labels" | "l" => Ok(ViewMode::Label),
            other => Err(EzError::Config(format!(
                "unknown view '{other}' — expected one of: tree, workspace, repo, owner, label"
            ))),
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
) -> Result<()> {
    let mut mode = initial;
    // --workspace short-circuits the root picker for the Workspace view only.
    let mut workspace_jump = workspace_override.map(|s| s.to_string());

    loop {
        let outcome = match mode {
            ViewMode::Tree => tree::run(selector, config, cd_file)?,
            ViewMode::Workspace => {
                let jump = workspace_jump.take();
                workspace::run(selector, config, cd_file, jump.as_deref())?
            }
            ViewMode::Repo => repo::run(selector, config, cd_file)?,
            ViewMode::Owner => owner::run(selector, config, cd_file)?,
            ViewMode::Label => label::run(selector, config, cd_file)?,
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
pub(super) fn view_switch_keys(kb: &config::model::KeybindsConfig) -> Vec<&str> {
    vec![
        kb.view_tree.as_str(),
        kb.view_workspace.as_str(),
        kb.view_repo.as_str(),
        kb.view_owner.as_str(),
        kb.view_label.as_str(),
    ]
}

/// Translate a pressed key into a view switch, if it matches any view keybind.
pub(super) fn match_view_switch(
    kb: &config::model::KeybindsConfig,
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
        None
    }
}

pub(super) fn view_header(current: &str, kb: &config::model::KeybindsConfig) -> String {
    format!(
        "view: {} │ {}:tree {}:workspace {}:repo {}:owner {}:label",
        current,
        kb.view_tree,
        kb.view_workspace,
        kb.view_repo,
        kb.view_owner,
        kb.view_label,
    )
}
