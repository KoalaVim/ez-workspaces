pub mod selector;

use std::fs;
use std::path::Path;

use colored::Colorize;

use crate::config;
use crate::error::Result;
use crate::paths;
use crate::repo;
use crate::session;
use selector::{ActionResult, FzfSelector, InteractiveSelector, SelectItem};

/// Main interactive browser entry point (bare `ez` command).
pub fn browse(cd_file: Option<&Path>, tree_mode: bool) -> Result<()> {
    let config = config::load()?;
    let selector = FzfSelector::new(&config.fzf)?;

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
                display: paths::collapse_tilde(&expanded.to_string_lossy()),
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

    // Step 4: Session selection with action keybinds
    session_action_loop(&repo_entry, &selector, cd_file)
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

/// Session selection loop with action keybinds.
fn session_action_loop(
    repo_entry: &repo::model::RepoEntry,
    selector: &dyn InteractiveSelector,
    cd_file: Option<&Path>,
) -> Result<()> {
    loop {
        let tree = session::ensure_default_session(&repo_entry.id, &repo_entry.path)?;
        let rendered = tree.render_tree();

        let session_items: Vec<SelectItem> = rendered
            .iter()
            .map(|(depth, s)| {
                let indent = "  ".repeat(*depth);
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
                SelectItem {
                    display: format!("{}{}{}{}", indent, s.name.bold(), marker, path_info),
                    value: s.id.clone(),
                }
            })
            .collect();

        let ez_bin = std::env::current_exe().ok();
        let repo_path_str = repo_entry.path.to_string_lossy();
        let preview_cmd = ez_bin.map(|bin| {
            format!(
                "{} preview --session-actions {}",
                bin.display(),
                repo_path_str
            )
        });

        let action = selector.select_with_actions(
            &session_items,
            &repo_entry.name,
            preview_cmd.as_deref(),
            &["alt-n", "alt-d", "alt-r"],
            None,
        )?;

        log::debug!("session_action_loop: action={:?}", match &action {
            ActionResult::Select(i) => format!("Select({})", i),
            ActionResult::Action(k, i) => format!("Action({}, {})", k, i),
            ActionResult::Cancel => "Cancel".to_string(),
        });

        match action {
            ActionResult::Select(idx) => {
                let selected = rendered[idx].1;
                let target_dir = selected
                    .path
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| repo_entry.path.clone());
                return write_cd_target(cd_file, &target_dir);
            }
            ActionResult::Action(key, idx) => {
                let selected = rendered[idx].1;
                match key.as_str() {
                    "alt-n" => {
                        // New child session under the selected one
                        let name = selector.input("Session name", None)?;
                        if !name.is_empty() {
                            session::create_child_session(
                                &repo_entry.id,
                                &selected.id,
                                &name,
                            )?;
                            eprintln!(
                                "{} {} → {}",
                                "Created:".green(),
                                name.bold(),
                                selected.name.dimmed()
                            );
                        }
                    }
                    "alt-d" => {
                        let msg = format!("Delete session '{}'?", selected.name);
                        if selector.confirm(&msg, false)? {
                            session::delete_session_by_id(
                                &repo_entry.id,
                                &selected.id,
                                true,
                            )?;
                            eprintln!(
                                "{} {}",
                                "Deleted:".green(),
                                selected.name.bold()
                            );
                        }
                    }
                    "alt-r" => {
                        let new_name = selector.input(
                            "New name",
                            Some(&selected.name),
                        )?;
                        if !new_name.is_empty() && new_name != selected.name {
                            session::rename_session_by_id(
                                &repo_entry.id,
                                &selected.id,
                                &new_name,
                            )?;
                            eprintln!(
                                "{} {} → {}",
                                "Renamed:".green(),
                                selected.name.bold(),
                                new_name.bold()
                            );
                        }
                    }
                    _ => {}
                }
                // Loop back to show updated session list
            }
            ActionResult::Cancel => return Ok(()),
        }
    }
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

        nodes.push((
            format!("{}{}", root_connector.dimmed(), root.bold().blue()),
            None,
            root_path.clone(),
        ));

        if !root_path.is_dir() {
            continue;
        }

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

            if let Some(repo_entry) = index.find_by_path(repo_path) {
                if let Ok(tree) = session::store::load_sessions(&repo_entry.id) {
                    if !tree.sessions.is_empty() {
                        let rendered = tree.render_tree();
                        let num_sessions = rendered.len();
                        for (sess_i, (depth, s)) in rendered.iter().enumerate() {
                            let is_last_session = sess_i == num_sessions - 1;
                            let sess_connector =
                                if is_last_session { "└── " } else { "├── " };
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
        None => Ok(()),
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
        if current.join(".git").exists() {
            return Ok(Some(current));
        }

        let mut entries: Vec<(String, std::path::PathBuf)> = Vec::new();

        if let Ok(read_dir) = fs::read_dir(&current) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.is_dir()
                    && !path
                        .file_name()
                        .map_or(true, |n| n.to_string_lossy().starts_with('.'))
                {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();

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

        let ez_bin = std::env::current_exe().ok();
        let preview_cmd = ez_bin.map(|bin| format!("{} preview {{}}", bin.display()));

        let idx = match selector.select_one(
            &items,
            &current.file_name().unwrap_or_default().to_string_lossy(),
            preview_cmd.as_deref(),
        )? {
            Some(idx) => idx,
            None => {
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

/// Run a git command and capture stdout.
fn git_cmd(path: &Path, args: &[&str]) -> Option<String> {
    std::process::Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s.is_empty() { None } else { Some(s) }
            } else {
                None
            }
        })
}

/// Get the current branch of a git repo.
fn get_branch(path: &Path) -> Option<String> {
    git_cmd(path, &["symbolic-ref", "--short", "HEAD"])
}

fn preview_section(title: &str) {
    let bar = "─".repeat(40);
    println!("{}", format!("┌{bar}").dimmed());
    println!("{} {}", "│".dimmed(), title.bold().cyan());
    println!("{}", format!("└{bar}").dimmed());
}

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
                println!("{}{}{}{}", indent, s.name.bold(), marker, path_info);
            }
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
    preview_section("Keybinds");
    println!(
        "  {}  {}",
        "Enter".bold().green(),
        "Enter session"
    );
    println!(
        "  {}  {}",
        "Alt-N".bold().yellow(),
        "New child session"
    );
    println!(
        "  {}  {}",
        "Alt-R".bold().yellow(),
        "Rename session"
    );
    println!(
        "  {}  {}",
        "Alt-D".bold().red(),
        "Delete session"
    );
    println!(
        "  {}  {}",
        "Esc".bold().dimmed(),
        "Go back"
    );
}
