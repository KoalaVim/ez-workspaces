use std::path::Path;

use colored::Colorize;

use crate::config;
use crate::error::Result;
use crate::paths;
use crate::repo;

use super::super::selector::{ActionResult, InteractiveSelector, SelectItem};
use super::super::{browse_repo, get_branch, parse_label_input};
use super::{match_view_switch, view_header, view_switch_keys, Outcome};

pub(super) fn run(
    selector: &dyn InteractiveSelector,
    config: &config::model::EzConfig,
    cd_file: Option<&Path>,
) -> Result<Outcome> {
    let index = repo::store::load_index()?;
    if index.repos.is_empty() {
        println!("{}", "No registered repos. Use `ez add` or `ez clone`.".yellow());
        return Ok(Outcome::Done);
    }

    let items: Vec<SelectItem> = index
        .repos
        .iter()
        .map(|r| {
            let branch = get_branch(&r.path).unwrap_or_else(|| "?".into());
            let meta = repo::store::load_repo_meta(&r.id).unwrap_or_default();
            let labels = if meta.labels.is_empty() {
                String::new()
            } else {
                format!(" [{}]", meta.labels.join(",")).magenta().to_string()
            };
            SelectItem {
                display: format!(
                    "{} {} {}{}",
                    r.name.bold().green(),
                    paths::collapse_tilde(&r.path.to_string_lossy()).dimmed(),
                    format!("[{branch}]").cyan(),
                    labels,
                ),
                value: r.path.to_string_lossy().to_string(),
            }
        })
        .collect();

    let ez_bin = std::env::current_exe().ok();
    let preview_cmd = ez_bin.map(|bin| format!("{} preview {{}}", bin.display()));
    let header = view_header("repo", &config.keybinds);
    let keys = {
        let mut k = view_switch_keys(&config.keybinds);
        k.push(config.keybinds.edit_labels.as_str());
        k
    };

    loop {
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
                if let Some(next) = match_view_switch(&config.keybinds, &key) {
                    return Ok(Outcome::Switch(next));
                }
                if key == config.keybinds.edit_labels {
                    let entry = &index.repos[idx];
                    let meta = repo::store::load_repo_meta(&entry.id).unwrap_or_default();
                    let current = meta.labels.join(",");
                    let input = selector.input(
                        "Labels (comma-sep; prefix - to remove)",
                        Some(&current),
                    )?;
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
                let entry = &index.repos[idx];
                browse_repo(&entry.path, selector, cd_file, &config.keybinds)?;
                return Ok(Outcome::Done);
            }
        }
    }
}
