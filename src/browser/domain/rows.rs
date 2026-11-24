//! Row builders for the Tree, Workspace, Repo, Owner and Label views and the
//! directory drill-down. They take config/index data plus the branch cache
//! and return `Vec<Row>`; no selector or terminal is involved.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use colored::Colorize;

use crate::config::model::EzConfig;
use crate::paths;
use crate::repo;
use crate::repo::model::{RepoEntry, RepoIndex};
use crate::session;
use crate::session::build_worktree_info;

use super::super::format::{format_branch_indicator, format_repo_display};
use super::super::git::BranchCache;
use super::super::SortMode;
use super::{Row, RowKind};

/// Tree view: workspace roots, the repos under them and their sessions.
/// `Row::value` is the path the preview should render (the repo or root path).
pub(crate) fn tree_rows(
    config: &EzConfig,
    index: &RepoIndex,
    branch_cache: &BranchCache,
) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
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

        rows.push(Row {
            display: format!("{}{}", root_connector.dimmed(), root.bold().blue()),
            value: root_path.to_string_lossy().to_string(),
            kind: RowKind::Root(root_path.clone()),
        });

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

        // Pre-resolve all repo branches in parallel
        std::thread::scope(|s| {
            for (_, repo_path) in &repos {
                s.spawn(|| branch_cache.get_branch(repo_path));
            }
        });

        let num_repos = repos.len();
        for (repo_i, (repo_name, repo_path)) in repos.iter().enumerate() {
            let is_last_repo = repo_i == num_repos - 1;
            let repo_connector = if is_last_repo {
                "└── "
            } else {
                "├── "
            };
            let repo_cont = if is_last_repo { "    " } else { "│   " };

            let branch = branch_cache
                .get_branch(repo_path)
                .unwrap_or_else(|| "?".into());

            rows.push(Row {
                display: format!(
                    "{}{}{} {}",
                    root_cont.dimmed(),
                    repo_connector.dimmed(),
                    repo_name.bold().green(),
                    format!("[{branch}]").cyan(),
                ),
                value: repo_path.to_string_lossy().to_string(),
                kind: RowKind::Repo {
                    path: repo_path.clone(),
                    entry: index.find_by_path(repo_path).cloned(),
                },
            });

            let Some(repo_entry) = index.find_by_path(repo_path) else {
                continue;
            };
            let Ok(tree) = session::store::load_sessions(&repo_entry.id) else {
                continue;
            };
            if tree.sessions.is_empty() {
                continue;
            }
            let worktree_info = build_worktree_info(repo_entry, &tree);
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

                let session_path = node.session.path.as_deref().unwrap_or(repo_path);
                let branch = worktree_info
                    .get_branch_for_path(session_path)
                    .or_else(|| branch_cache.get_branch(session_path));
                let branch_indicator = format_branch_indicator(branch.as_deref());
                log::debug!(
                    "tree view: session={} branch={:?}",
                    node.session.name,
                    branch
                );

                rows.push(Row {
                    display: format!(
                        "{}{}{}{}{}{}{}",
                        root_cont.dimmed(),
                        repo_cont.dimmed(),
                        sess_connector.dimmed(),
                        session_prefix.dimmed(),
                        node.session.name.bold().yellow(),
                        branch_indicator,
                        marker,
                    ),
                    value: repo_path.to_string_lossy().to_string(),
                    kind: RowKind::Session {
                        repo: repo_entry.clone(),
                        session: Box::new(node.session.clone()),
                        target_dir,
                    },
                });
            }
        }
    }
    rows
}

/// Workspace view: the configured roots.
pub(crate) fn workspace_root_rows(config: &EzConfig) -> Vec<Row> {
    config
        .workspace_roots
        .iter()
        .map(|r| {
            let expanded = paths::expand_tilde(r);
            let display = paths::collapse_tilde(&expanded.to_string_lossy());
            Row {
                display: display.bold().blue().to_string(),
                value: expanded.to_string_lossy().to_string(),
                kind: RowKind::Root(expanded),
            }
        })
        .collect()
}

/// Directory drill-down: sub-directories of `current` (hidden ones skipped),
/// git repos shown with branch and labels. Sorted by display string.
pub(crate) fn directory_rows(
    current: &Path,
    index: &RepoIndex,
    branch_cache: &BranchCache,
) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
    if let Ok(read_dir) = fs::read_dir(current) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir()
                && !path
                    .file_name()
                    .is_none_or(|n| n.to_string_lossy().starts_with('.'))
            {
                let name = path.file_name().unwrap().to_string_lossy().to_string();

                let (display, kind) = if path.join(".git").exists() {
                    let branch = branch_cache.get_branch(&path).unwrap_or_else(|| "?".into());
                    let repo_entry = index.find_by_path(&path);
                    let labels = repo_entry
                        .and_then(|e| repo::store::load_repo_meta(&e.id).ok())
                        .map(|m| m.labels)
                        .unwrap_or_default();
                    (
                        format_repo_display(&name, None, Some(&branch), &labels),
                        RowKind::Repo {
                            path: path.clone(),
                            entry: repo_entry.cloned(),
                        },
                    )
                } else {
                    (
                        name.bold().blue().to_string(),
                        RowKind::Directory(path.clone()),
                    )
                };

                rows.push(Row {
                    display,
                    value: path.to_string_lossy().to_string(),
                    kind,
                });
            }
        }
    }
    rows.sort_by(|a, b| a.display.cmp(&b.display));
    rows
}

