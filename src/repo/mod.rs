pub mod model;
pub mod store;

use std::path::Path;

use chrono::Utc;
use colored::Colorize;

use crate::cli::{LabelCommand, RepoCommand};
use crate::error::{EzError, Result};
use crate::paths;
use model::{RepoEntry, RepoMeta};

/// Clone a git repo and register it.
pub fn clone_repo(url: &str, path: Option<&Path>) -> Result<()> {
    let target = if let Some(p) = path {
        p.to_path_buf()
    } else {
        // Derive directory name from URL
        let name = url
            .rsplit('/')
            .next()
            .unwrap_or("repo")
            .trim_end_matches(".git");
        std::env::current_dir()?.join(name)
    };

    println!("{} {url} into {}...", "Cloning".cyan(), target.display());
    let status = std::process::Command::new("git")
        .args(["clone", url])
        .arg(&target)
        .status()?;

    if !status.success() {
        return Err(EzError::Git(format!("git clone failed with status {status}")));
    }

    let canonical = std::fs::canonicalize(&target)?;
    register_repo(&canonical)?;

    // Detect remote URL and default branch
    let meta = detect_repo_meta(&canonical);
    let repo_id = paths::repo_id_from_path(&canonical);
    store::save_repo_meta(&repo_id, &meta)?;

    println!("{} {}", "Registered:".green(), canonical.display());
    Ok(())
}

/// Register an existing repo (default: current directory).
pub fn add_repo(path: Option<&Path>) -> Result<()> {
    let target = if let Some(p) = path {
        std::fs::canonicalize(p)?
    } else {
        std::env::current_dir()?
    };

    if !target.join(".git").exists() && !target.join(".git").is_file() {
        // Check if it's a worktree (has .git file instead of dir)
        return Err(EzError::Git(format!(
            "{} is not a git repository",
            target.display()
        )));
    }

    register_repo(&target)?;

    let meta = detect_repo_meta(&target);
    let repo_id = paths::repo_id_from_path(&target);
    store::save_repo_meta(&repo_id, &meta)?;

    println!("Registered: {}", target.display());
    Ok(())
}

fn register_repo(path: &Path) -> Result<()> {
    let mut index = store::load_index()?;

    if index.find_by_path(path).is_some() {
        return Err(EzError::RepoAlreadyRegistered(path.display().to_string()));
    }

    let id = paths::repo_id_from_path(path);
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let entry = RepoEntry {
        id,
        path: path.to_path_buf(),
        name,
        registered_at: Utc::now(),
    };

    index.repos.push(entry);
    store::save_index(&index)?;
    Ok(())
}

fn detect_repo_meta(path: &Path) -> RepoMeta {
    let remote_url = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(path)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        });

    let default_branch = std::process::Command::new("git")
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
        });

    RepoMeta {
        remote_url,
        default_branch,
        labels: Vec::new(),
        plugin_state: Default::default(),
    }
}

/// Dispatch repo subcommands.
pub fn dispatch(command: RepoCommand) -> Result<()> {
    match command {
        RepoCommand::List { label } => list_repos(label.as_deref()),
        RepoCommand::Remove { name, purge } => remove_repo(&name, purge),
        RepoCommand::Label { command } => dispatch_label(command),
    }
}

fn list_repos(label_filter: Option<&str>) -> Result<()> {
    let index = store::load_index()?;
    if index.repos.is_empty() {
        println!("{}", "No repositories registered. Use `ez add` or `ez clone` to get started.".yellow());
        return Ok(());
    }

    let mut shown = 0;
    for repo in &index.repos {
        if let Some(want) = label_filter {
            let meta = store::load_repo_meta(&repo.id).unwrap_or_default();
            if !meta.labels.iter().any(|l| l == want) {
                continue;
            }
        }
        let labels_display = if label_filter.is_none() {
            let meta = store::load_repo_meta(&repo.id).unwrap_or_default();
            if meta.labels.is_empty() {
                String::new()
            } else {
                format!("  {}", meta.labels.join(", ").magenta())
            }
        } else {
            String::new()
        };
        println!(
            "{:<25} {}{}",
            repo.name.cyan(),
            repo.path.display(),
            labels_display
        );
        shown += 1;
    }

    if shown == 0 {
        if let Some(want) = label_filter {
            println!("{} {}", "No repos with label".yellow(), want.bold());
        }
    }
    Ok(())
}

