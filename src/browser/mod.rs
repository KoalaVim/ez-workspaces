pub mod selector;

use std::fs;
use std::path::Path;

use colored::Colorize;

use crate::config;
use crate::error::Result;
use crate::paths;
use crate::repo;
use crate::session;
use selector::{FzfSelector, InteractiveSelector, SelectItem};

/// Main interactive browser entry point (bare `ez` command).
pub fn browse(cd_file: Option<&Path>, tree_mode: bool) -> Result<()> {
    let config = config::load()?;
    let selector = FzfSelector::new(config.selector.fzf_opts.clone())?;

    if config.workspace_roots.is_empty() {
        println!("{}", "No workspace roots configured.".yellow());
        println!("Add roots to your config with: {}", "ez config --edit".bold());
        println!("Example: workspace_roots = [\"~/workspace\"]");
        return Ok(());
    }

    if tree_mode {
        return browse_tree(&config, &selector, cd_file);
    }

    // Step 1: Select workspace root
    let root_items: Vec<SelectItem> = config
        .workspace_roots
        .iter()
        .map(|r| {
            let expanded = paths::expand_tilde(r);
            SelectItem {
                display: r.clone(),
                value: expanded.to_string_lossy().to_string(),
            }
        })
        .collect();

    let root_idx = match selector.select_one(&root_items, "workspace", None)? {
        Some(idx) => idx,
        None => return Ok(()), // User cancelled
    };

    let root_path = paths::expand_tilde(&root_items[root_idx].value);

    // Step 2: Drill into directories until a repo is selected
    let repo_path = drill_into_directory(&root_path, &selector)?;
    let repo_path = match repo_path {
        Some(p) => p,
        None => return Ok(()),
    };

    // Step 3: Ensure repo is registered
    let index = repo::store::load_index()?;
    let repo_entry = if let Some(entry) = index.find_by_path(&repo_path) {
        entry.clone()
    } else {
        // Auto-register
        repo::add_repo(Some(&repo_path))?;
        let index = repo::store::load_index()?;
        index
            .find_by_path(&repo_path)
            .cloned()
            .expect("just registered")
    };

    // Step 4: Ensure default session exists, show session tree
    let tree = session::ensure_default_session(&repo_entry.id, &repo_entry.path)?;

    let rendered = tree.render_tree();
    let session_items: Vec<SelectItem> = rendered
        .iter()
        .map(|(depth, s)| {
            let indent = "  ".repeat(*depth);
            let marker = if s.is_default { " *" } else { "" };
            let path_info = s
                .path
                .as_ref()
                .map(|p| format!(" ({})", p.display()))
                .unwrap_or_default();
            SelectItem {
                display: format!("{}{}{}{}", indent, s.name, marker, path_info),
                value: s.id.clone(),
            }
        })
        .collect();

    let session_idx = match selector.select_one(&session_items, "session", None)? {
        Some(idx) => idx,
        None => return Ok(()),
    };

    let selected_session = rendered[session_idx].1;

    // Step 5: Enter the session
    let target_dir = selected_session
        .path
        .as_ref()
        .cloned()
        .unwrap_or_else(|| repo_entry.path.clone());

    write_cd_target(cd_file, &target_dir)
}

/// Write the target directory for the shell wrapper to cd into.
fn write_cd_target(cd_file: Option<&Path>, target_dir: &Path) -> Result<()> {
    if let Some(cd_path) = cd_file {
        fs::write(cd_path, target_dir.to_string_lossy().as_bytes())?;
    } else {
        println!("{}", target_dir.display());
    }
    Ok(())
}

