pub mod model;
pub mod store;
pub mod tree;

use std::collections::HashMap;
use std::path::Path;

use chrono::Utc;
use colored::Colorize;
use uuid::Uuid;

use crate::cli::{SessionCommand, SessionLabelCommand};
use crate::error::{EzError, Result};
use crate::plugin;
use crate::repo;
use model::{Session, SessionTree};

/// Dispatch session subcommands.
pub fn dispatch(command: SessionCommand, cd_file: Option<&Path>) -> Result<()> {
    match command {
        SessionCommand::New { name, parent, repo } => {
            new_session(name.as_deref(), parent.as_deref(), repo.as_deref())
        }
        SessionCommand::List { repo, flat } => list_sessions(repo.as_deref(), flat),
        SessionCommand::Delete { name, repo, force } => {
            delete_session(&name, repo.as_deref(), force)
        }
        SessionCommand::Enter { name, repo } => {
            enter_session(&name, repo.as_deref(), cd_file)
        }
        SessionCommand::Exit => exit_session(),
        SessionCommand::Rename {
            name,
            new_name,
            repo,
        } => rename_session(&name, &new_name, repo.as_deref()),
        SessionCommand::Label { command } => dispatch_label(command),
    }
}

fn dispatch_label(cmd: SessionLabelCommand) -> Result<()> {
    match cmd {
        SessionLabelCommand::Add { name, labels, repo } => {
            let repo_entry = repo::resolve_repo(repo.as_deref())?;
            let session_id = find_session_id(&repo_entry.id, &name)?;
            let changed = set_session_labels(&repo_entry.id, &session_id, &labels, &[])?;
            println!(
                "{} {} {}",
                "Labels on session".green(),
                name.bold(),
                format_label_change(&changed)
            );
            Ok(())
        }
        SessionLabelCommand::Remove { name, labels, repo } => {
            let repo_entry = repo::resolve_repo(repo.as_deref())?;
            let session_id = find_session_id(&repo_entry.id, &name)?;
            let changed = set_session_labels(&repo_entry.id, &session_id, &[], &labels)?;
            println!(
                "{} {} {}",
                "Labels on session".green(),
                name.bold(),
                format_label_change(&changed)
            );
            Ok(())
        }
        SessionLabelCommand::List { name, repo } => {
            let repo_entry = repo::resolve_repo(repo.as_deref())?;
            let tree = store::load_sessions(&repo_entry.id)?;
            match name {
                Some(n) => {
                    let session = tree
                        .find_by_name(&n)
                        .ok_or_else(|| EzError::SessionNotFound(n.clone()))?;
                    if session.labels.is_empty() {
                        println!("{}", "(no labels)".dimmed());
                    } else {
                        for label in &session.labels {
                            println!("{}", label.magenta());
                        }
                    }
                }
                None => {
                    use std::collections::BTreeMap;
                    let mut by_label: BTreeMap<String, Vec<String>> = BTreeMap::new();
                    for session in &tree.sessions {
                        for label in &session.labels {
                            by_label
                                .entry(label.clone())
                                .or_default()
                                .push(session.name.clone());
                        }
                    }
                    if by_label.is_empty() {
                        println!("{}", "No session labels set.".dimmed());
                        return Ok(());
                    }
                    for (label, sessions) in by_label {
                        println!("{}", label.bold().magenta());
                        for s in sessions {
                            println!("  {}", s.yellow());
                        }
                    }
                }
            }
            Ok(())
        }
    }
}

fn find_session_id(repo_id: &str, name: &str) -> Result<String> {
    let tree = store::load_sessions(repo_id)?;
    tree.find_by_name(name)
        .map(|s| s.id.clone())
        .ok_or_else(|| EzError::SessionNotFound(name.into()))
}

