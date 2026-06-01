pub mod model;
pub mod name_builder;
pub mod store;
pub mod tree;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use chrono::Utc;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::cli::{SessionCommand, SessionLabelCommand};
use crate::error::{EzError, Result};
use crate::plugin;
use crate::repo;
use model::{Session, SessionTree};

/// Payload written to a temp file and consumed by the `reap-delete` subcommand.
#[derive(Serialize, Deserialize)]
struct ReapPayload {
    repo_id: String,
    sessions: Vec<Session>,
}

/// Dispatch session subcommands.
pub fn dispatch(
    command: SessionCommand,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
    on_enter: Option<&str>,
    on_create: Option<&str>,
) -> Result<()> {
    match command {
        SessionCommand::New { name, parent, repo } => {
            new_session(name.as_deref(), parent.as_deref(), repo.as_deref(), cd_file, post_cmd_file, on_create)
        }
        SessionCommand::List { repo, flat } => list_sessions(repo.as_deref(), flat),
        SessionCommand::Delete { name, repo, force } => {
            delete_session(&name, repo.as_deref(), force)
        }
        SessionCommand::Enter { name, repo } => {
            enter_session(&name, repo.as_deref(), cd_file, post_cmd_file, on_enter)
        }
        SessionCommand::Exit => exit_session(),
        SessionCommand::Rename {
            name,
            new_name,
            repo,
        } => rename_session(&name, &new_name, repo.as_deref()),
        SessionCommand::Label { command } => dispatch_label(command),
        SessionCommand::ReapDelete { payload } => reap_delete(&payload),
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

fn new_session(
    name: Option<&str>,
    parent: Option<&str>,
    repo_arg: Option<&str>,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
    on_create: Option<&str>,
) -> Result<()> {
    let repo_entry = repo::resolve_repo(repo_arg)?;
    let mut tree = store::load_sessions(&repo_entry.id)?;

    // If a name was provided on the CLI, use it verbatim. Otherwise, run the
    // configured staged-name prompt.
    let mut config = crate::config::load()?;
    if let Some(v) = on_create {
        config.on_create = v.into();
    }

    let session_name = match name {
        Some(s) => s.to_string(),
        None => name_builder::prompt_session_name_default(&config)?,
    };

    let parent_id = if let Some(parent_name) = parent {
        let parent_session = tree
            .find_by_name(parent_name)
            .ok_or_else(|| EzError::SessionNotFound(parent_name.into()))?;
        Some(parent_session.id.clone())
    } else {
        None
    };

    let session_id = Uuid::new_v4().to_string();
    let session = Session {
        id: session_id.clone(),
        name: session_name.clone(),
        parent_id,
        path: None,
        env: HashMap::new(),
        plugin_state: HashMap::new(),
        labels: Vec::new(),
        created_at: Utc::now(),
        is_default: false,
    };

    handle_branch_conflict(&repo_entry.path, &session_name)?;
    tree.add(session.clone())?;

    // Run plugin hooks
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

    if crate::browser::on_create_is_noop(&config.on_create) {
        println!("{} {}", "Created session:".green(), session_name.bold());
    } else {
        // Get post-hook session (path may have been set by a plugin such as git-worktree).
        let created = tree.find_by_id(&session_id).cloned().unwrap_or(session);
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

/// Returns the names of sessions in `to_reap` whose worktree has uncommitted changes.
/// Skips default/main sessions (no dedicated worktree) and paths that don't exist.
fn dirty_worktrees(to_reap: &[model::Session]) -> Vec<String> {
    to_reap
        .iter()
        .filter(|s| !s.is_default)
        .filter_map(|s| s.path.as_ref().map(|p| (s, p)))
        .filter(|(_, p)| p.exists() && crate::browser::is_dirty(p))
        .map(|(s, _)| s.name.clone())
        .collect()
}

/// Returns the names of sessions (target + descendants) that have uncommitted changes.
/// Used by the TUI to warn the user before performing a forced delete.
pub fn cascade_dirty(repo_id: &str, session_id: &str) -> Result<Vec<String>> {
    let tree = store::load_sessions(repo_id)?;
    let sid = session_id.to_string();
    let session = tree
        .find_by_id(&sid)
        .ok_or_else(|| EzError::SessionNotFound(session_id.into()))?;
    let mut to_reap: Vec<model::Session> =
        tree.descendants(&session.id).into_iter().cloned().collect();
    to_reap.push(session.clone());
    Ok(dirty_worktrees(&to_reap))
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

    // Snapshot sessions to reap: descendants deepest-first, then the session itself.
    let to_reap: Vec<Session> = {
        let descs = tree.descendants(&session.id);
        let mut v: Vec<Session> = descs.into_iter().rev().cloned().collect();
        v.push(session.clone());
        v
    };

    // Pre-flight: abort if any worktree in the cascade has uncommitted changes.
    if !force {
        let dirty = dirty_worktrees(&to_reap);
        if !dirty.is_empty() {
            return Err(EzError::SessionWorktreeDirty { dirty });
        }
    }

    // Persist the removal synchronously BEFORE running hooks.  A hook (e.g.
    // tmux kill-session) may destroy the controlling terminal and SIGHUP this
    // process; the record must already be gone before that can happen.
    for s in &to_reap {
        tree.remove(&s.id)?;
    }
    store::save_sessions(&repo_entry.id, &tree)?;
    println!("{} {}", "Deleted session:".green(), name.bold());

    // Run plugin teardown (worktree removal, tmux kill, …) in a detached
    // worker that outlives any terminal teardown triggered by the hooks.
    spawn_detached_reap(&repo_entry.id, &to_reap)?;

    Ok(())
}

fn enter_session(
    name: &str,
    repo_arg: Option<&str>,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
    on_enter: Option<&str>,
) -> Result<()> {
    let repo_entry = repo::resolve_repo(repo_arg)?;
    let mut tree = store::load_sessions(&repo_entry.id)?;

    let session = tree
        .find_by_name(name)
        .ok_or_else(|| EzError::SessionNotFound(name.into()))?
        .clone();

    let mut config = crate::config::load()?;
    if let Some(v) = on_enter {
        config.on_enter = v.into();
    }
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

    // Determine the target directory, then apply the on_enter action.
    let target_dir = session
        .path
        .as_ref()
        .cloned()
        .unwrap_or_else(|| repo_entry.path.clone());

    crate::browser::accept_session(
        &config.on_enter,
        &repo_entry,
        &session,
        &target_dir,
        cd_file,
        post_cmd_file,
        &config,
    )
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
/// Create a new child session and return the post-hook `Session` (which may have a
/// `path` set by plugins such as git-worktree).
pub fn create_child_session(repo_id: &str, parent_id: &str, name: &str) -> Result<Session> {
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
    let session = Session {
        id: session_id.clone(),
        name: name.to_string(),
        parent_id: Some(parent_id.to_string()),
        path: None,
        env: HashMap::new(),
        plugin_state: HashMap::new(),
        labels: Vec::new(),
        created_at: Utc::now(),
        is_default: false,
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

    // Return the post-hook session so callers can read the updated path.
    let created = tree
        .find_by_id(&session_id)
        .cloned()
        .unwrap_or(session);
    Ok(created)
}

/// When the new session's name matches an existing local branch, prompt the user to
/// choose between reusing the existing branch or recreating it from the updated base.
///
/// Must be called BEFORE `tree.add` so that a cancelled or failed prompt leaves no
/// orphan session record behind.
fn handle_branch_conflict(repo_path: &Path, name: &str) -> Result<()> {
    if !crate::browser::branch_exists(repo_path, name) {
        return Ok(());
    }
    let recreate = crate::browser::selector::confirm_prompt(
        &format!(
            "Branch '{name}' already exists.\n  \
             [N] use the existing branch  (default)\n  \
             [y] recreate from the latest base (origin/main or parent) — discards '{name}'\n\
             Recreate?"
        ),
        false,
    )?;
    if recreate && !crate::browser::git_run(repo_path, &["branch", "-D", name]) {
        return Err(EzError::Git(format!(
            "Cannot recreate branch '{name}': delete failed \
             (it may be checked out in another worktree). \
             Remove that session first, or reuse the branch."
        )));
    }
    Ok(())
}

/// Delete a session by ID (with forced cascade). Used by the browser action menu.
pub fn delete_session_by_id(repo_id: &str, session_id: &str, force: bool) -> Result<()> {
    // Verify the repo exists before doing anything.
    let _repo_entry = repo::store::load_index()?
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

    // Snapshot sessions to reap: descendants deepest-first, then the session itself.
    let to_reap: Vec<Session> = {
        let descs = tree.descendants(&session.id);
        let mut v: Vec<Session> = descs.into_iter().rev().cloned().collect();
        v.push(session.clone());
        v
    };

    // Pre-flight: abort if any worktree in the cascade has uncommitted changes.
    if !force {
        let dirty = dirty_worktrees(&to_reap);
        if !dirty.is_empty() {
            return Err(EzError::SessionWorktreeDirty { dirty });
        }
    }

    // Persist removal synchronously before any hook can tear down the terminal.
    for s in &to_reap {
        tree.remove(&s.id)?;
    }
    store::save_sessions(repo_id, &tree)?;

    // Run plugin teardown in a detached worker.
    spawn_detached_reap(repo_id, &to_reap)?;

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

/// Spawn a detached worker process to run the OnSessionDelete plugin hooks for
/// sessions that have already been removed from the store.
///
/// The worker runs in a new process session (via `setsid`) so it has no
/// controlling terminal.  When a hook tears down the terminal (e.g.
/// `tmux kill-session` destroys the pane we're in), the worker is unaffected
/// and runs to completion.  The foreground `ez` has already persisted the
/// store and printed its output before this returns, so there is no data race.
fn spawn_detached_reap(repo_id: &str, sessions: &[Session]) -> Result<()> {
    let payload = ReapPayload {
        repo_id: repo_id.to_string(),
        sessions: sessions.to_vec(),
    };
    let json = serde_json::to_string(&payload)
        .map_err(|e| EzError::Config(format!("reap payload serialize error: {e}")))?;

    // Use the ez pid as part of the name so concurrent deletes don't collide.
    let tmp_path: PathBuf =
        std::env::temp_dir().join(format!("ez-reap-{}.json", std::process::id()));
    std::fs::write(&tmp_path, &json)?;

    let exe = std::env::current_exe()
        .map_err(|e| EzError::Config(format!("cannot resolve current exe: {e}")))?;

    let mut cmd = Command::new(&exe);
    cmd.args(["session", "reap-delete", "--payload"])
        .arg(&tmp_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // setsid() puts the child in a new session with no controlling terminal,
        // making it immune to SIGHUP when the current terminal is torn down.
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }

    cmd.spawn()
        .map_err(|e| EzError::Config(format!("failed to spawn reap worker: {e}")))?;
    // Intentionally not awaited — returning immediately is the whole point.

    Ok(())
}

/// Entry point for the hidden `session reap-delete` subcommand.
///
/// Reads the session snapshot from the temp payload file, runs the
/// OnSessionDelete plugin hooks (worktree removal, tmux kill, …), then
/// deletes the file.  Never writes to `sessions.toml`.
fn reap_delete(payload_path: &Path) -> Result<()> {
    let json = std::fs::read_to_string(payload_path)?;
    // Remove the temp file right away so it doesn't linger on error paths.
    let _ = std::fs::remove_file(payload_path);

    let payload: ReapPayload = serde_json::from_str(&json)
        .map_err(|e| EzError::Config(format!("reap payload parse error: {e}")))?;

    let repo_entry = repo::store::load_index()?
        .repos
        .into_iter()
        .find(|r| r.id == payload.repo_id)
        .ok_or_else(|| EzError::RepoNotFound(payload.repo_id.clone()))?;

    let config = crate::config::load()?;
    let repo_meta = repo::store::load_repo_meta(&repo_entry.id)?;

    // Build a throwaway SessionTree from the snapshot so plugin::run_hooks
    // can resolve parent information.  This tree is never saved to disk.
    let mut tree = SessionTree {
        sessions: payload.sessions.clone(),
    };

    for session in &payload.sessions {
        // Swallow hook errors — the record is already removed; this is
        // best-effort external cleanup.
        let _ = plugin::run_hooks(
            plugin::model::HookType::OnSessionDelete,
            &repo_entry,
            &repo_meta,
            Some(session),
            &config,
            &mut tree,
        );
    }

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
