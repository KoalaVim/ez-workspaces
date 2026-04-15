use std::path::Path;

use colored::Colorize;

use crate::config;
use crate::error::Result;
use crate::paths;

use super::super::selector::{ActionResult, InteractiveSelector, SelectItem};
use super::super::{browse_repo, drill_into_directory};
use super::{match_view_switch, view_header, view_switch_keys, Outcome, ViewMode};

pub(super) fn run(
    selector: &dyn InteractiveSelector,
    config: &config::model::EzConfig,
    cd_file: Option<&Path>,
    jump: Option<&str>,
) -> Result<Outcome> {
    if config.workspace_roots.is_empty() {
        println!("{}", "No workspace roots configured.".yellow());
        println!("Add roots to your config with: {}", "ez config --edit".bold());
        println!("Example: workspace_roots = [\"~/workspace\"]");
        return Ok(Outcome::Done);
    }

    let root_path = if let Some(ws_raw) = jump {
        let ws = ws_raw.trim_end_matches('/');
        let matched = config.workspace_roots.iter().find(|r| {
            let r_trimmed = r.trim_end_matches('/');
            let expanded = paths::expand_tilde(r);
            let collapsed = paths::collapse_tilde(&expanded.to_string_lossy());
            let collapsed_trimmed = collapsed.trim_end_matches('/');
            let expanded_trimmed =
                expanded.to_string_lossy().trim_end_matches('/').to_string();
            let dir_name = expanded
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            r_trimmed == ws
                || collapsed_trimmed == ws
                || expanded_trimmed == ws
                || dir_name == ws
        });
        match matched {
            Some(r) => paths::expand_tilde(r),
            None => {
                eprintln!(
                    "{} workspace '{}' not found in config roots",
                    "ez:".red().bold(),
                    ws
                );
                return Ok(Outcome::Done);
            }
        }
    } else {
        let root_items: Vec<SelectItem> = config
            .workspace_roots
            .iter()
            .map(|r| {
                let expanded = paths::expand_tilde(r);
                SelectItem {
                    display: paths::collapse_tilde(&expanded.to_string_lossy()),
                    value: expanded.to_string_lossy().to_string(),
                }
            })
            .collect();

        let header = view_header("workspace", &config.keybinds);
        let action = selector.select_with_actions(
            &root_items,
            "workspace",
            None,
            &view_switch_keys(&config.keybinds),
            Some(&header),
        )?;

        match action {
            ActionResult::Cancel => return Ok(Outcome::Done),
            ActionResult::Action(key, _) => {
                return match match_view_switch(&config.keybinds, &key) {
                    Some(next) => Ok(Outcome::Switch(next)),
                    None => Ok(Outcome::Done),
                }
            }
            ActionResult::Select(idx) => paths::expand_tilde(&root_items[idx].value),
        }
    };

    // Drill into directories to find a repo.
    let repo_path = drill_into_directory(&root_path, selector)?;
    let repo_path = match repo_path {
        Some(p) => p,
        None => return Ok(Outcome::Switch(ViewMode::Workspace)),
    };

    browse_repo(&repo_path, selector, cd_file, &config.keybinds)?;
    Ok(Outcome::Done)
}