/// Apply add/remove label mutations to a session. Returns the resulting label set.
pub fn set_session_labels(
    repo_id: &str,
    session_id: &str,
    add: &[String],
    remove: &[String],
) -> Result<Vec<String>> {
    let mut tree = store::load_sessions(repo_id)?;
    let session = tree
        .sessions
        .iter_mut()
        .find(|s| s.id == session_id)
        .ok_or_else(|| EzError::SessionNotFound(session_id.into()))?;

    let mut labels: std::collections::BTreeSet<String> =
        std::mem::take(&mut session.labels).into_iter().collect();
    for l in remove {
        labels.remove(l.as_str());
    }
    for l in add {
        if !l.trim().is_empty() {
            labels.insert(l.trim().to_string());
        }
    }
    let sorted: Vec<String> = labels.into_iter().collect();
    session.labels = sorted.clone();
    store::save_sessions(repo_id, &tree)?;
    Ok(sorted)
}

fn format_label_change(labels: &[String]) -> String {
    if labels.is_empty() {
        "→ (none)".dimmed().to_string()
    } else {
        format!("→ {}", labels.join(", ").magenta())
    }
}

fn new_session(name: Option<&str>, parent: Option<&str>, repo_arg: Option<&str>) -> Result<()> {
    let repo_entry = repo::resolve_repo(repo_arg)?;
    let mut tree = store::load_sessions(&repo_entry.id)?;

    let session_name = name
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("session-{}", &Uuid::new_v4().to_string()[..8]));

    let parent_id = if let Some(parent_name) = parent {
        let parent_session = tree
            .find_by_name(parent_name)
            .ok_or_else(|| EzError::SessionNotFound(parent_name.into()))?;
        Some(parent_session.id.clone())
    } else {
        None
    };

    let session = Session {
        id: Uuid::new_v4().to_string(),
        name: session_name.clone(),
        parent_id,
        path: None,
        env: HashMap::new(),
        plugin_state: HashMap::new(),
        labels: Vec::new(),
        created_at: Utc::now(),
        is_default: false,
    };

    tree.add(session.clone())?;

    // Run plugin hooks
    let config = crate::config::load()?;
    let repo_meta = repo::store::load_repo_meta(&repo_entry.id)?;
    plugin::run_hooks(
        plugin::model::HookType::OnSessionCreate,
        &repo_entry,
        &repo_meta,
        Some(&session),
        &config,
        &mut tree,
    )?;

    store::save_sessions(&repo_entry.id, &tree)?;
    println!("{} {}", "Created session:".green(), session_name.bold());
    Ok(())
}

fn list_sessions(repo_arg: Option<&str>, flat: bool) -> Result<()> {
    let repo_entry = repo::resolve_repo(repo_arg)?;
    let tree = store::load_sessions(&repo_entry.id)?;

    if tree.sessions.is_empty() {
        println!("{}", format!("No sessions for {}. Use `ez session new` to create one.", repo_entry.name).yellow());
        return Ok(());
    }

    if flat {
        for session in &tree.sessions {
            let default_marker = if session.is_default { " *".yellow().to_string() } else { String::new() };
            let path_info = session
                .path
                .as_ref()
                .map(|p| format!(" ({})", p.display()).dimmed().to_string())
                .unwrap_or_default();
            println!("{}{}{}", session.name.bold().yellow(), default_marker, path_info);
        }
    } else {
        let rendered = tree.render_tree();
        for (depth, session) in rendered {
            let indent = "  ".repeat(depth);
            let default_marker = if session.is_default { " *".yellow().to_string() } else { String::new() };
            let path_info = session
                .path
                .as_ref()
                .map(|p| format!(" ({})", p.display()).dimmed().to_string())
                .unwrap_or_default();
            println!("{}{}{}{}", indent, session.name.bold().yellow(), default_marker, path_info);
        }
    }
    Ok(())
}

