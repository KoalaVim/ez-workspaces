use std::fs;
use std::path::Path;

use colored::Colorize;

use crate::config;
use crate::error::Result;
use crate::repo;
use crate::session;

use super::{get_branch, git_cmd};

/// Preview handler for fzf (hidden `ez preview <path>` command).
pub fn preview(path: &Path, show_session_actions: bool) -> Result<()> {
    if !path.exists() {
        println!("{}", "Path does not exist".red().bold());
        println!("{}", path.display());
        return Ok(());
    }

    if path.join(".git").exists() {
        preview_repo(path, show_session_actions)?;
    } else {
        preview_directory(path);
    }

    Ok(())
}

fn preview_repo(path: &Path, show_actions: bool) -> Result<()> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    println!("{} {}", "■".green(), name.bold().green());
    println!("{}", path.display().to_string().dimmed());
    println!();

    // ── Sessions (first) ──
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
            for (depth, s) in rendered {
                let indent = "  ".repeat(depth + 1);
                let marker = if s.is_default {
                    " ★".yellow().to_string()
                } else {
                    String::new()
                };
                let path_info = s
                    .path
                    .as_ref()
                    .map(|p| format!(" → {}", p.display()).dimmed().to_string())
                    .unwrap_or_default();
                let labels = if s.labels.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", s.labels.join(",")).magenta().to_string()
                };
                println!("{}{}{}{}{}", indent, s.name.bold(), marker, labels, path_info);
            }
        }

        // Per-repo labels
        let meta = repo::store::load_repo_meta(&entry.id).unwrap_or_default();
        if !meta.labels.is_empty() {
            println!();
            preview_section("Repo Labels");
            println!("  {}", meta.labels.join(", ").magenta());
        }
    } else {
        println!("  {}", "(unregistered — select to register)".dimmed());
    }

    if show_actions {
        println!();
        preview_keybind_help();
    }

    println!();

    // ── Git Info ──
    preview_section("Git Info");
    let branch = get_branch(path).unwrap_or_else(|| "detached".into());
    println!("  {} {}", "branch:".bold(), branch.cyan());

    if let Some(remote) = git_cmd(path, &["remote", "get-url", "origin"]) {
        println!("  {} {}", "remote:".bold(), remote.dimmed());
    }

    let dirty = git_cmd(path, &["status", "--porcelain"])
        .map(|s| s.lines().count())
        .unwrap_or(0);
    if dirty > 0 {
        println!(
            "  {} {}",
            "status:".bold(),
            format!("{dirty} modified file(s)").yellow()
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
    if let Some(log) = git_cmd(path, &["log", "--oneline", "--decorate", "--no-color", "-8"]) {
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
                    dirs.push(format!("  {} {}/", "▸".blue(), entry_name));
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
    let keybinds = config::load().map(|c| c.keybinds).unwrap_or_default();

    let fmt_key = |k: &str| k.replace("alt-", "Alt-").replace("ctrl-", "Ctrl-");

    preview_section("Keybinds");
    println!("  {}  {}", "Enter".bold().green(), "Enter session");
    println!(
        "  {}  {}",
        fmt_key(&keybinds.new_session).bold().yellow(),
        "New child session"
    );
    println!(
        "  {}  {}",
        fmt_key(&keybinds.rename_session).bold().yellow(),
        "Rename session"
    );
    println!(
        "  {}  {}",
        fmt_key(&keybinds.delete_session).bold().red(),
        "Delete session"
    );
    println!(
        "  {}  {}",
        fmt_key(&keybinds.edit_labels).bold().magenta(),
        "Edit labels"
    );
    println!("  {}  {}", "Esc".bold().dimmed(), "Go back");
}

fn preview_section(title: &str) {
    let bar = "─".repeat(40);
    println!("{}", format!("┌{bar}").dimmed());
    println!("{} {}", "│".dimmed(), title.bold().cyan());
    println!("{}", format!("└{bar}").dimmed());
}
