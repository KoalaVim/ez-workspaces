//! fzf front end for the workspace directory drill-down.

use std::path::{Path, PathBuf};

use colored::Colorize;

use crate::config;
use crate::error::Result;
use crate::repo;

use super::domain::rows::directory_rows;
use super::domain::{to_items, RowKind};
use super::git::BranchCache;
use super::selector::{ActionResult, InteractiveSelector};

/// Drill into directories until a git repo is found or user selects one.
/// Supports action keybinds (e.g. clone) via `select_with_actions`.
pub(crate) fn drill_into_directory(
    start: &Path,
    selector: &dyn InteractiveSelector,
    config: &config::model::EzConfig,
    branch_cache: &BranchCache,
) -> Result<Option<PathBuf>> {
    let mut current = start.to_path_buf();
    let mut history: Vec<PathBuf> = Vec::new();

    loop {
        if current.join(".git").exists() {
            return Ok(Some(current));
        }

        // Load once per level so registered-repo labels render consistently
        // with the Repo/Owner views.
        let index = repo::store::load_index().unwrap_or_default();
        let rows = directory_rows(&current, &index, branch_cache);

        if rows.is_empty() {
            println!("{} {}", "No subdirectories in".yellow(), current.display());
            if let Some(prev) = history.pop() {
                current = prev;
                continue;
            }
            return Ok(None);
        }

        let items = to_items(&rows);

        let ez_bin = std::env::current_exe().ok();
        let preview_cmd = ez_bin.map(|bin| format!("{} preview {{}}", bin.display()));

        let expect_keys = vec![config.keybinds.clone_repo.as_str()];

        let action = selector.select_with_actions(
            &items,
            &current.file_name().unwrap_or_default().to_string_lossy(),
            preview_cmd.as_deref(),
            &expect_keys,
            None,
        )?;

        match action {
            ActionResult::Select(idx) => {
                history.push(current.clone());
                current = match &rows[idx].kind {
                    RowKind::Directory(path) | RowKind::Repo { path, .. } => path.clone(),
                    _ => continue,
                };
            }
            ActionResult::Action(ref key, _) if key == &config.keybinds.clone_repo => {
                let url = selector.input("Git URL to clone", None)?;
                if url.trim().is_empty() {
                    continue;
                }
                let name = url
                    .trim()
                    .rsplit('/')
                    .next()
                    .unwrap_or("repo")
                    .trim_end_matches(".git");
                let target = current.join(name);
                log::debug!(
                    "drill_into_directory: cloning {} into {}",
                    url,
                    target.display()
                );
                match repo::clone_repo(url.trim(), Some(&target)) {
                    Ok(()) => {
                        let canonical = crate::paths::normalize(&target);
                        return Ok(Some(canonical));
                    }
                    Err(e) => {
                        eprintln!("{} {}", "Clone failed:".red(), e);
                        continue;
                    }
                }
            }
            ActionResult::Action(_, _) => {}
            ActionResult::Cancel => {
                if let Some(prev) = history.pop() {
                    current = prev;
                } else {
                    return Ok(None);
                }
            }
        }
    }
}

/// Find the configured workspace root matching `ws` (as written in the config,
/// tilde-collapsed, expanded, or just the directory name).
pub(crate) fn match_workspace_root(
    config: &config::model::EzConfig,
    ws_raw: &str,
) -> Option<PathBuf> {
    let ws = ws_raw.trim_end_matches('/');
    config
        .workspace_roots
        .iter()
        .find(|r| {
            let r_trimmed = r.trim_end_matches('/');
            let expanded = crate::paths::expand_tilde(r);
            let collapsed = crate::paths::collapse_tilde(&expanded.to_string_lossy());
            let collapsed_trimmed = collapsed.trim_end_matches('/');
            let expanded_trimmed = expanded.to_string_lossy().trim_end_matches('/').to_string();
            let dir_name = expanded
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            r_trimmed == ws || collapsed_trimmed == ws || expanded_trimmed == ws || dir_name == ws
        })
        .map(|r| crate::paths::expand_tilde(r))
}
