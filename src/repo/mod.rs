pub mod model;
pub mod store;

use std::path::Path;

use chrono::Utc;

use crate::cli::RepoCommand;
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

    println!("Cloning {url} into {}...", target.display());
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

    println!("Registered: {}", canonical.display());
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
        plugin_state: Default::default(),
    }
}

/// Dispatch repo subcommands.
pub fn dispatch(command: RepoCommand) -> Result<()> {
    match command {
        RepoCommand::List => list_repos(),
        RepoCommand::Remove { name, purge } => remove_repo(&name, purge),
    }
}

fn list_repos() -> Result<()> {
    let index = store::load_index()?;
    if index.repos.is_empty() {
        println!("No repositories registered. Use `ez add` or `ez clone` to get started.");
        return Ok(());
    }
    for repo in &index.repos {
        println!("{:<25} {}", repo.name, repo.path.display());
    }
    Ok(())
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
        println!("Purged metadata for: {}", entry.name);
    }

    println!("Removed: {}", entry.name);
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