fn delete_session(name: &str, repo_arg: Option<&str>, force: bool) -> Result<()> {
    let repo_entry = repo::resolve_repo(repo_arg)?;
    let mut tree = store::load_sessions(&repo_entry.id)?;

    let session = tree
        .find_by_name(name)
        .ok_or_else(|| EzError::SessionNotFound(name.into()))?
        .clone();

    // Check for children
    let children = tree.descendants(&session.id);
    if !children.is_empty() && !force {
        let child_names: Vec<String> = children.iter().map(|c| c.name.clone()).collect();
        return Err(EzError::SessionHasChildren {
            name: name.into(),
            children: child_names,
        });
    }

    // Delete children first (bottom-up)
    let config = crate::config::load()?;
    let repo_meta = repo::store::load_repo_meta(&repo_entry.id)?;

    // Collect descendant IDs in reverse order (deepest first)
    let descendant_ids: Vec<String> = {
        let descs = tree.descendants(&session.id);
        descs.iter().rev().map(|s| s.id.clone()).collect()
    };

    for desc_id in &descendant_ids {
        let desc = tree.find_by_id(desc_id).cloned();
        if let Some(desc_session) = desc {
            plugin::run_hooks(
                plugin::model::HookType::OnSessionDelete,
                &repo_entry,
                &repo_meta,
                Some(&desc_session),
                &config,
                &mut tree,
            )?;
            tree.remove(desc_id)?;
        }
    }

    // Delete the session itself
    plugin::run_hooks(
        plugin::model::HookType::OnSessionDelete,
        &repo_entry,
        &repo_meta,
        Some(&session),
        &config,
        &mut tree,
    )?;
    tree.remove(&session.id)?;

    store::save_sessions(&repo_entry.id, &tree)?;
    println!("{} {}", "Deleted session:".green(), name.bold());
    Ok(())
}

fn enter_session(name: &str, repo_arg: Option<&str>, cd_file: Option<&Path>) -> Result<()> {
    let repo_entry = repo::resolve_repo(repo_arg)?;
    let mut tree = store::load_sessions(&repo_entry.id)?;

    let session = tree
        .find_by_name(name)
        .ok_or_else(|| EzError::SessionNotFound(name.into()))?
        .clone();

    let config = crate::config::load()?;
    let repo_meta = repo::store::load_repo_meta(&repo_entry.id)?;

    plugin::run_hooks(
        plugin::model::HookType::OnSessionEnter,
        &repo_entry,
        &repo_meta,
        Some(&session),
        &config,
        &mut tree,
    )?;

    store::save_sessions(&repo_entry.id, &tree)?;

    // Determine the target directory
    let target_dir = session
        .path
        .as_ref()
        .cloned()
        .unwrap_or_else(|| repo_entry.path.clone());

    if let Some(cd_path) = cd_file {
        std::fs::write(cd_path, target_dir.to_string_lossy().as_bytes())?;
    } else {
        println!("{}", target_dir.display());
    }

    Ok(())
}

fn exit_session() -> Result<()> {
    // For now, exit is a no-op beyond plugin hooks
    // In the future, this could track which session is active
    println!("{}", "Exited session.".green());
    Ok(())
}

fn rename_session(name: &str, new_name: &str, repo_arg: Option<&str>) -> Result<()> {
    let repo_entry = repo::resolve_repo(repo_arg)?;
    let mut tree = store::load_sessions(&repo_entry.id)?;

    // Check new name doesn't conflict
    if tree.find_by_name(new_name).is_some() {
        return Err(EzError::SessionAlreadyExists(new_name.into()));
    }

    let session = tree
        .sessions
        .iter_mut()
        .find(|s| s.name == name)
        .ok_or_else(|| EzError::SessionNotFound(name.into()))?;

    session.name = new_name.to_string();

    store::save_sessions(&repo_entry.id, &tree)?;
    println!("{} {} -> {}", "Renamed session:".green(), name.bold(), new_name.bold());
    Ok(())
}

