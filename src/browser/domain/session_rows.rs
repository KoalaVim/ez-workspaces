//! Row builder for the per-repo session list.

use std::collections::HashSet;

use colored::Colorize;

use crate::repo::model::RepoEntry;
use crate::session::tree::{format_session_tree_line, TreeNode};
use crate::session::WorktreeInfo;

use super::super::format::{format_branch_indicator, format_last_accessed, format_pr_indicator};
use super::super::git::BranchCache;
use super::{Row, RowKind};

/// Value of the non-selectable "Not Registered" separator row.
const HEADER_VALUE: &str = "__header__";
/// Prefix of the value of unmanaged worktree rows (followed by the path).
const UNMANAGED_PREFIX: &str = "__unmanaged__:";

/// Managed sessions (one row per `rendered` node, in order) followed by a
/// header and the unmanaged worktrees, if any.
pub(crate) fn session_rows(
    repo_entry: &RepoEntry,
    rendered: &[TreeNode<'_>],
    worktree_info: &WorktreeInfo,
    attached: &HashSet<String>,
    branch_cache: &BranchCache,
) -> Vec<Row> {
    let mut rows: Vec<Row> = rendered
        .iter()
        .map(|node| {
            let prefix = format_session_tree_line(node).dimmed().to_string();
            let marker = if node.session.is_default {
                " ★".yellow().to_string()
            } else {
                String::new()
            };
            let bare_indicator = if node.session.bare {
                " [bare]".dimmed().to_string()
            } else {
                String::new()
            };
            let labels = if node.session.labels.is_empty() {
                String::new()
            } else {
                format!(" [{}]", node.session.labels.join(","))
                    .magenta()
                    .to_string()
            };
            let session_path = node.session.path.as_deref().unwrap_or(&repo_entry.path);
            let branch = worktree_info
                .get_branch_for_path(session_path)
                .or_else(|| branch_cache.get_branch(session_path));
            log::debug!(
                "session_rows: session '{}' branch={:?}",
                node.session.name,
                branch
            );
            let branch_indicator = format_branch_indicator(branch.as_deref());
            let pr_indicator = format_pr_indicator(&node.session.env);
            let last_used = node
                .session
                .last_accessed
                .as_ref()
                .map(|ts| {
                    format!(" ({})", format_last_accessed(ts))
                        .dimmed()
                        .to_string()
                })
                .unwrap_or_default();
            let name_colored = if attached.contains(&node.session.id) {
                node.session.name.bold().blue()
            } else {
                node.session.name.bold().yellow()
            };
            Row {
                display: format!(
                    "{}{}{}{}{}{}{}{}",
                    prefix,
                    name_colored,
                    branch_indicator,
                    marker,
                    bare_indicator,
                    pr_indicator,
                    labels,
                    last_used
                ),
                value: node.session.id.clone(),
                kind: RowKind::Session {
                    repo: repo_entry.clone(),
                    session: Box::new(node.session.clone()),
                    target_dir: node
                        .session
                        .path
                        .as_ref()
                        .cloned()
                        .unwrap_or_else(|| repo_entry.path.clone()),
                },
            }
        })
        .collect();

    let unmanaged = &worktree_info.unmanaged;
    if !unmanaged.is_empty() {
        rows.push(Row {
            display: format!("{}", "── Not Registered ──────────────────────".dimmed()),
            value: HEADER_VALUE.to_string(),
            kind: RowKind::Header,
        });
        for wt in unmanaged {
            let name = wt.branch.as_deref().unwrap_or("(detached)");
            rows.push(Row {
                display: format!(
                    "  {} {}",
                    name.dimmed(),
                    format!("→ {}", wt.path.display()).dimmed()
                ),
                value: format!("{UNMANAGED_PREFIX}{}", wt.path.display()),
                kind: RowKind::UnmanagedWorktree {
                    path: wt.path.clone(),
                    branch: wt.branch.clone(),
                },
            });
        }
    }
    rows
}
