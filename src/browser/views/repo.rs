use std::path::Path;

use colored::Colorize;

use crate::config;
use crate::error::Result;
use crate::paths;
use crate::plugin;
use crate::repo;

use super::super::selector::{ActionResult, InteractiveSelector, SelectItem};
use super::super::{browse_repo, format_repo_display, get_branch, parse_label_input, SortMode};
use super::{match_view_switch, view_header, view_switch_keys, Outcome, ViewMode};

pub(super) fn run(
    selector: &dyn InteractiveSelector,
    config: &config::model::EzConfig,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
) -> Result<Outcome> {
    let index = repo::store::load_index()?;
    if index.repos.is_empty() {
        println!(
            "{}",
            "No registered repos. Use `ez add` or `ez clone`.".yellow()
        );
        return Ok(Outcome::Done);
    }

    let plugin_views = plugin::collect_plugin_views("repo", config).unwrap_or_default();
    let mut sort_mode = SortMode::from_config(&config.default_sort);

    let ez_bin = std::env::current_exe().ok();
    let preview_cmd = ez_bin.map(|bin| format!("{} preview {{}}", bin.display()));

    loop {
        let mut repo_entries: Vec<&repo::model::RepoEntry> = index.repos.iter().collect();

        if sort_mode == SortMode::Lru {
            repo_entries.sort_by(|a, b| {
                let a_ts = repo::store::load_repo_meta(&a.id)
                    .ok()
                    .and_then(|m| m.last_accessed);
                let b_ts = repo::store::load_repo_meta(&b.id)
                    .ok()
                    .and_then(|m| m.last_accessed);
                match (&b_ts, &a_ts) {
                    (Some(b_v), Some(a_v)) => b_v.cmp(a_v),
                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            });
        }

        let items: Vec<SelectItem> = repo_entries
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

        let base_header = view_header("repo", &config.keybinds, &plugin_views);
        let header = format!(
            "sort: {} ({})  {}",
            sort_mode.label(),
            config.keybinds.sort_toggle,
            base_header,
        );
        let keys = {
            let mut k = view_switch_keys(&config.keybinds, &plugin_views);
            k.push(config.keybinds.edit_labels.as_str());
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
                    let entry = repo_entries[idx];
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
            }
            ActionResult::Select(idx) => {
                let entry = repo_entries[idx];
                if browse_repo(&entry.path, selector, cd_file, post_cmd_file, config)? {
                    return Ok(Outcome::Done);
                }
                return Ok(Outcome::Switch(ViewMode::Repo));
            }
        }
    }
}