/// Tree browser: show all roots → repos → sessions in one unicode tree.
fn browse_tree(
    config: &config::model::EzConfig,
    selector: &dyn InteractiveSelector,
    cd_file: Option<&Path>,
) -> Result<()> {
    let index = repo::store::load_index()?;
    let mut nodes: Vec<(String, Option<std::path::PathBuf>, std::path::PathBuf)> = Vec::new();
    let num_roots = config.workspace_roots.len();

    for (root_i, root) in config.workspace_roots.iter().enumerate() {
        let root_path = paths::expand_tilde(root);
        let is_last_root = root_i == num_roots - 1;
        let root_connector = if is_last_root { "└── " } else { "├── " };
        let root_cont = if is_last_root { "    " } else { "│   " };

        // Root header (non-selectable)
        nodes.push((
            format!("{}{}", root_connector.dimmed(), root.bold().blue()),
            None,
            root_path.clone(),
        ));

        if !root_path.is_dir() {
            continue;
        }

        // Collect repos in this root
        let mut repos: Vec<(String, std::path::PathBuf)> = Vec::new();
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

            // Repo node (selectable)
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

            // Sessions for this repo
            if let Some(repo_entry) = index.find_by_path(repo_path) {
                if let Ok(tree) = session::store::load_sessions(&repo_entry.id) {
                    if !tree.sessions.is_empty() {
                        let rendered = tree.render_tree();
                        let num_sessions = rendered.len();
                        for (sess_i, (depth, s)) in rendered.iter().enumerate() {
                            let is_last_session = sess_i == num_sessions - 1;
                            let sess_connector = if is_last_session {
                                "└── "
                            } else {
                                "├── "
                            };
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
                                    s.name.bold().cyan(),
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
        return Ok(());
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

    let idx = match selector.select_one(&items, "ez tree", preview_cmd.as_deref())? {
        Some(idx) => idx,
        None => return Ok(()),
    };

    match &nodes[idx].1 {
        Some(target) => write_cd_target(cd_file, target),
        None => Ok(()), // Non-selectable root header
    }
}

/// Drill into directories until a git repo is found or user selects one.
fn drill_into_directory(
    start: &Path,
    selector: &dyn InteractiveSelector,
) -> Result<Option<std::path::PathBuf>> {
    let mut current = start.to_path_buf();
    let mut history: Vec<std::path::PathBuf> = Vec::new();

    loop {
        // Check if current directory is a repo
        if current.join(".git").exists() {
            return Ok(Some(current));
        }

        // List subdirectories
        let mut entries: Vec<(String, std::path::PathBuf)> = Vec::new();

        if let Ok(read_dir) = fs::read_dir(&current) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.is_dir() && !path.file_name().map_or(true, |n| n.to_string_lossy().starts_with('.')) {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();

                    // Annotate repos with branch info
                    let display = if path.join(".git").exists() {
                        let branch = get_branch(&path).unwrap_or_else(|| "?".into());
                        format!("{name} [{branch}]")
                    } else {
                        name
                    };

                    entries.push((display, path));
                }
            }
        }

        entries.sort_by(|a, b| a.0.cmp(&b.0));

        if entries.is_empty() {
            println!("{} {}", "No subdirectories in".yellow(), current.display());
            // Go back if we have history, otherwise return None
            if let Some(prev) = history.pop() {
                current = prev;
                continue;
            }
            return Ok(None);
        }

        let items: Vec<SelectItem> = entries
            .iter()
            .map(|(display, path)| SelectItem {
                display: display.clone(),
                value: path.to_string_lossy().to_string(),
            })
            .collect();

        // Use ez preview for fzf preview
        let ez_bin = std::env::current_exe().ok();
        let preview_cmd = ez_bin.map(|bin| format!("{} preview {{}}", bin.display()));

        let idx = match selector.select_one(
            &items,
            &current.file_name().unwrap_or_default().to_string_lossy(),
            preview_cmd.as_deref(),
        )? {
            Some(idx) => idx,
            None => {
                // Escape: go back to previous directory
                if let Some(prev) = history.pop() {
                    current = prev;
                    continue;
                }
                return Ok(None);
            }
        };

        history.push(current.clone());
        current = entries[idx].1.clone();
    }
}

/// Get the current branch of a git repo.
fn get_branch(path: &Path) -> Option<String> {
    std::process::Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .current_dir(path)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
}

/// Preview handler for fzf (hidden `ez preview <path>` command).
pub fn preview(path: &Path) -> Result<()> {
    if !path.exists() {
        println!("Path does not exist: {}", path.display());
        return Ok(());
    }

    if path.join(".git").exists() {
        // It's a repo — show sessions
        let index = repo::store::load_index()?;
        if let Some(entry) = index.find_by_path(path) {
            let tree = session::store::load_sessions(&entry.id)?;
            if tree.sessions.is_empty() {
                println!("Repository: {}", entry.name);
                println!("No sessions (will auto-create 'main')");
            } else {
                println!("Repository: {}", entry.name);
                println!("Sessions:");
                let rendered = tree.render_tree();
                for (depth, session) in rendered {
                    let indent = "  ".repeat(depth);
                    let marker = if session.is_default { " *" } else { "" };
                    println!("  {}{}{}", indent, session.name, marker);
                }
            }
        } else {
            println!("Repository (unregistered): {}", path.display());
            let branch = get_branch(path).unwrap_or_else(|| "?".into());
            println!("Branch: {branch}");
        }
    } else {
        // It's a directory — show contents
        println!("Directory: {}", path.display());
        if let Ok(entries) = fs::read_dir(path) {
            let mut dirs: Vec<String> = Vec::new();
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() && !p.file_name().map_or(true, |n| n.to_string_lossy().starts_with('.')) {
                    let name = p.file_name().unwrap().to_string_lossy().to_string();
                    if p.join(".git").exists() {
                        let branch = get_branch(&p).unwrap_or_else(|| "?".into());
                        dirs.push(format!("  {name} [{branch}]"));
                    } else {
                        dirs.push(format!("  {name}/"));
                    }
                }
            }
            dirs.sort();
            for d in dirs {
                println!("{d}");
            }
        }
    }

    Ok(())
}
