use std::path::Path;

use colored::Colorize;

use crate::config;
use crate::error::Result;
use crate::plugin;

use super::super::domain::rows::workspace_root_rows;
use super::super::domain::{to_items, RowKind};
use super::super::drill::match_workspace_root;
use super::super::selector::{ActionResult, InteractiveSelector};
use super::super::{browse_repo, drill_into_directory, BranchCache};
use super::{match_view_switch, view_header, view_switch_keys, Outcome, ViewMode};

pub(super) fn run(
    selector: &dyn InteractiveSelector,
    config: &config::model::EzConfig,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
    jump: Option<&str>,
    branch_cache: &BranchCache,
) -> Result<Outcome> {
    if config.workspace_roots.is_empty() {
        println!("{}", "No workspace roots configured.".yellow());
        println!(
            "Add roots to your config with: {}",
            "ez config --edit".bold()
        );
        println!("Example: workspace_roots = [\"~/workspace\"]");
        return Ok(Outcome::Done);
    }

    let plugin_views = plugin::collect_plugin_views("workspace", config).unwrap_or_default();

    let root_path = if let Some(ws_raw) = jump {
        let matched = match_workspace_root(config, ws_raw);
        match matched {
            Some(r) => r,
            None => {
                eprintln!(
                    "{} workspace '{}' not found in config roots",
                    "ez:".red().bold(),
                    ws_raw.trim_end_matches('/')
                );
                return Ok(Outcome::Done);
            }
        }
    } else {
        let root_rows = workspace_root_rows(config);
        let root_items = to_items(&root_rows);

        let header = view_header("workspace", &config.keybinds, &plugin_views);
        let action = selector.select_with_actions(
            &root_items,
            "workspace",
            None,
            &view_switch_keys(&config.keybinds, &plugin_views),
            Some(&header),
        )?;

        match action {
            ActionResult::Cancel => return Ok(Outcome::Done),
            ActionResult::Action(key, _) => {
                return match match_view_switch(
                    &config.keybinds,
                    &plugin_views,
                    &ViewMode::Workspace,
                    &key,
                ) {
                    Some(next) => Ok(Outcome::Switch(next)),
                    None => Ok(Outcome::Done),
                }
            }
            ActionResult::Select(idx) => match &root_rows[idx].kind {
                RowKind::Root(path) => path.clone(),
                _ => return Ok(Outcome::Done),
            },
        }
    };

    // Drill into directories to find a repo (supports clone keybind).
    let repo_path = drill_into_directory(&root_path, selector, config, branch_cache)?;
    let repo_path = match repo_path {
        Some(p) => p,
        None => return Ok(Outcome::Switch(ViewMode::Workspace)),
    };

    if browse_repo(
        &repo_path,
        selector,
        cd_file,
        post_cmd_file,
        config,
        branch_cache,
    )? {
        Ok(Outcome::Done)
    } else {
        Ok(Outcome::Switch(ViewMode::Workspace))
    }
}
