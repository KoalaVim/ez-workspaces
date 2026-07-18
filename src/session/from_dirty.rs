use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use chrono::Utc;
use colored::Colorize;
use uuid::Uuid;

use super::model::Session;
use super::{current, git_output, handle_branch_conflict, store};
use crate::error::{EzError, Result};
use crate::plugin;
use crate::repo;

/// CLI entry point for `ez session from-dirty`.
pub fn session_from_dirty(
    name: &str,
    repo_arg: Option<&str>,
    parent_arg: Option<&str>,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
    on_create: Option<&str>,
) -> Result<()> {
    let created = session_from_dirty_inner(name, repo_arg, parent_arg)?;

    let mut config = crate::config::load()?;
    if let Some(v) = on_create {
        config.on_create = v.into();
    }

    if crate::browser::on_create_is_noop(&config.on_create) {
        println!("{} {}", "Created session from dirty:".green(), name.bold());
    } else {
        let repo_entry = repo::resolve_repo(repo_arg)?;
        let target_dir = created
            .path
            .as_ref()
            .cloned()
            .unwrap_or_else(|| repo_entry.path.clone());
        crate::browser::accept_session(
            &config.on_create,
            &repo_entry,
            &created,
            &target_dir,
            cd_file,
            post_cmd_file,
            &config,
        )?;
    }
    Ok(())
}

/// Create a new session by moving current uncommitted changes to a new worktree via git stash.
/// Returns the created session. Used by both the CLI subcommand and the browser keybind.
pub fn session_from_dirty_inner(
    name: &str,
    repo_arg: Option<&str>,
    parent_arg: Option<&str>,
) -> Result<Session> {
    let target = current::resolve_current_session(repo_arg)?;
    let repo_entry = target.repo_entry;
    let current_session = target.session;

    let worktree_path = current_session.path.as_ref().ok_or_else(|| {
        EzError::Config(
            "Cannot create session from dirty: current session has no worktree path (bare session)"
                .into(),
        )
    })?;

    log::debug!(
        "session_from_dirty: current session='{}' worktree='{}'",
        current_session.name,
        worktree_path.display()
    );

    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(worktree_path)
        .output()?;
    if !status_output.status.success() {
        let stderr = String::from_utf8_lossy(&status_output.stderr)
            .trim()
            .to_string();
        return Err(EzError::Git(format!(
            "git status --porcelain failed in {}: {stderr}",
            worktree_path.display()
        )));
    }
    let status = String::from_utf8_lossy(&status_output.stdout)
        .trim()
        .to_string();
    if status.is_empty() {
        return Err(EzError::Config("No uncommitted changes to move".into()));
    }

    let head_commit = git_output(worktree_path, &["rev-parse", "HEAD"])?;
    log::debug!("session_from_dirty: HEAD={}", head_commit);

    let stash_msg = format!("ez: from-dirty for {name}");
    log::debug!("session_from_dirty: stashing with message '{}'", stash_msg);
    let stash_result = Command::new("git")
        .args(["stash", "push", "--include-untracked", "-m", &stash_msg])
        .current_dir(worktree_path)
        .output()?;
    if !stash_result.status.success() {
        let stderr = String::from_utf8_lossy(&stash_result.stderr)
            .trim()
            .to_string();
        return Err(EzError::Git(format!("git stash push failed: {stderr}")));
    }
    log::debug!("session_from_dirty: stash push succeeded");

    let parent_id = if let Some(parent_name) = parent_arg {
        let tree = store::load_sessions(&repo_entry.id)?;
        let parent = tree
            .find_by_name(parent_name)
            .ok_or_else(|| EzError::SessionNotFound(parent_name.into()))?;
        parent.id.clone()
    } else {
        current_session.id.clone()
    };

    let create_result =
        create_child_session_with_start_point(&repo_entry.id, &parent_id, name, &head_commit);

    let created = match create_result {
        Ok(session) => session,
        Err(e) => {
            eprintln!(
                "{} {}",
                "Session creation failed, restoring stash...".yellow(),
                e
            );
            let pop_result = Command::new("git")
                .args(["stash", "pop"])
                .current_dir(worktree_path)
                .output();
            match pop_result {
                Ok(out) if out.status.success() => {
                    log::debug!("session_from_dirty: rollback stash pop succeeded");
                }
                _ => {
                    eprintln!(
                        "{}",
                        "Warning: failed to restore stash. Your changes are in `git stash list`."
                            .yellow()
                    );
                }
            }
            return Err(e);
        }
    };

    let new_worktree = match &created.path {
        Some(p) => p.clone(),
        None => {
            eprintln!(
                "{}",
                "Warning: new session has no worktree path; stash preserved in original worktree."
                    .yellow()
            );
            println!("{} {}", "Created session from dirty:".green(), name.bold());
            return Ok(created);
        }
    };

    log::debug!(
        "session_from_dirty: popping stash in new worktree '{}'",
        new_worktree.display()
    );
    let pop_result = Command::new("git")
        .args(["stash", "pop"])
        .current_dir(&new_worktree)
        .output();
    match pop_result {
        Ok(out) if out.status.success() => {
            log::debug!("session_from_dirty: stash pop in new worktree succeeded");
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            eprintln!(
                "{} {}",
                "Warning: stash pop had issues (stash preserved):".yellow(),
                stderr
            );
        }
        Err(e) => {
            eprintln!(
                "{} {}",
                "Warning: could not pop stash in new worktree (stash preserved):".yellow(),
                e
            );
        }
    }

    Ok(created)
}

/// Create a child session with a specific start_point for the worktree.
/// Sets `ez_start_point` in the session env before running hooks so the
/// git-worktree plugin uses it instead of resolving from parent/origin.
fn create_child_session_with_start_point(
    repo_id: &str,
    parent_id: &str,
    name: &str,
    start_point: &str,
) -> Result<Session> {
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

    let session_id = Uuid::new_v4().to_string();
    let mut env = HashMap::new();
    env.insert("ez_start_point".to_string(), start_point.to_string());

    let session = Session {
        id: session_id.clone(),
        name: name.to_string(),
        parent_id: Some(parent_id.to_string()),
        path: None,
        env,
        plugin_state: HashMap::new(),
        labels: Vec::new(),
        created_at: Utc::now(),
        is_default: false,
        bare: false,
        last_accessed: None,
    };

    handle_branch_conflict(&repo_entry.path, name)?;
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

    let created = tree.find_by_id(&session_id).cloned().unwrap_or(session);
    Ok(created)
}
