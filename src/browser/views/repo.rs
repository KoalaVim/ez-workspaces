use std::path::Path;

use colored::Colorize;

use crate::config;
use crate::error::Result;
use crate::plugin;
use crate::repo;

use super::super::domain::rows::repo_view_rows;
use super::super::domain::{to_items, Row, RowKind};
use super::super::selector::{ActionResult, InteractiveSelector};
use super::super::{browse_repo, parse_label_input, BranchCache, SortMode};
use super::{match_view_switch, view_header, view_switch_keys, Outcome, ViewMode};

pub(super) fn run(
    selector: &dyn InteractiveSelector,
    config: &config::model::EzConfig,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
    branch_cache: &BranchCache,
) -> Result<Outcome> {
    let plugin_views = plugin::collect_plugin_views("repo", config).unwrap_or_default();
    let mut sort_mode = SortMode::from_config(&config.default_sort);

    let ez_bin = std::env::current_exe().ok();
    let preview_cmd = ez_bin.map(|bin| format!("{} preview {{}}", bin.display()));

    loop {
        let index = repo::store::load_index()?;
        if index.repos.is_empty() {
            println!(
                "{}",
                "No registered repos. Use `ez add` or `ez clone`.".yellow()
            );
            return Ok(Outcome::Done);
        }

        let rows = repo_view_rows(&index, sort_mode, branch_cache);
        let items = to_items(&rows);

        let header = format!(
            "{}  sort: {} ({})",
            view_header("repo", &config.keybinds, &plugin_views),
            sort_mode.label(),
            config.keybinds.sort_toggle,
        );
        let keys = {
            let mut k = view_switch_keys(&config.keybinds, &plugin_views);
            k.push(config.keybinds.edit_labels.as_str());
            k.push(config.keybinds.remove_repo.as_str());
            k.push(config.keybinds.sort_toggle.as_str());
            k
        };

        let action = selector.select_with_actions(
            &items,
            "repos",
            preview_cmd.as_deref(),
            &keys,
            Some(&header),
        )?;

        match action {
            ActionResult::Cancel => return Ok(Outcome::Done),
            ActionResult::Action(key, idx) => {
                if key == config.keybinds.sort_toggle {
                    sort_mode = sort_mode.toggle();
                    log::debug!("repo view: sort toggled to {:?}", sort_mode);
                    continue;
                }
                if let Some(next) = match_view_switch(&config.keybinds, &plugin_views, &key) {
                    return Ok(Outcome::Switch(next));
                }
                if key == config.keybinds.edit_labels {
                    let Some(entry) = row_entry(&rows[idx]) else {
                        continue;
                    };
                    let meta = repo::store::load_repo_meta(&entry.id).unwrap_or_default();
                    let current = meta.labels.join(",");
                    let input =
                        selector.input("Labels (comma-sep; prefix - to remove)", Some(&current))?;
                    let (add, remove) = parse_label_input(&input);
                    let result = repo::set_repo_labels(&entry.id, &add, &remove)?;
                    eprintln!(
                        "{} {} → {}",
                        "Labels on".green(),
                        entry.name.bold(),
                        if result.is_empty() {
                            "(none)".dimmed().to_string()
                        } else {
                            result.join(", ").magenta().to_string()
                        }
                    );
                    continue;
                }
                if key == config.keybinds.remove_repo {
                    let Some(entry) = row_entry(&rows[idx]) else {
                        continue;
                    };
                    let msg = format!("Remove repo '{}'?", entry.name);
                    if selector.confirm(&msg, false)? {
                        let mut idx_mut = repo::store::load_index()?;
                        idx_mut.remove_by_id(&entry.id);
                        repo::store::save_index(&idx_mut)?;
                        eprintln!("{} {}", "Removed:".green(), entry.name.bold());
                    }
                    continue;
                }
            }
            ActionResult::Select(idx) => {
                let Some(entry) = row_entry(&rows[idx]) else {
                    continue;
                };
                if browse_repo(
                    &entry.path,
                    selector,
                    cd_file,
                    post_cmd_file,
                    config,
                    branch_cache,
                )? {
                    return Ok(Outcome::Done);
                }
                return Ok(Outcome::Switch(ViewMode::Repo));
            }
        }
    }
}

fn row_entry(row: &Row) -> Option<&repo::model::RepoEntry> {
    match &row.kind {
        RowKind::Repo { entry, .. } => entry.as_ref(),
        _ => None,
    }
}
