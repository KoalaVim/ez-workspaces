use std::fs;
use std::path::Path;

use colored::Colorize;

use crate::config;
use crate::error::Result;
use crate::plugin;
use crate::repo;
use crate::session;

use super::{format_last_accessed, format_pr_indicator, get_branch, git_cmd};

/// Preview handler for fzf (hidden `ez preview <path>` command).
pub fn preview(path: &Path, show_session_actions: bool, session_id: Option<&str>) -> Result<()> {
    if !path.exists() {
        println!("{}", "Path does not exist".red().bold());
        println!("{}", path.display());
        return Ok(());
    }

    if show_session_actions {
        if let Some(sid) = session_id {
            return preview_session(path, sid);
        }
    }

    if path.join(".git").exists() {
        preview_repo(path)?;
    } else {
        let index = repo::store::load_index()?;
        if index.find_by_path(path).is_some() {
            preview_non_git_repo(path)?;
        } else {
            preview_directory(path);
        }
    }

    Ok(())
}

fn preview_session(repo_path: &Path, session_id: &str) -> Result<()> {
    let index = repo::store::load_index()?;
    let entry = match index.find_by_path(repo_path) {
        Some(e) => e,
        None => {
            println!("{}", "Repo not registered".red());
            return Ok(());
        }
    };
    let tree = session::store::load_sessions(&entry.id)?;
    let session = match tree.sessions.iter().find(|s| s.id == session_id) {
        Some(s) => s,
        None => {
            println!("{}", "Session not found".red());
            return Ok(());
        }
    };

    println!("{} {}", "■".green(), session.name.bold().green());
    println!();

    // ── Metadata ──
    preview_section("Metadata");
    println!("  {} {}", "repo:".bold(), entry.name.cyan());
    if let Some(ref p) = session.path {
        println!("  {} {}", "path:".bold(), p.display().to_string().dimmed());
    } else if session.bare {
        println!("  {} {}", "path:".bold(), "(bare session)".dimmed());
    }
    if let Some(ref ts) = session.last_accessed {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
            let local = dt.with_timezone(&chrono::Local);
            let formatted = local.format("%d/%m/%Y %H:%M").to_string();
            println!(
                "  {} {} ({})",
                "last used:".bold(),
                formatted,
                format_last_accessed(ts).dimmed()
            );
        }
    }
    if !session.labels.is_empty() {
        println!(
            "  {} {}",
            "labels:".bold(),
            session.labels.join(", ").magenta()
        );
    }

    // ── Git Info (for sessions with worktree) ──
    if let Some(ref worktree_path) = session.path {
        if worktree_path.exists() && entry.is_git {
            println!();
            preview_section("Git Info");
            let branch = get_branch(worktree_path).unwrap_or_else(|| "detached".into());
            println!("  {} {}", "branch:".bold(), branch.cyan());

            let dirty_count = git_cmd(worktree_path, &["status", "--porcelain"])
                .map(|s| s.lines().count())
                .unwrap_or(0);
            if dirty_count > 0 {
                println!(
                    "  {} {}",
                    "status:".bold(),
                    format!("{dirty_count} modified file(s)").yellow()
                );
            } else {
                println!("  {} {}", "status:".bold(), "clean".green());
            }

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
                println!(
                    "  {} #{} {}{}",
                    "pr:".bold(),
                    num.cyan(),
                    status_colored,
                    url_part
                );
            }
        }
    }

    println!();
    preview_keybind_help();

    // Recent commits last, after keybinds
    if let Some(ref worktree_path) = session.path {
        if worktree_path.exists() && entry.is_git {
            println!();
            preview_section("Recent Commits");
            if let Some(log) = git_cmd(
                worktree_path,
                &["log", "--oneline", "--decorate", "--no-color", "-8"],
            ) {
                for line in log.lines() {
                    if let Some((hash, msg)) = line.split_once(' ') {
                        println!("  {} {}", hash.yellow(), msg);
                    } else {
                        println!("  {line}");
                    }
                }
            }
        }
    }

    Ok(())
}

fn preview_repo(path: &Path) -> Result<()> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    println!("{} {}", "■".green(), name.bold().green());
    println!("{}", path.display().to_string().dimmed());
    println!();

    // ── Sessions ──
    preview_section("Sessions");
    let index = repo::store::load_index()?;
    if let Some(entry) = index.find_by_path(path) {
        let tree = session::store::load_sessions(&entry.id)?;
        if tree.sessions.is_empty() {
            println!(
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
                let path_info = node
                    .session
                    .path
                    .as_ref()
                    .map(|p| format!(" → {}", p.display()).dimmed().to_string())
                    .unwrap_or_default();
                let labels = if node.session.labels.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", node.session.labels.join(","))
                        .magenta()
                        .to_string()
                };
                let pr_indicator = format_pr_indicator(&node.session.env);
                println!(
                    "{}{}{}{}{}{}{}",
                    indent,
                    prefix,
                    node.session.name.bold().yellow(),
                    marker,
                    pr_indicator,
                    labels,
                    path_info
                );
                if let Some(url) = node.session.env.get("ez_pr_url") {
                    println!("{}  {}", indent, url.dimmed());
                }
            }
        }

        let meta = repo::store::load_repo_meta(&entry.id).unwrap_or_default();
        if !meta.labels.is_empty() {
            println!();
            preview_section("Repo Labels");
            println!("  {}", meta.labels.join(", ").magenta());
        }
    } else {
        println!("  {}", "(unregistered — select to register)".dimmed());
    }

    println!();
    preview_main_keybind_help();

    // Recent commits last, after keybinds
    println!();
    preview_section("Recent Commits");
    if let Some(log) = git_cmd(
        path,
        &["log", "--oneline", "--decorate", "--no-color", "-8"],
    ) {
        for line in log.lines() {
            if let Some((hash, msg)) = line.split_once(' ') {
                println!("  {} {}", hash.yellow(), msg);
            } else {
                println!("  {line}");
            }
        }
    } else {
        println!("  {}", "no commits".dimmed());
    }

    Ok(())
}