/// Rows for a list of registered repos (path, branch, labels), in the given order.
pub(crate) fn repo_rows(entries: &[&RepoEntry], branch_cache: &BranchCache) -> Vec<Row> {
    let branches: Vec<Option<String>> = std::thread::scope(|s| {
        let handles: Vec<_> = entries
            .iter()
            .map(|r| s.spawn(|| branch_cache.get_branch(&r.path)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    entries
        .iter()
        .zip(branches.iter())
        .map(|(r, branch)| {
            let branch_str = branch.as_deref().unwrap_or("?");
            let meta = repo::store::load_repo_meta(&r.id).unwrap_or_default();
            let path_str = paths::collapse_tilde(&r.path.to_string_lossy());
            Row {
                display: format_repo_display(
                    &r.name,
                    Some(&path_str),
                    Some(branch_str),
                    &meta.labels,
                ),
                value: r.path.to_string_lossy().to_string(),
                kind: RowKind::Repo {
                    path: r.path.clone(),
                    entry: Some((*r).clone()),
                },
            }
        })
        .collect()
}

/// Repo view: all registered repos, optionally ordered most-recently-used first.
pub(crate) fn repo_view_rows(
    index: &RepoIndex,
    sort_mode: SortMode,
    branch_cache: &BranchCache,
) -> Vec<Row> {
    let mut entries: Vec<&RepoEntry> = index.repos.iter().collect();

    if sort_mode == SortMode::Lru {
        entries.sort_by(|a, b| {
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
    repo_rows(&entries, branch_cache)
}

/// Owner view: one row per owner, carrying its repos.
pub(crate) fn owner_rows(index: &RepoIndex) -> Vec<Row> {
    let mut by_owner: BTreeMap<String, Vec<RepoEntry>> = BTreeMap::new();
    for entry in &index.repos {
        let owner = owner_for_repo(entry).unwrap_or_else(|| "(unknown)".to_string());
        by_owner.entry(owner).or_default().push(entry.clone());
    }

    by_owner
        .into_iter()
        .map(|(owner, repos)| Row {
            display: format!(
                "{} {}",
                owner.bold().purple(),
                format!(
                    "({} repo{})",
                    repos.len(),
                    if repos.len() == 1 { "" } else { "s" }
                )
                .dimmed()
            ),
            value: owner,
            kind: RowKind::Owner { repos },
        })
        .collect()
}

fn owner_for_repo(entry: &RepoEntry) -> Option<String> {
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

/// Label view: one row per label, carrying the tagged repos and sessions as child rows.
pub(crate) fn label_rows(index: &RepoIndex) -> Vec<Row> {
    let mut by_label: BTreeMap<String, Vec<Row>> = BTreeMap::new();

    for entry in &index.repos {
        let meta = repo::store::load_repo_meta(&entry.id).unwrap_or_default();
        for label in &meta.labels {
            by_label
                .entry(label.clone())
                .or_default()
                .push(labeled_repo_row(entry));
        }
        if let Ok(tree) = session::store::load_sessions(&entry.id) {
            for s in &tree.sessions {
                for label in &s.labels {
                    by_label
                        .entry(label.clone())
                        .or_default()
                        .push(labeled_session_row(entry, s));
                }
            }
        }
    }

    by_label
        .into_iter()
        .map(|(label, mut children)| {
            for (i, child) in children.iter_mut().enumerate() {
                child.value = i.to_string();
            }
            Row {
                display: format!(
                    "{} {}",
                    label.bold().magenta(),
                    format!(
                        "({} item{})",
                        children.len(),
                        if children.len() == 1 { "" } else { "s" }
                    )
                    .dimmed()
                ),
                value: label,
                kind: RowKind::Label { children },
            }
        })
        .collect()
}

fn labeled_repo_row(r: &RepoEntry) -> Row {
    Row {
        display: format!(
            "{} {} {}",
            "●".green(),
            r.name.bold().green(),
            paths::collapse_tilde(&r.path.to_string_lossy()).dimmed(),
        ),
        value: r.path.to_string_lossy().to_string(),
        kind: RowKind::Repo {
            path: r.path.clone(),
            entry: Some(r.clone()),
        },
    }
}

fn labeled_session_row(repo: &RepoEntry, s: &session::model::Session) -> Row {
    Row {
        display: format!(
            "{} {} {}{}",
            "◆".yellow(),
            s.name.bold().yellow(),
            format!("({})", repo.name).dimmed(),
            s.path
                .as_ref()
                .map(|p| format!(" → {}", p.display()).dimmed().to_string())
                .unwrap_or_default(),
        ),
        value: s.id.clone(),
        kind: RowKind::Session {
            repo: repo.clone(),
            session: Box::new(s.clone()),
            target_dir: s.path.clone().unwrap_or_else(|| repo.path.clone()),
        },
    }
}

/// Rows for items returned by a plugin view's `OnView` hook.
pub(crate) fn plugin_item_rows(items: &[crate::plugin::protocol::ViewItem]) -> Vec<Row> {
    items
        .iter()
        .map(|vi| Row {
            display: vi.display.clone(),
            value: vi.value.clone(),
            kind: RowKind::PluginItem,
        })
        .collect()
}