/// Create a child session under a given parent (by ID). Used by the browser action menu.
pub fn create_child_session(repo_id: &str, parent_id: &str, name: &str) -> Result<()> {
    let repo_entry = repo::store::load_index()?
        .repos
        .iter()
        .find(|r| r.id == repo_id)
        .cloned()
        .ok_or_else(|| EzError::RepoNotFound(repo_id.into()))?;

    let mut tree = store::load_sessions(repo_id)?;

    if tree.find_by_name(name).is_some() {
        return Err(EzError::SessionAlreadyExists(name.into()));
    }

    let session = Session {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        parent_id: Some(parent_id.to_string()),
        path: None,
        env: HashMap::new(),
        plugin_state: HashMap::new(),
        labels: Vec::new(),
        created_at: Utc::now(),
        is_default: false,
    };

    tree.add(session.clone())?;

    let config = crate::config::load()?;
    let repo_meta = repo::store::load_repo_meta(&repo_entry.id)?;
    plugin::run_hooks(
        plugin::model::HookType::OnSessionCreate,
        &repo_entry,
        &repo_meta,
        Some(&session),
        &config,
        &mut tree,
    )?;

    store::save_sessions(repo_id, &tree)?;
    Ok(())
}

/// Delete a session by ID (with forced cascade). Used by the browser action menu.
pub fn delete_session_by_id(repo_id: &str, session_id: &str, force: bool) -> Result<()> {
    let repo_entry = repo::store::load_index()?
        .repos
        .iter()
        .find(|r| r.id == repo_id)
        .cloned()
        .ok_or_else(|| EzError::RepoNotFound(repo_id.into()))?;

    let mut tree = store::load_sessions(repo_id)?;

    let sid = session_id.to_string();
    let session = tree
        .find_by_id(&sid)
        .cloned()
        .ok_or_else(|| EzError::SessionNotFound(session_id.into()))?;

    let children = tree.descendants(&session.id);
    if !children.is_empty() && !force {
        let child_names: Vec<String> = children.iter().map(|c| c.name.clone()).collect();
        return Err(EzError::SessionHasChildren {
            name: session.name.clone(),
            children: child_names,
        });
    }

    let config = crate::config::load()?;
    let repo_meta = repo::store::load_repo_meta(&repo_entry.id)?;

    let descendant_ids: Vec<String> = {
        let descs = tree.descendants(&session.id);
        descs.iter().rev().map(|s| s.id.clone()).collect()
    };

    for desc_id in &descendant_ids {
        let desc = tree.find_by_id(desc_id).cloned();
        if let Some(desc_session) = desc {
            plugin::run_hooks(
                plugin::model::HookType::OnSessionDelete,
                &repo_entry,
                &repo_meta,
                Some(&desc_session),
                &config,
                &mut tree,
            )?;
            tree.remove(desc_id)?;
        }
    }

    plugin::run_hooks(
        plugin::model::HookType::OnSessionDelete,
        &repo_entry,
        &repo_meta,
        Some(&session),
        &config,
        &mut tree,
    )?;
    tree.remove(&session.id)?;

    store::save_sessions(repo_id, &tree)?;
    Ok(())
}

/// Rename a session by ID. Used by the browser action menu.
pub fn rename_session_by_id(repo_id: &str, session_id: &str, new_name: &str) -> Result<()> {
    let mut tree = store::load_sessions(repo_id)?;

    if tree.find_by_name(new_name).is_some() {
        return Err(EzError::SessionAlreadyExists(new_name.into()));
    }

    let session = tree
        .sessions
        .iter_mut()
        .find(|s| s.id == session_id)
        .ok_or_else(|| EzError::SessionNotFound(session_id.into()))?;

    session.name = new_name.to_string();

    store::save_sessions(repo_id, &tree)?;
    Ok(())
}

/// Ensure a repo has at least a default "main" session.
/// Creates one if none exist, pointing to the repo's working directory.
pub fn ensure_default_session(repo_id: &str, repo_path: &Path) -> Result<SessionTree> {
    let mut tree = store::load_sessions(repo_id)?;
    if tree.sessions.is_empty() {
        let session = Session {
            id: Uuid::new_v4().to_string(),
            name: "main".to_string(),
            parent_id: None,
            path: Some(repo_path.to_path_buf()),
            env: HashMap::new(),
            plugin_state: HashMap::new(),
            labels: Vec::new(),
            created_at: Utc::now(),
            is_default: true,
        };
        tree.add(session)?;
        store::save_sessions(repo_id, &tree)?;
    }
    Ok(tree)
}
