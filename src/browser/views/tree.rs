use std::fs;
use std::path::{Path, PathBuf};

use colored::Colorize;

use crate::config;
use crate::error::Result;
use crate::paths;
use crate::plugin;
use crate::repo;
use crate::session;

use super::super::selector::{ActionResult, InteractiveSelector, SelectItem};
use super::super::{accept_session, browse_repo, get_branch};
use super::{match_view_switch, view_header, view_switch_keys, Outcome, ViewMode};

/// Distinguishes tree nodes so selection can dispatch to the right action.
enum NodeKind {
    Root,
    Repo(PathBuf),
    Session {
        repo_entry: repo::model::RepoEntry,
        session: Box<session::model::Session>,
        target_dir: PathBuf,
    },
}

pub(super) fn run(
    selector: &dyn InteractiveSelector,
    config: &config::model::EzConfig,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
) -> Result<Outcome> {
    let index = repo::store::load_index()?;
    let mut nodes: Vec<(String, NodeKind, PathBuf)> = Vec::new();
    let num_roots = config.workspace_roots.len();

    for (root_i, root) in config.workspace_roots.iter().enumerate() {
        let root_path = paths::expand_tilde(root);
        let is_last_root = root_i == num_roots - 1;
        let root_connector = if is_last_root {
            "└── "
        } else {
            "├── "
        };
        let root_cont = if is_last_root { "    " } else { "│   " };

        nodes.push((
            format!("{}{}", root_connector.dimmed(), root.bold().blue()),
            NodeKind::Root,
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
            let repo_connector = if is_last_repo {
                "└── "
            } else {
                "├── "
            };
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
                NodeKind::Repo(repo_path.clone()),
                repo_path.clone(),
            ));

            if let Some(repo_entry) = index.find_by_path(repo_path) {
                if let Ok(tree) = session::store::load_sessions(&repo_entry.id) {
                    if !tree.sessions.is_empty() {
                        let rendered = tree.render_tree();
                        let num_sessions = rendered.len();
                        for (sess_i, node) in rendered.iter().enumerate() {
                            let is_last_session = sess_i == num_sessions - 1;
                            let sess_connector = if is_last_session {
                                "└── "
                            } else {
                                "├── "
                            };
                            let session_prefix = session::tree::format_session_tree_line(node);
                            let marker = if node.session.is_default {
                                " ★".yellow().to_string()
                            } else {
                                String::new()
                            };

                            let target_dir = node
                                .session
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
                                    session_prefix.dimmed(),
                                    node.session.name.bold().yellow(),
                                    marker,
                                ),
                                NodeKind::Session {
                                    repo_entry: repo_entry.clone(),
                                    session: Box::new(node.session.clone()),
                                    target_dir,
                                },
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

    let plugin_views = plugin::collect_plugin_views("tree", config).unwrap_or_default();

    let items: Vec<SelectItem> = nodes
        .iter()
        .enumerate()
        .map(|(i, (display, _, preview_path))| SelectItem {
            display: display.clone(),
            value: format!("{}:{}", i, preview_path.to_string_lossy()),
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
        ActionResult::Select(idx) => match &nodes[idx].1 {
            NodeKind::Root => Ok(Outcome::Switch(ViewMode::Tree)),
            NodeKind::Repo(path) => {
                if browse_repo(path, selector, cd_file, post_cmd_file, config)? {
                    Ok(Outcome::Done)
                } else {
                    Ok(Outcome::Switch(ViewMode::Tree))
                }
            }
            NodeKind::Session {
                repo_entry,
                session,
                target_dir,
            } => {
                accept_session(
                    &config.on_enter,
                    repo_entry,
                    session,
                    target_dir,
                    cd_file,
                    post_cmd_file,
                    config,
                )?;
                Ok(Outcome::Done)
            }
        },
    }
}