fn preview_non_git_repo(path: &Path) -> Result<()> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    println!(
        "{} {} {}",
        "■".blue(),
        name.bold().blue(),
        "(non-git)".dimmed()
    );
    println!("{}", path.display().to_string().dimmed());
    println!();

    preview_section("Sessions");
    let index = repo::store::load_index()?;
    if let Some(entry) = index.find_by_path(path) {
        let tree = session::store::load_sessions(&entry.id)?;
        if tree.sessions.is_empty() {
            println!(
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
                println!(
                    "{}{}{}{}{}{}",
                    indent,
                    prefix,
                    node.session.name.bold().yellow(),
                    marker,
                    pr_indicator,
                    labels
                );
                if let Some(url) = node.session.env.get("ez_pr_url") {
                    println!("{}  {}", indent, url.dimmed());
                }
            }
        }

        let meta = repo::store::load_repo_meta(&entry.id).unwrap_or_default();
        if !meta.labels.is_empty() {
            println!();
            preview_section("Repo Labels");
            println!("  {}", meta.labels.join(", ").magenta());
        }
    }

    println!();

    // ── Directory Contents ──
    preview_section("Contents");
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
            println!("{item}");
        }

        if file_count > 0 {
            println!("  {} {file_count} file(s)", "…".dimmed());
        }

        if items.is_empty() && file_count == 0 {
            println!("  {}", "(empty)".dimmed());
        }
    }

    println!();
    preview_main_keybind_help();

    Ok(())
}

fn preview_directory(path: &Path) {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    println!("{} {}", "■".blue(), name.bold().blue());
    println!("{}", path.display().to_string().dimmed());
    println!();

    preview_section("Contents");

    if let Ok(entries) = fs::read_dir(path) {
        let mut repos: Vec<String> = Vec::new();
        let mut dirs: Vec<String> = Vec::new();
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
                if p.join(".git").exists() {
                    let branch = get_branch(&p).unwrap_or_else(|| "?".into());
                    repos.push(format!(
                        "  {} {} {}",
                        "●".green(),
                        entry_name.bold(),
                        format!("[{branch}]").cyan()
                    ));
                } else {
                    dirs.push(format!("  {} {}/", "▸".blue(), entry_name.bold().blue()));
                }
            } else {
                file_count += 1;
            }
        }

        repos.sort();
        dirs.sort();

        if !repos.is_empty() {
            println!(
                "  {}",
                format!("Repositories ({})", repos.len()).bold().green()
            );
            for r in &repos {
                println!("{r}");
            }
        }

        if !dirs.is_empty() {
            if !repos.is_empty() {
                println!();
            }
            println!(
                "  {}",
                format!("Directories ({})", dirs.len()).bold().blue()
            );
            for d in &dirs {
                println!("{d}");
            }
        }

        if file_count > 0 {
            println!();
            println!("  {} {file_count} file(s)", "…".dimmed());
        }

        if repos.is_empty() && dirs.is_empty() && file_count == 0 {
            println!("  {}", "(empty)".dimmed());
        }
    }

    println!();
    preview_main_keybind_help();
}

fn fmt_key(k: &str) -> String {
    k.replace("alt-", "Alt-")
        .replace("ctrl-", "Ctrl-")
        .replace("shift-", "Shift-")
}

fn ansi_pad(s: &str, width: usize) -> String {
    let visible = console::measure_text_width(s);
    if visible >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - visible))
    }
}

const KEYBIND_KEY_W: usize = 8;

fn center_in(text: &str, width: usize) -> String {
    let visible = console::measure_text_width(text);
    if visible >= width {
        return text.to_string();
    }
    let pad_left = (width - visible) / 2;
    format!("{}{}", " ".repeat(pad_left), text)
}

fn keybind_col_width(entries: &[(String, String)]) -> usize {
    let max_entry = entries
        .iter()
        .map(|(_k, v)| {
            // "  " + key padded to KEYBIND_KEY_W + " " + description
            2 + KEYBIND_KEY_W + 1 + console::measure_text_width(v)
        })
        .max()
        .unwrap_or(20);
    max_entry + 2 // breathing room
}

