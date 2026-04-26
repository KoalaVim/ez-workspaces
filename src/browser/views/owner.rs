use std::collections::BTreeMap;
use std::path::Path;

use colored::Colorize;

use crate::config;
use crate::error::Result;
use crate::paths;
use crate::plugin;
use crate::repo;

use super::super::selector::{ActionResult, InteractiveSelector, SelectItem};
use super::super::{browse_repo, format_repo_display, get_branch};
use super::{match_view_switch, view_header, view_switch_keys, Outcome, ViewMode};

pub(super) fn run(
    selector: &dyn InteractiveSelector,
    config: &config::model::EzConfig,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
) -> Result<Outcome> {
    let index = repo::store::load_index()?;
    if index.repos.is_empty() {
        println!("{}", "No registered repos. Use `ez add` or `ez clone`.".yellow());
        return Ok(Outcome::Done);
    }

    let plugin_views = plugin::collect_plugin_views("owner", config).unwrap_or_default();

    let mut by_owner: BTreeMap<String, Vec<repo::model::RepoEntry>> = BTreeMap::new();
    for entry in &index.repos {
        let owner = owner_for_repo(entry).unwrap_or_else(|| "(unknown)".to_string());
        by_owner.entry(owner).or_default().push(entry.clone());
    }

    let owners: Vec<(String, Vec<repo::model::RepoEntry>)> = by_owner.into_iter().collect();
    let items: Vec<SelectItem> = owners
        .iter()
        .map(|(owner, entries)| SelectItem {
            display: format!(
                "{} {}",
                owner.bold().purple(),
                format!(
                    "({} repo{})",
                    entries.len(),
                    if entries.len() == 1 { "" } else { "s" }
                )
                .dimmed()
            ),
            value: owner.clone(),
        })
        .collect();

    let header = view_header("owner", &config.keybinds, &plugin_views);

    let action = selector.select_with_actions(
        &items,
        "owner",
        None,
        &view_switch_keys(&config.keybinds, &plugin_views),
        Some(&header),
    )?;

    let owner_idx = match action {
        ActionResult::Cancel => return Ok(Outcome::Done),
        ActionResult::Action(key, _) => {
            return match match_view_switch(&config.keybinds, &plugin_views, &key) {
                Some(next) => Ok(Outcome::Switch(next)),
                None => Ok(Outcome::Done),
            }
        }
        ActionResult::Select(idx) => idx,
    };

    let (_owner, entries) = &owners[owner_idx];
    let sub_items: Vec<SelectItem> = entries
        .iter()
        .map(|r| {
            let branch = get_branch(&r.path).unwrap_or_else(|| "?".into());
            let meta = repo::store::load_repo_meta(&r.id).unwrap_or_default();
            let path_str = paths::collapse_tilde(&r.path.to_string_lossy());
            SelectItem {
                display: format_repo_display(
                    &r.name,
                    Some(&path_str),
                    Some(&branch),
                    &meta.labels,
                ),
                value: r.path.to_string_lossy().to_string(),
            }
        })
        .collect();

    let ez_bin = std::env::current_exe().ok();
    let preview_cmd = ez_bin.map(|bin| format!("{} preview {{}}", bin.display()));

    let sub_action = selector.select_with_actions(
        &sub_items,
        "repo",
        preview_cmd.as_deref(),
        &view_switch_keys(&config.keybinds, &plugin_views),
        Some(&header),
    )?;

    match sub_action {
        ActionResult::Cancel => Ok(Outcome::Switch(ViewMode::Owner)),
        ActionResult::Action(key, _) => match match_view_switch(&config.keybinds, &plugin_views, &key) {
            Some(next) => Ok(Outcome::Switch(next)),
            None => Ok(Outcome::Done),
        },
        ActionResult::Select(idx) => {
            let entry = &entries[idx];
            browse_repo(&entry.path, selector, cd_file, post_cmd_file, config)?;
            Ok(Outcome::Done)
        }
    }
}

fn owner_for_repo(entry: &repo::model::RepoEntry) -> Option<String> {
    let meta = repo::store::load_repo_meta(&entry.id).unwrap_or_default();
    if let Some(url) = meta.remote_url.as_deref() {
        if let Some(owner) = repo::model::parse_owner(url) {
            return Some(owner);
        }
    }
    // Fallback: immediate parent directory name of the repo path.
    entry
        .path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
}
