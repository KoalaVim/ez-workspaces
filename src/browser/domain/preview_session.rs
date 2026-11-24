//! Preview panes for a single session and for unregistered worktrees.

use std::fmt::Write;
use std::path::Path;

use colored::Colorize;

use crate::error::Result;
use crate::repo;
use crate::session;
use crate::session::notes;

use super::super::format::format_last_accessed;
use super::keybind_help::{preview_keybind_help, preview_section};
use super::preview_slow::{Sink, Slow};

pub(super) fn preview_session(out: &mut Sink, repo_path: &Path, session_id: &str) -> Result<()> {
    let index = repo::store::load_index()?;
    let entry = match index.find_by_path(repo_path) {
        Some(e) => e,
        None => {
            outln!(out, "{}", "Repo not registered".red());
            return Ok(());
        }
    };
    let tree = session::store::load_sessions(&entry.id)?;
    let session = match tree.sessions.iter().find(|s| s.id == session_id) {
        Some(s) => s,
        None => {
            outln!(out, "{}", "Session not found".red());
            return Ok(());
        }
    };

    outln!(out, "{} {}", "■".green(), session.name.bold().green());
    outln!(out);

    // ── Metadata ──
    preview_section(out, "Metadata");
    outln!(out, "  {} {}", "repo:".bold(), entry.name.cyan());
    if let Some(ref p) = session.path {
        outln!(
            out,
            "  {} {}",
            "path:".bold(),
            p.display().to_string().dimmed()
        );
    } else if session.bare {
        outln!(out, "  {} {}", "path:".bold(), "(bare session)".dimmed());
    }
    if let Some(ref ts) = session.last_accessed {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
            let local = dt.with_timezone(&chrono::Local);
            let formatted = local.format("%d/%m/%Y %H:%M").to_string();
            outln!(
                out,
                "  {} {} ({})",
                "last used:".bold(),
                formatted,
                format_last_accessed(ts).dimmed()
            );
        }
    }
    if !session.labels.is_empty() {
        outln!(
            out,
            "  {} {}",
            "labels:".bold(),
            session.labels.join(", ").magenta()
        );
    }

    // ── Git Info (for sessions with worktree) ──
    if let Some(ref worktree_path) = session.path {
        if worktree_path.exists() && entry.is_git {
            outln!(out);
            preview_section(out, "Git Info");
            out.slow(Slow::GitInfo(worktree_path.clone()));

            if session.env.contains_key("ez_pr_number") {
                let num = session.env.get("ez_pr_number").unwrap();
                let status = session
                    .env
                    .get("ez_pr_status")
                    .map(|s| s.as_str())
                    .unwrap_or("?");
                let status_colored = match status {
                    "open" => status.green().to_string(),
                    "merged" => status.magenta().to_string(),
                    "closed" => status.red().to_string(),
                    _ => status.to_string(),
                };
                let url_part = session
                    .env
                    .get("ez_pr_url")
                    .map(|u| format!(" {}", u.dimmed()))
                    .unwrap_or_default();
                outln!(
                    out,
                    "  {} #{} {}{}",
                    "pr:".bold(),
                    num.cyan(),
                    status_colored,
                    url_part
                );
            }
        }
    }

    outln!(out);
    preview_keybind_help(out);

    // Note section (bat-rendered README.md), after keybinds
    if notes::notes_readme_exists(&entry.id, session_id) {
        if let Ok(readme_path) = crate::paths::notes_readme(&entry.id, session_id) {
            out.slow(Slow::Note(readme_path));
        }
    }

    // Recent commits last
    if let Some(ref worktree_path) = session.path {
        if worktree_path.exists() && entry.is_git {
            outln!(out);
            preview_section(out, "Recent Commits");
            out.slow(Slow::Commits {
                path: worktree_path.clone(),
                none_text: false,
            });
        }
    }

    Ok(())
}

pub(super) fn preview_unmanaged_worktree(out: &mut Sink, path: &Path) -> Result<()> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    outln!(
        out,
        "{} {} {}",
        "■".dimmed(),
        name.bold(),
        "(not registered)".dimmed()
    );
    outln!(out, "{}", path.display().to_string().dimmed());
    outln!(out);

    if path.exists() {
        preview_section(out, "Git Info");
        out.slow(Slow::GitInfo(path.to_path_buf()));

        outln!(out);
        outln!(
            out,
            "  {}",
            "Press Enter to register as a session".bold().green()
        );

        outln!(out);
        preview_section(out, "Recent Commits");
        out.slow(Slow::Commits {
            path: path.to_path_buf(),
            none_text: false,
        });
    } else {
        outln!(out, "  {}", "Worktree path does not exist".red());
    }

    Ok(())
}