fn print_keybind_table(
    left_title: &str,
    right_title: &str,
    left: &[(String, String)],
    right: &[(String, String)],
) {
    let col_w = keybind_col_width(left);
    let sep = "│".dimmed();

    // Header
    let left_h = center_in(&left_title.bold().cyan().to_string(), col_w);
    let right_h = center_in(&right_title.bold().cyan().to_string(), col_w);
    println!("{}{sep}{}", ansi_pad(&left_h, col_w), right_h);
    println!(
        "{}",
        format!("{}┼{}", "─".repeat(col_w), "─".repeat(col_w)).dimmed()
    );

    // Rows
    let rows = left.len().max(right.len());
    for i in 0..rows {
        let left_cell = if i < left.len() {
            format!("  {} {}", ansi_pad(&left[i].0, KEYBIND_KEY_W), left[i].1)
        } else {
            String::new()
        };
        let right_cell = if i < right.len() {
            format!(" {} {}", ansi_pad(&right[i].0, KEYBIND_KEY_W), right[i].1)
        } else {
            String::new()
        };
        println!("{}{sep}{}", ansi_pad(&left_cell, col_w), right_cell);
    }
}

fn preview_keybind_help() {
    let config = config::load().unwrap_or_default();
    let keybinds = &config.keybinds;

    preview_section("Keybinds");

    let mut left: Vec<(String, String)> = vec![
        ("Enter".bold().green().to_string(), "Enter session".into()),
        (
            fmt_key(&keybinds.new_session).bold().yellow().to_string(),
            "New child session".into(),
        ),
        (
            fmt_key(&keybinds.new_bare_session)
                .bold()
                .yellow()
                .to_string(),
            "New bare session".into(),
        ),
        (
            fmt_key(&keybinds.session_from_dirty)
                .bold()
                .yellow()
                .to_string(),
            "From dirty".into(),
        ),
        (
            fmt_key(&keybinds.rename_session)
                .bold()
                .yellow()
                .to_string(),
            "Rename".into(),
        ),
        (
            fmt_key(&keybinds.delete_session).bold().red().to_string(),
            "Delete".into(),
        ),
        (
            fmt_key(&keybinds.edit_labels).bold().magenta().to_string(),
            "Edit labels".into(),
        ),
        (
            fmt_key(&keybinds.cd_session).bold().yellow().to_string(),
            "Cd into worktree".into(),
        ),
    ];
    for pb in plugin::collect_plugin_binds("session", &config).unwrap_or_default() {
        let desc = pb.description.as_deref().unwrap_or(&pb.label);
        left.push((fmt_key(&pb.key).bold().cyan().to_string(), desc.into()));
    }

    let mut right: Vec<(String, String)> = vec![
        (
            fmt_key(&keybinds.sort_toggle).bold().yellow().to_string(),
            "Toggle sort".into(),
        ),
        ("Esc".bold().dimmed().to_string(), "Go back".into()),
    ];
    for pv in plugin::collect_plugin_views("session", &config).unwrap_or_default() {
        right.push((
            fmt_key(&pv.key).bold().cyan().to_string(),
            format!("{} view", pv.label),
        ));
    }

    print_keybind_table("Session", "Menu", &left, &right);
}

fn preview_main_keybind_help() {
    let config = config::load().unwrap_or_default();
    let keybinds = &config.keybinds;

    preview_section("Keybinds");

    let left: Vec<(String, String)> = vec![
        ("Enter".bold().green().to_string(), "Open sessions".into()),
        (
            fmt_key(&keybinds.clone_repo).bold().yellow().to_string(),
            "Clone repo".into(),
        ),
        (
            fmt_key(&keybinds.edit_labels).bold().magenta().to_string(),
            "Edit labels".into(),
        ),
        (
            fmt_key(&keybinds.sort_toggle).bold().yellow().to_string(),
            "Toggle sort".into(),
        ),
        ("Esc".bold().dimmed().to_string(), "Quit".into()),
    ];

    let mut right: Vec<(String, String)> = vec![
        (
            fmt_key(&keybinds.view_tree).bold().yellow().to_string(),
            "Tree".into(),
        ),
        (
            fmt_key(&keybinds.view_workspace)
                .bold()
                .yellow()
                .to_string(),
            "Workspace".into(),
        ),
        (
            fmt_key(&keybinds.view_repo).bold().yellow().to_string(),
            "Repo".into(),
        ),
        (
            fmt_key(&keybinds.view_owner).bold().yellow().to_string(),
            "Owner".into(),
        ),
        (
            fmt_key(&keybinds.view_label).bold().yellow().to_string(),
            "Label".into(),
        ),
    ];
    for pv in plugin::collect_plugin_views("repo", &config).unwrap_or_default() {
        right.push((fmt_key(&pv.key).bold().cyan().to_string(), pv.label.clone()));
    }

    print_keybind_table("Repo", "Views", &left, &right);
}

fn preview_section(title: &str) {
    let bar = "─".repeat(40);
    println!("{}", format!("┌{bar}").dimmed());
    println!("{} {}", "│".dimmed(), title.bold().cyan());
    println!("{}", format!("└{bar}").dimmed());
}
