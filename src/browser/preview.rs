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
                println!("  {} #{} {}", "pr:".bold(), num.cyan(), status_colored);
                if let Some(url) = session.env.get("ez_pr_url") {
                    println!("  {}", url.dimmed());
                }
            }

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

    println!();
    preview_keybind_help();

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

    // ── Git Info ──
    preview_section("Git Info");
    let branch = get_branch(path).unwrap_or_else(|| "detached".into());
    println!("  {} {}", "branch:".bold(), branch.cyan());

    if let Some(remote) = git_cmd(path, &["remote", "get-url", "origin"]) {
        println!("  {} {}", "remote:".bold(), remote.dimmed());
    }

    let dirty_count = git_cmd(path, &["status", "--porcelain"])
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

    if let Some(tags) = git_cmd(path, &["tag", "--sort=-creatordate"]) {
        let tag_list: Vec<&str> = tags.lines().take(3).collect();
        if !tag_list.is_empty() {
            println!("  {}  {}", "tags:".bold(), tag_list.join(", ").magenta());
        }
    }

    if let Some(branches) = git_cmd(path, &["branch", "--list"]) {
        let count = branches.lines().count();
        println!("  {} {count}", "branches:".bold());
    }

    println!();

    // ── Recent Commits ──
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
}

fn preview_keybind_help() {
    let config = config::load().unwrap_or_default();
    let keybinds = &config.keybinds;

    let fmt_key = |k: &str| k.replace("alt-", "Alt-").replace("ctrl-", "Ctrl-");

    preview_section("Keybinds");
    println!("  {}  Enter session", "Enter".bold().green());
    println!(
        "  {}  New child session",
        fmt_key(&keybinds.new_session).bold().yellow()
    );
    println!(
        "  {}  Rename session",
        fmt_key(&keybinds.rename_session).bold().yellow()
    );
    println!(
        "  {}  Delete session",
        fmt_key(&keybinds.delete_session).bold().red()
    );
    println!(
        "  {}  Edit labels",
        fmt_key(&keybinds.edit_labels).bold().magenta()
    );
    println!(
        "  {}  Cd into session worktree",
        fmt_key(&keybinds.cd_session).bold().yellow()
    );
    for pb in plugin::collect_plugin_binds("session", &config).unwrap_or_default() {
        let desc = pb.description.as_deref().unwrap_or(&pb.label);
        println!("  {}  {}", fmt_key(&pb.key).bold().cyan(), desc);
    }
    println!("  {}  Go back", "Esc".bold().dimmed());
}

fn preview_section(title: &str) {
    let bar = "─".repeat(40);
    println!("{}", format!("┌{bar}").dimmed());
    println!("{} {}", "│".dimmed(), title.bold().cyan());
    println!("{}", format!("└{bar}").dimmed());
}
