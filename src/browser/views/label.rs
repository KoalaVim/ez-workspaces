use std::path::Path;

use colored::Colorize;

use crate::config;
use crate::error::Result;
use crate::plugin;
use crate::repo;

use super::super::domain::rows::label_rows;
use super::super::domain::{to_items, RowKind};
use super::super::selector::{ActionResult, InteractiveSelector};
use super::super::{browse_repo, write_cd_target, BranchCache};
use super::{match_view_switch, view_header, view_switch_keys, Outcome, ViewMode};

pub(super) fn run(
    selector: &dyn InteractiveSelector,
    config: &config::model::EzConfig,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
    branch_cache: &BranchCache,
) -> Result<Outcome> {
    let index = repo::store::load_index()?;
    let labels = label_rows(&index);

    if labels.is_empty() {
        println!(
            "{}",
            "No labels found. Use `ez repo label add` or `ez session label add`.".yellow()
        );
        return Ok(Outcome::Done);
    }

    let plugin_views = plugin::collect_plugin_views("label", config).unwrap_or_default();

    let items = to_items(&labels);

    let header = view_header("label", &config.keybinds, &plugin_views);

    let action = selector.select_with_actions(
        &items,
        "label",
        None,
        &view_switch_keys(&config.keybinds, &plugin_views),
        Some(&header),
    )?;

    let label_idx = match action {
        ActionResult::Cancel => return Ok(Outcome::Done),
        ActionResult::Action(key, _) => {
            return match match_view_switch(&config.keybinds, &plugin_views, &ViewMode::Label, &key)
            {
                Some(next) => Ok(Outcome::Switch(next)),
                None => Ok(Outcome::Done),
            }
        }
        ActionResult::Select(idx) => idx,
    };

    let RowKind::Label { children: tagged } = &labels[label_idx].kind else {
        return Ok(Outcome::Done);
    };
    let sub_items = to_items(tagged);

    let sub_action = selector.select_with_actions(
        &sub_items,
        "tagged",
        None,
        &view_switch_keys(&config.keybinds, &plugin_views),
        Some(&header),
    )?;

    match sub_action {
        ActionResult::Cancel => Ok(Outcome::Switch(ViewMode::Label)),
        ActionResult::Action(key, _) => {
            match match_view_switch(&config.keybinds, &plugin_views, &ViewMode::Label, &key) {
                Some(next) => Ok(Outcome::Switch(next)),
                None => Ok(Outcome::Done),
            }
        }
        ActionResult::Select(idx) => match &tagged[idx].kind {
            RowKind::Repo { path, .. } => {
                if browse_repo(path, selector, cd_file, post_cmd_file, config, branch_cache)? {
                    Ok(Outcome::Done)
                } else {
                    Ok(Outcome::Switch(ViewMode::Label))
                }
            }
            RowKind::Session { target_dir, .. } => {
                write_cd_target(cd_file, target_dir)?;
                Ok(Outcome::Done)
            }
            _ => Ok(Outcome::Done),
        },
    }
}
