use std::path::Path;

use colored::Colorize;

use crate::config;
use crate::error::Result;
use crate::plugin;
use crate::repo;

use super::super::domain::rows::tree_rows;
use super::super::domain::{to_items, Row, RowKind};
use super::super::selector::{ActionResult, InteractiveSelector, SelectItem};
use super::super::{accept_session, browse_repo, BranchCache};
use super::{match_view_switch, view_header, view_switch_keys, Outcome, ViewMode};

pub(super) fn run(
    selector: &dyn InteractiveSelector,
    config: &config::model::EzConfig,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
    branch_cache: &BranchCache,
) -> Result<Outcome> {
    let index = repo::store::load_index()?;
    let rows: Vec<Row> = tree_rows(config, &index, branch_cache);

    if rows.is_empty() {
        println!("{}", "No repositories found in workspace roots.".yellow());
        return Ok(Outcome::Done);
    }

    let plugin_views = plugin::collect_plugin_views("tree", config).unwrap_or_default();

    // fzf values are `<index>:<preview path>` so identical paths stay distinct.
    let items: Vec<SelectItem> = to_items(&rows)
        .into_iter()
        .enumerate()
        .map(|(i, item)| SelectItem {
            value: format!("{}:{}", i, item.value),
            ..item
        })
        .collect();

    let ez_bin = std::env::current_exe().ok();
    let preview_cmd = ez_bin.map(|bin| {
        format!(
            "{} preview \"$(printf '%s' {{}} | cut -d: -f2-)\"",
            bin.display()
        )
    });
    let header = view_header("tree", &config.keybinds, &plugin_views);

    let action = selector.select_with_actions(
        &items,
        "ez tree",
        preview_cmd.as_deref(),
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
        ActionResult::Select(idx) => match &rows[idx].kind {
            RowKind::Repo { path, .. } => {
                if browse_repo(path, selector, cd_file, post_cmd_file, config, branch_cache)? {
                    Ok(Outcome::Done)
                } else {
                    Ok(Outcome::Switch(ViewMode::Tree))
                }
            }
            RowKind::Session {
                repo,
                session,
                target_dir,
            } => {
                accept_session(
                    &config.on_enter,
                    repo,
                    session,
                    target_dir,
                    cd_file,
                    post_cmd_file,
                    config,
                )?;
                Ok(Outcome::Done)
            }
            // Roots (and anything else) just reload the tree.
            _ => Ok(Outcome::Switch(ViewMode::Tree)),
        },
    }
}
