//! Preview pane rendering (repo, directory, session, unmanaged worktree).
//!
//! Renderers write ANSI text into a `String`; the hidden `ez preview`
//! subcommand prints it, and the native TUI shows it in its preview pane.

use std::fmt::Write;
use std::fs;
use std::path::Path;

use colored::Colorize;

use crate::error::Result;
use crate::repo;
use crate::session;

use super::super::format::format_pr_indicator;
use super::keybind_help::{preview_main_keybind_help, preview_section};
use super::preview_session::{preview_session, preview_unmanaged_worktree};
use super::preview_slow::Sink;
pub use super::preview_slow::{Segment, Slow};

/// Render the preview for `path` (hidden `ez preview <path>` command) into a String.
///
/// Colors follow the global `colored` override, so callers that want ANSI
/// output regardless of the TTY must set it first.
pub fn preview(
    path: &Path,
    show_session_actions: bool,
    session_id: Option<&str>,
) -> Result<String> {
    let mut out = Sink::full();
    render(&mut out, path, show_session_actions, session_id)?;
    Ok(out.into_text())
}

/// Like [`preview`], but slow sections are returned as [`Segment::Slow`]
/// instead of being rendered, so the caller can load them in the background.
/// Concatenating the segments with each slow section rendered gives exactly
/// the output of [`preview`].
pub fn preview_segments(
    path: &Path,
    show_session_actions: bool,
    session_id: Option<&str>,
) -> Result<Vec<Segment>> {
    let mut out = Sink::deferred();
    render(&mut out, path, show_session_actions, session_id)?;
    Ok(out.into_segments())
}

fn render(
    out: &mut Sink,
    path: &Path,
    show_session_actions: bool,
    session_id: Option<&str>,
) -> Result<()> {
    if !path.exists() {
        outln!(out, "{}", "Path does not exist".red().bold());
        outln!(out, "{}", path.display());
        return Ok(());
    }

    if show_session_actions {
        if let Some(sid) = session_id {
            if sid == "__header__" {
                return Ok(());
            }
            if let Some(wt_path_str) = sid.strip_prefix("__unmanaged__:") {
                return preview_unmanaged_worktree(out, Path::new(wt_path_str));
            }
            return preview_session(out, path, sid);
        }
    }

    if path.join(".git").exists() {
        preview_repo(out, path)?;
    } else {
        let index = repo::store::load_index()?;
        if index.find_by_path(path).is_some() {
            preview_non_git_repo(out, path)?;
        } else {
            preview_directory(out, path);
        }
    }

    Ok(())
}

fn preview_repo(out: &mut Sink, path: &Path) -> Result<()> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    outln!(out, "{} {}", "■".green(), name.bold().green());
    outln!(out, "{}", path.display().to_string().dimmed());
    outln!(out);

    // ── Sessions ──
    preview_section(out, "Sessions");
    let index = repo::store::load_index()?;
    if let Some(entry) = index.find_by_path(path) {
        let tree = session::store::load_sessions(&entry.id)?;
        if tree.sessions.is_empty() {
            outln!(
                out,
                "  {} {}",
                "none".dimmed(),
                "(will auto-create 'main')".dimmed()
            );
        } else {
            let rendered = tree.render_tree();
            for node in &rendered {
                let prefix = session::tree::format_session_tree_line(node)
                    .dimmed()
                    .to_string();
                let indent = "  ";
                let marker = if node.session.is_default {
                    " ★".yellow().to_string()
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
                let pr_indicator = format_pr_indicator(&node.session.env);
                outln!(
                    out,
                    "{}{}{}{}{}{}",
                    indent,
                    prefix,
                    node.session.name.bold().yellow(),
                    marker,
                    pr_indicator,
                    labels
                );
                if let Some(url) = node.session.env.get("ez_pr_url") {
                    outln!(out, "{}  {}", indent, url.dimmed());
                }
            }
        }

        let meta = repo::store::load_repo_meta(&entry.id).unwrap_or_default();
        if !meta.labels.is_empty() {
            outln!(out);
            preview_section(out, "Repo Labels");
            outln!(out, "  {}", meta.labels.join(", ").magenta());
        }
    } else {
        outln!(out, "  {}", "(unregistered — select to register)".dimmed());
    }

    outln!(out);
    preview_main_keybind_help(out);

    // Recent commits last, after keybinds
    outln!(out);
    preview_section(out, "Recent Commits");
    out.slow(Slow::Commits {
        path: path.to_path_buf(),
        none_text: true,
    });

    Ok(())
}

fn preview_non_git_repo(out: &mut Sink, path: &Path) -> Result<()> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    outln!(
        out,
        "{} {} {}",
        "■".blue(),
        name.bold().blue(),
        "(non-git)".dimmed()
    );
    outln!(out, "{}", path.display().to_string().dimmed());
    outln!(out);

    preview_section(out, "Sessions");
    let index = repo::store::load_index()?;
    if let Some(entry) = index.find_by_path(path) {
        let tree = session::store::load_sessions(&entry.id)?;
        if tree.sessions.is_empty() {
            outln!(
                out,
                "  {} {}",
                "none".dimmed(),
                "(will auto-create 'main')".dimmed()
            );
        } else {
            let rendered = tree.render_tree();
            for node in &rendered {
                let prefix = session::tree::format_session_tree_line(node)
                    .dimmed()
                    .to_string();
                let indent = "  ";
                let marker = if node.session.is_default {
                    " ★".yellow().to_string()
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
                let pr_indicator = format_pr_indicator(&node.session.env);
                outln!(
                    out,
                    "{}{}{}{}{}{}",
                    indent,
                    prefix,
                    node.session.name.bold().yellow(),
                    marker,
                    pr_indicator,
                    labels
                );
                if let Some(url) = node.session.env.get("ez_pr_url") {
                    outln!(out, "{}  {}", indent, url.dimmed());
                }
            }
        }

        let meta = repo::store::load_repo_meta(&entry.id).unwrap_or_default();
        if !meta.labels.is_empty() {
            outln!(out);
            preview_section(out, "Repo Labels");
            outln!(out, "  {}", meta.labels.join(", ").magenta());
        }
    }

    outln!(out);

    // ── Directory Contents ──
    preview_section(out, "Contents");
    if let Ok(entries) = fs::read_dir(path) {
        let mut items: Vec<String> = Vec::new();
        let mut file_count: usize = 0;

        for entry in entries.flatten() {
            let p = entry.path();
            let entry_name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            if entry_name.starts_with('.') {
                continue;
            }

            if p.is_dir() {
                items.push(format!("  {} {}/", "▸".blue(), entry_name.bold().blue()));
            } else {
                file_count += 1;
            }
        }

        items.sort();
        for item in &items {
            outln!(out, "{item}");
        }

        if file_count > 0 {
            outln!(out, "  {} {file_count} file(s)", "…".dimmed());
        }

        if items.is_empty() && file_count == 0 {
            outln!(out, "  {}", "(empty)".dimmed());
        }
    }

    outln!(out);
    preview_main_keybind_help(out);

    Ok(())
}

fn preview_directory(out: &mut Sink, path: &Path) {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    outln!(out, "{} {}", "■".blue(), name.bold().blue());
    outln!(out, "{}", path.display().to_string().dimmed());
    outln!(out);

    preview_section(out, "Contents");
    out.slow(Slow::DirContents(path.to_path_buf()));

    outln!(out);
    preview_main_keybind_help(out);
}
