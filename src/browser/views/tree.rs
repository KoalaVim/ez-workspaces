use std::fs;
use std::path::{Path, PathBuf};

use colored::Colorize;

use crate::config;
use crate::error::Result;
use crate::paths;
use crate::repo;
use crate::session;

use super::super::selector::{ActionResult, InteractiveSelector, SelectItem};
use super::super::{get_branch, write_cd_target};
use super::{match_view_switch, view_header, view_switch_keys, Outcome, ViewMode};

pub(super) fn run(
    selector: &dyn InteractiveSelector,
    config: &config::model::EzConfig,
    cd_file: Option<&Path>,
) -> Result<Outcome> {
    let index = repo::store::load_index()?;
    // nodes: (display, Option<target-dir>, preview-path)
    let mut nodes: Vec<(String, Option<PathBuf>, PathBuf)> = Vec::new();
    let num_roots = config.workspace_roots.len();

    for (root_i, root) in config.workspace_roots.iter().enumerate() {
        let root_path = paths::expand_tilde(root);
        let is_last_root = root_i == num_roots - 1;
        let root_connector = if is_last_root { "└── " } else { "├── " };
        let root_cont = if is_last_root { "    " } else { "│   " };

        nodes.push((
            format!("{}{}", root_connector.dimmed(), root.bold().blue()),
            None,
            root_path.clone(),
        ));

        if !root_path.is_dir() {
            continue;
        }

        let mut repos: Vec<(String, PathBuf)> = Vec::new();
        if let Ok(read_dir) = fs::read_dir(&root_path) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if name.starts_with('.') {
                    continue;
                }
                if path.join(".git").exists() {
                    repos.push((name, path));
                }
            }
        }
        repos.sort_by(|a, b| a.0.cmp(&b.0));

        let num_repos = repos.len();
        for (repo_i, (repo_name, repo_path)) in repos.iter().enumerate() {
            let is_last_repo = repo_i == num_repos - 1;
            let repo_connector = if is_last_repo { "└── " } else { "├── " };
            let repo_cont = if is_last_repo { "    " } else { "│   " };

            let branch = get_branch(repo_path).unwrap_or_else(|| "?".into());

            nodes.push((
                format!(
                    "{}{}{} {}",
                    root_cont.dimmed(),
                    repo_connector.dimmed(),
                    repo_name.bold().green(),
                    format!("[{branch}]").cyan(),
                ),
                Some(repo_path.clone()),
                repo_path.clone(),
            ));

            if let Some(repo_entry) = index.find_by_path(repo_path) {
                if let Ok(tree) = session::store::load_sessions(&repo_entry.id) {
                    if !tree.sessions.is_empty() {
                        let rendered = tree.render_tree();
                        let num_sessions = rendered.len();
                        for (sess_i, (depth, s)) in rendered.iter().enumerate() {
                            let is_last_session = sess_i == num_sessions - 1;
                            let sess_connector =
                                if is_last_session { "└── " } else { "├── " };
                            let session_indent = "    ".repeat(*depth);
                            let marker = if s.is_default {
                                " ★".yellow().to_string()
                            } else {
                                String::new()
                            };

                            let target = s
                                .path
                                .as_ref()
                                .cloned()
                                .unwrap_or_else(|| repo_path.clone());

                            nodes.push((
                                format!(
                                    "{}{}{}{}{}{}",
                                    root_cont.dimmed(),
                                    repo_cont.dimmed(),
                                    sess_connector.dimmed(),
                                    session_indent,
                                    s.name.bold().yellow(),
                                    marker,
                                ),
                                Some(target),
                                repo_path.clone(),
                            ));
                        }
                    }
                }
            }
        }
    }

    if nodes.is_empty() {
        println!("{}", "No repositories found in workspace roots.".yellow());
        return Ok(Outcome::Done);
    }

    let items: Vec<SelectItem> = nodes
        .iter()
        .map(|(display, _target, preview_path)| SelectItem {
            display: display.clone(),
            value: preview_path.to_string_lossy().to_string(),
        })
        .collect();

    let ez_bin = std::env::current_exe().ok();
    let preview_cmd = ez_bin.map(|bin| format!("{} preview {{}}", bin.display()));
    let header = view_header("tree", &config.keybinds);

    let action = selector.select_with_actions(
        &items,
        "ez tree",
        preview_cmd.as_deref(),
        &view_switch_keys(&config.keybinds),
        Some(&header),
    )?;

    match action {
        ActionResult::Cancel => Ok(Outcome::Done),
        ActionResult::Action(key, _) => match match_view_switch(&config.keybinds, &key) {
            Some(next) => Ok(Outcome::Switch(next)),
            None => Ok(Outcome::Done),
        },
        ActionResult::Select(idx) => match &nodes[idx].1 {
            Some(target) => {
                write_cd_target(cd_file, target)?;
                Ok(Outcome::Done)
            }
            None => Ok(Outcome::Switch(ViewMode::Tree)),
        },
    }
}