fn dispatch_label(cmd: LabelCommand) -> Result<()> {
    match cmd {
        LabelCommand::Add { target, labels } => {
            let entry = resolve_repo(Some(&target))?;
            let changed = set_repo_labels(&entry.id, &labels, &[])?;
            println!(
                "{} {} {}",
                "Labels on".green(),
                entry.name.bold(),
                format_label_change(&changed)
            );
            Ok(())
        }
        LabelCommand::Remove { target, labels } => {
            let entry = resolve_repo(Some(&target))?;
            let changed = set_repo_labels(&entry.id, &[], &labels)?;
            println!(
                "{} {} {}",
                "Labels on".green(),
                entry.name.bold(),
                format_label_change(&changed)
            );
            Ok(())
        }
        LabelCommand::List { target } => {
            let index = store::load_index()?;
            match target {
                Some(t) => {
                    let entry = index
                        .find_by_name_or_id(&t)
                        .cloned()
                        .ok_or_else(|| EzError::RepoNotFound(t.clone()))?;
                    let meta = store::load_repo_meta(&entry.id)?;
                    if meta.labels.is_empty() {
                        println!("{}", "(no labels)".dimmed());
                    } else {
                        for label in &meta.labels {
                            println!("{}", label.magenta());
                        }
                    }
                }
                None => {
                    use std::collections::BTreeMap;
                    let mut by_label: BTreeMap<String, Vec<String>> = BTreeMap::new();
                    for repo in &index.repos {
                        let meta = store::load_repo_meta(&repo.id).unwrap_or_default();
                        for label in meta.labels {
                            by_label.entry(label).or_default().push(repo.name.clone());
                        }
                    }
                    if by_label.is_empty() {
                        println!("{}", "No repo labels set.".dimmed());
                        return Ok(());
                    }
                    for (label, repos) in by_label {
                        println!("{}", label.bold().magenta());
                        for repo in repos {
                            println!("  {}", repo.cyan());
                        }
                    }
                }
            }
            Ok(())
        }
    }
}

/// Apply add/remove label mutations to a repo. Returns the resulting label set.
pub fn set_repo_labels(repo_id: &str, add: &[String], remove: &[String]) -> Result<Vec<String>> {
    let mut meta = store::load_repo_meta(repo_id)?;
    let mut labels: std::collections::BTreeSet<String> = meta.labels.into_iter().collect();
    for l in remove {
        labels.remove(l.as_str());
    }
    for l in add {
        if !l.trim().is_empty() {
            labels.insert(l.trim().to_string());
        }
    }
    let sorted: Vec<String> = labels.into_iter().collect();
    meta.labels = sorted.clone();
    store::save_repo_meta(repo_id, &meta)?;
    Ok(sorted)
}

fn format_label_change(labels: &[String]) -> String {
    if labels.is_empty() {
        "→ (none)".dimmed().to_string()
    } else {
        format!("→ {}", labels.join(", ").magenta())
    }
}

fn remove_repo(name: &str, purge: bool) -> Result<()> {
    let mut index = store::load_index()?;
    let entry = index
        .find_by_name_or_id(name)
        .ok_or_else(|| EzError::RepoNotFound(name.into()))?
        .clone();

    index.remove_by_id(&entry.id);
    store::save_index(&index)?;

    if purge {
        store::delete_repo_meta(&entry.id)?;
        println!("{} {}", "Purged metadata for:".yellow(), entry.name);
    }

    println!("{} {}", "Removed:".green(), entry.name);
    Ok(())
}

/// Resolve which repo the user means: explicit --repo flag, or detect from cwd.
pub fn resolve_repo(repo_arg: Option<&str>) -> Result<RepoEntry> {
    let index = store::load_index()?;

    if let Some(query) = repo_arg {
        return index
            .find_by_name_or_id(query)
            .cloned()
            .ok_or_else(|| EzError::RepoNotFound(query.into()));
    }

    // Try to detect from current directory
    let cwd = std::env::current_dir()?;
    for repo in &index.repos {
        if cwd.starts_with(&repo.path) {
            return Ok(repo.clone());
        }
    }

    Err(EzError::RepoNotFound(
        "Could not detect repo from current directory. Use --repo to specify.".into(),
    ))
}
