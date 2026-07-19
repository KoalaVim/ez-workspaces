pub mod current;
pub mod cursor;
pub mod from_dirty;
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
        SessionCommand::New {
            name,
            parent,
            repo,
            interactive,
            bare,
        } => new_session(
            name.as_deref(),
            parent.as_deref(),
            repo.as_deref(),
            cd_file,
            post_cmd_file,
            on_create,
            interactive,
            bare,
        ),
        SessionCommand::List { repo, flat, json } => list_sessions(repo.as_deref(), flat, json),
        SessionCommand::Register {
            path,
            name,
            parent,
            repo,
        } => register_existing_worktree(
            path.as_deref(),
            name.as_deref(),
            parent.as_deref(),
            repo.as_deref(),
        ),
        SessionCommand::Delete { name, repo, force } => {
            delete_session(name.as_deref(), repo.as_deref(), force)
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
        SessionCommand::FromDirty { name, repo, parent } => from_dirty::session_from_dirty(
            &name,
            repo.as_deref(),
            parent.as_deref(),
            cd_file,
            post_cmd_file,
            on_create,
        ),
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

#[allow(clippy::too_many_arguments)]
fn new_session(
    name: Option<&str>,
    parent: Option<&str>,
    repo_arg: Option<&str>,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
    on_create: Option<&str>,
    interactive: bool,
    bare: bool,
) -> Result<()> {
    let repo_entry = repo::resolve_repo(repo_arg)?;
    let mut tree = store::load_sessions(&repo_entry.id)?;

    // If a name was provided on the CLI, use it verbatim. Otherwise, run the
    // configured staged-name prompt.
    let mut config = crate::config::load()?;
    if let Some(v) = on_create {
        config.on_create = v.into();
    }

    let name_result = match name {
        Some(s) if !interactive => name_builder::NameResult {
            name: s.to_string(),
            pr_metadata: None,
        },
        _ => name_builder::prompt_session_name_default(&config)?,
    };
    let session_name = name_result.name;
    let pr_metadata = name_result.pr_metadata;

    let session_env = pr_metadata
        .as_ref()
        .map(|pr| pr.to_session_env())
        .unwrap_or_default();

    let parent_id = if let Some(parent_name) = parent {
        let parent_session = tree
            .find_by_name(parent_name)
            .ok_or_else(|| EzError::SessionNotFound(parent_name.into()))?;
        Some(parent_session.id.clone())
    } else {
        tree.find_default().map(|s| s.id.clone())
    };

    let session_id = Uuid::new_v4().to_string();
    let session = Session {
        id: session_id.clone(),
        name: session_name.clone(),
        parent_id,
        path: None,
        env: session_env,
        plugin_state: HashMap::new(),
        labels: Vec::new(),
        created_at: Utc::now(),
        is_default: false,
        bare,
        last_accessed: None,
    };

    if tree.find_by_name(&session_name).is_some() {
        return Err(EzError::SessionAlreadyExists(session_name));
    }

    let skip_hooks = bare || !repo_entry.is_git;
    if !skip_hooks {
        handle_branch_conflict(&repo_entry.path, &session_name)?;
    }
    tree.add(session.clone())?;

    if bare {
        log::debug!(
            "bare session '{}': skipping OnSessionCreate hooks",
            session_name
        );
    } else if repo_entry.is_git {
        let repo_meta = repo::store::load_repo_meta(&repo_entry.id)?;
        plugin::run_hooks(
            plugin::model::HookType::OnSessionCreate,
            &repo_entry,
            &repo_meta,
            Some(&session),
            &config,
            &mut tree,
        )?;
    } else {
        log::debug!("new_session: non-git repo, setting path to repo root and skipping hooks");
        if let Some(s) = tree.sessions.iter_mut().find(|s| s.id == session_id) {
            s.path = Some(repo_entry.path.clone());
        }
    }

    store::save_sessions(&repo_entry.id, &tree)?;

    let created = tree.find_by_id(&session_id).cloned().unwrap_or(session);

    if let Some(pr) = &pr_metadata {
        if let Some(path) = &created.path {
            crate::browser::pr_merge_base_reset(path, &pr.base_ref);
        }
    }

    if crate::browser::on_create_is_noop(&config.on_create) {
        let suffix = if bare { " (bare)" } else { "" };
        println!(
            "{} {}{}",
            "Created session:".green(),
            session_name.bold(),
            suffix.dimmed()
        );
    } else {
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

fn register_existing_worktree(
    path: Option<&Path>,
    name: Option<&str>,
    parent: Option<&str>,
    repo_arg: Option<&str>,
) -> Result<()> {
    let requested_path = match path {
        Some(path) if path.is_absolute() => path.to_path_buf(),
        Some(path) => std::env::current_dir()?.join(path),
        None => std::env::current_dir()?,
    };
    let worktree = detect_existing_worktree(&requested_path)?;
    let repo_entry = resolve_registered_repo_for_worktree(repo_arg, &worktree.main_repo_path)?;
    let session_name = match name {
        Some(name) if !name.trim().is_empty() => name.trim().to_string(),
        Some(_) => {
            return Err(EzError::Config(
                "session name cannot be empty when registering a worktree".into(),
            ));
        }
        None => worktree.branch.clone().ok_or_else(|| {
            EzError::Config(
                "could not detect a branch for this worktree; pass --name explicitly".into(),
            )
        })?,
    };

    let mut tree = store::load_sessions(&repo_entry.id)?;
    if tree.find_by_name(&session_name).is_some() {
        return Err(EzError::SessionAlreadyExists(session_name));
    }

    if let Some(existing) = find_session_by_path(&tree, &worktree.worktree_path) {
        return Err(EzError::Config(format!(
            "worktree '{}' is already registered as session '{}'",
            worktree.worktree_path.display(),
            existing.name
        )));
    }

    let parent_id = if let Some(parent_name) = parent {
        let parent_session = tree
            .find_by_name(parent_name)
            .ok_or_else(|| EzError::SessionNotFound(parent_name.into()))?;
        Some(parent_session.id.clone())
    } else {
        tree.find_default().map(|s| s.id.clone())
    };

    let mut plugin_state = HashMap::new();
    plugin_state.insert(
        "worktree_path".to_string(),
        toml::Value::String(worktree.worktree_path.display().to_string()),
    );
    if let Some(branch) = &worktree.branch {
        plugin_state.insert("branch".to_string(), toml::Value::String(branch.clone()));
    }

    let session = Session {
        id: Uuid::new_v4().to_string(),
        name: session_name.clone(),
        parent_id,
        path: Some(worktree.worktree_path.clone()),
        env: HashMap::new(),
        plugin_state,
        labels: Vec::new(),
        created_at: Utc::now(),
        is_default: false,
        bare: false,
        last_accessed: None,
    };

    tree.add(session)?;
    store::save_sessions(&repo_entry.id, &tree)?;

    println!(
        "{} {} {} {}",
        "Registered session:".green(),
        session_name.bold(),
        "->".dimmed(),
        worktree.worktree_path.display()
    );
    Ok(())
}

struct ExistingWorktree {
    worktree_path: PathBuf,
    main_repo_path: PathBuf,
    branch: Option<String>,
}

fn detect_existing_worktree(path: &Path) -> Result<ExistingWorktree> {
    if !path.exists() {
        return Err(EzError::Path(format!(
            "worktree path does not exist: {}",
            path.display()
        )));
    }

    let worktree_path = git_output(path, &["rev-parse", "--show-toplevel"])?;
    let worktree_path = PathBuf::from(worktree_path);
    let worktree_path = worktree_path.canonicalize()?;

    let common_dir = git_output(&worktree_path, &["rev-parse", "--git-common-dir"])?;
    let common_dir = PathBuf::from(common_dir);
    let common_dir = if common_dir.is_absolute() {
        common_dir
    } else {
        worktree_path.join(common_dir)
    };
    let common_dir = common_dir.canonicalize()?;
    let main_repo_path = common_dir
        .file_name()
        .filter(|name| *name == ".git")
        .and_then(|_| common_dir.parent())
        .ok_or_else(|| {
            EzError::Git(format!(
                "could not resolve main repo from git common dir: {}",
                common_dir.display()
            ))
        })?
        .canonicalize()?;

    let branch = git_output(
        &worktree_path,
        &["symbolic-ref", "--quiet", "--short", "HEAD"],
    )
    .ok()
    .filter(|branch| !branch.trim().is_empty());

    Ok(ExistingWorktree {
        worktree_path,
        main_repo_path,
        branch,
    })
}

fn resolve_registered_repo_for_worktree(
    repo_arg: Option<&str>,
    main_repo_path: &Path,
) -> Result<repo::model::RepoEntry> {
    if let Some(repo_arg) = repo_arg {
        let repo_entry = repo::resolve_repo(Some(repo_arg))?;
        let registered_path = repo_entry.path.canonicalize()?;
        if registered_path != main_repo_path {
            return Err(EzError::RepoNotFound(format!(
                "worktree belongs to '{}', but --repo resolved to '{}'",
                main_repo_path.display(),
                repo_entry.path.display()
            )));
        }
        return Ok(repo_entry);
    }

    let index = repo::store::load_index()?;
    index
        .repos
        .into_iter()
        .find(|repo| {
            repo.path
                .canonicalize()
                .map(|path| path == main_repo_path)
                .unwrap_or(false)
        })
        .ok_or_else(|| {
            EzError::RepoNotFound(format!(
                "{} (register the main repo with `ez add {}` first)",
                main_repo_path.display(),
                main_repo_path.display()
            ))
        })
}

fn find_session_by_path<'a>(tree: &'a SessionTree, worktree_path: &Path) -> Option<&'a Session> {
    tree.sessions.iter().find(|session| {
        session
            .path
            .as_deref()
            .and_then(|path| path.canonicalize().ok())
            .map(|path| path == worktree_path)
            .unwrap_or(false)
    })
}

pub(crate) fn git_output(path: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git").args(args).current_dir(path).output()?;
    if output.status.success() {
        let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !value.is_empty() {
            return Ok(value);
        }
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(EzError::Git(format!(
        "git {} failed in {}{}{}",
        args.join(" "),
        path.display(),
        if stderr.is_empty() { "" } else { ": " },
        stderr
    )))
}

fn list_sessions(repo_arg: Option<&str>, flat: bool, json: bool) -> Result<()> {
    let repo_entry = repo::resolve_repo(repo_arg)?;
    let tree = store::load_sessions(&repo_entry.id)?;

    if tree.sessions.is_empty() {
        if json {
            println!("[]");
        } else {
            println!(
                "{}",
                format!(
                    "No sessions for {}. Use `ez session new` to create one.",
                    repo_entry.name
                )
                .yellow()
            );
        }
        return Ok(());
    }

    if json {
        let items: Vec<serde_json::Value> = tree
            .sessions
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "name": s.name,
                    "parent_id": s.parent_id,
                    "path": s.path.as_ref().map(|p| p.display().to_string()),
                    "bare": s.bare,
                    "labels": s.labels,
                    "last_accessed": s.last_accessed,
                    "env": s.env,
                    "is_default": s.is_default,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&items).unwrap_or_else(|_| "[]".into())
        );
        return Ok(());
    }

    if flat {
        for session in &tree.sessions {
            let default_marker = if session.is_default {
                " *".yellow().to_string()
            } else {
                String::new()
            };
            let bare_indicator = if session.bare {
                " [bare]".dimmed().to_string()
            } else {
                String::new()
            };
            let path_info = session
                .path
                .as_ref()
                .map(|p| format!(" ({})", p.display()).dimmed().to_string())
                .unwrap_or_default();
            println!(
                "{}{}{}{}",
                session.name.bold().yellow(),
                default_marker,
                bare_indicator,
                path_info
            );
        }
    } else {
        let rendered = tree.render_tree();
        for node in &rendered {
            let prefix = tree::format_session_tree_line(node).dimmed().to_string();
            let default_marker = if node.session.is_default {
                " *".yellow().to_string()
            } else {
                String::new()
            };
            let path_info = node
                .session
                .path
                .as_ref()
                .map(|p| format!(" ({})", p.display()).dimmed().to_string())
                .unwrap_or_default();
            println!(
                "{}{}{}{}",
                prefix,
                node.session.name.bold().yellow(),
                default_marker,
                path_info
            );
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

fn delete_session(name: Option<&str>, repo_arg: Option<&str>, force: bool) -> Result<()> {
    let (repo_entry, session) = match name {
        Some(name) => {
            let repo_entry = repo::resolve_repo(repo_arg)?;
            let tree = store::load_sessions(&repo_entry.id)?;
            let session = tree
                .find_by_name(name)
                .ok_or_else(|| EzError::SessionNotFound(name.into()))?
                .clone();
            (repo_entry, session)
        }
        None => {
            let target = current::resolve_current_session(repo_arg)?;
            current::confirm_delete_current_session(&target)?;
            (target.repo_entry, target.session)
        }
    };
    let mut tree = store::load_sessions(&repo_entry.id)?;

    // Check for children
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

    // Persist the removal synchronously BEFORE running hooks.  A hook (e.g.
    // tmux kill-session) may destroy the controlling terminal and SIGHUP this
    // process; the record must already be gone before that can happen.
    for s in &to_reap {
        tree.remove(&s.id)?;
    }
    store::save_sessions(&repo_entry.id, &tree)?;
    println!("{} {}", "Deleted session:".green(), session.name.bold());

    // Run plugin teardown (worktree removal, tmux kill, …) in a detached
    // worker that outlives any terminal teardown triggered by the hooks.
    spawn_detached_reap(&repo_entry.id, &to_reap)?;

    Ok(())
}

/// Refresh the PR status for a session if it has PR metadata and the status
/// is stale (older than 5 minutes). Updates the session env in-place.
fn refresh_pr_status(tree: &mut SessionTree, session_id: &str) {
    let (pr_number, pr_url, needs_refresh) = {
        let session = match tree.sessions.iter().find(|s| s.id == session_id) {
            Some(s) => s,
            None => return,
        };
        let pr_number = match session.env.get("ez_pr_number") {
            Some(n) => n.clone(),
            None => return,
        };
        let pr_url = session.env.get("ez_pr_url").cloned();

        let needs_refresh = match session.env.get("ez_pr_status_updated") {
            Some(updated) => match chrono::DateTime::parse_from_rfc3339(updated) {
                Ok(dt) => Utc::now().signed_duration_since(dt).num_seconds() >= 300,
                Err(_) => true,
            },
            None => true,
        };
        (pr_number, pr_url, needs_refresh)
    };

    if !needs_refresh {
        log::debug!("refresh_pr_status: status for PR #{pr_number} is fresh, skipping");
        return;
    }

    if which::which("gh").is_err() {
        log::debug!("refresh_pr_status: gh not found, skipping");
        return;
    }

    log::debug!("refresh_pr_status: refreshing status for PR #{pr_number}");

    let arg = pr_url.as_deref().unwrap_or(&pr_number);
    let output = Command::new("gh")
        .args(["pr", "view", arg, "--json", "state"])
        .stderr(Stdio::null())
        .output();

    match output {
        Ok(o) if o.status.success() => {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&o.stdout) {
                if let Some(state) = json.get("state").and_then(|v| v.as_str()) {
                    let status = state.to_lowercase();
                    log::debug!("refresh_pr_status: PR #{pr_number} status={status}");
                    if let Some(s) = tree.sessions.iter_mut().find(|s| s.id == session_id) {
                        s.env.insert("ez_pr_status".into(), status);
                        s.env
                            .insert("ez_pr_status_updated".into(), Utc::now().to_rfc3339());
                    }
                }
            }
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            log::debug!("refresh_pr_status: gh pr view failed: {stderr}");
        }
        Err(e) => {
            log::debug!("refresh_pr_status: failed to run gh: {e}");
        }
    }
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

    refresh_pr_status(&mut tree, &session.id);

    let now = Utc::now().to_rfc3339();
    if let Some(s) = tree.sessions.iter_mut().find(|s| s.id == session.id) {
        s.last_accessed = Some(now.clone());
    }
    store::save_sessions(&repo_entry.id, &tree)?;

    let mut repo_meta = repo_meta;
    repo_meta.last_accessed = Some(now);
    repo::store::save_repo_meta(&repo_entry.id, &repo_meta)?;
    log::debug!(
        "enter_session: updated last_accessed for session '{}' and repo '{}'",
        session.name,
        repo_entry.id
    );

    if session.bare && config.on_enter == "cd" {
        println!(
            "{}",
            format!(
                "Session '{}' has no worktree path (bare session)",
                session.name
            )
            .yellow()
        );
        return Ok(());
    }

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

    if tree.find_by_name(new_name).is_some() {
        return Err(EzError::SessionAlreadyExists(new_name.into()));
    }

    let session = tree
        .sessions
        .iter_mut()
        .find(|s| s.name == name)
        .ok_or_else(|| EzError::SessionNotFound(name.into()))?;

    let old_name = session.name.clone();
    let rename_result =
        perform_session_rename(session, new_name, &repo_entry.path, repo_entry.is_git);

    store::save_sessions(&repo_entry.id, &tree)?;

    let config = crate::config::load()?;
    if config.copy_cursor_conversations {
        if let (Some(old_path), Some(new_path)) = (&rename_result.old_path, &rename_result.new_path)
        {
            cursor::copy_cursor_conversations(old_path, new_path);
        }
    }

    let repo_meta = repo::store::load_repo_meta(&repo_entry.id)?;
    run_rename_hooks(
        &repo_entry,
        &repo_meta,
        &tree,
        &old_name,
        new_name,
        &rename_result,
        &config,
    );

    println!(
        "{} {} -> {}",
        "Renamed session:".green(),
        old_name.bold(),
        new_name.bold()
    );
    Ok(())
}

/// Result of the physical rename operations (branch + worktree move).
struct RenameResult {
    old_path: Option<PathBuf>,
    new_path: Option<PathBuf>,
}

/// Shared helper: rename the git branch and move the worktree directory.
/// Updates `session.name` and `session.path` in place.
fn perform_session_rename(
    session: &mut Session,
    new_name: &str,
    repo_path: &Path,
    is_git: bool,
) -> RenameResult {
    let old_name = session.name.clone();
    let old_path = session.path.clone();
    session.name = new_name.to_string();

    if session.bare || !is_git {
        log::debug!(
            "perform_session_rename: skipping branch/worktree ops (bare={}, is_git={})",
            session.bare,
            is_git
        );
        return RenameResult {
            old_path: old_path.clone(),
            new_path: old_path,
        };
    }

    let worktree_path = match &session.path {
        Some(p) => p.clone(),
        None => {
            log::debug!("perform_session_rename: no session path, skipping git ops");
            return RenameResult {
                old_path: None,
                new_path: None,
            };
        }
    };

    // Rename the git branch
    let branch_result = Command::new("git")
        .args(["branch", "-m", &old_name, new_name])
        .current_dir(&worktree_path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output();

    match &branch_result {
        Ok(output) if output.status.success() => {
            log::debug!(
                "perform_session_rename: renamed branch '{}' -> '{}'",
                old_name,
                new_name
            );
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::debug!(
                "perform_session_rename: git branch -m failed: {}",
                stderr.trim()
            );
            eprintln!(
                "{}",
                format!("Warning: could not rename branch: {}", stderr.trim()).yellow()
            );
        }
        Err(e) => {
            log::debug!("perform_session_rename: git branch -m error: {}", e);
            eprintln!(
                "{}",
                format!("Warning: could not rename branch: {}", e).yellow()
            );
        }
    }

    // Move the worktree directory
    let new_worktree_path = worktree_path
        .parent()
        .map(|parent| parent.join(new_name))
        .unwrap_or_else(|| PathBuf::from(new_name));

    let move_result = Command::new("git")
        .args([
            "worktree",
            "move",
            &worktree_path.display().to_string(),
            &new_worktree_path.display().to_string(),
        ])
        .current_dir(repo_path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output();

    match &move_result {
        Ok(output) if output.status.success() => {
            log::debug!(
                "perform_session_rename: moved worktree '{}' -> '{}'",
                worktree_path.display(),
                new_worktree_path.display()
            );
            session.path = Some(new_worktree_path.clone());
            RenameResult {
                old_path: Some(worktree_path),
                new_path: Some(new_worktree_path),
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::debug!(
                "perform_session_rename: git worktree move failed: {}",
                stderr.trim()
            );
            eprintln!(
                "{}",
                format!("Warning: could not move worktree: {}", stderr.trim()).yellow()
            );
            RenameResult {
                old_path: Some(worktree_path),
                new_path: session.path.clone(),
            }
        }
        Err(e) => {
            log::debug!("perform_session_rename: git worktree move error: {}", e);
            eprintln!(
                "{}",
                format!("Warning: could not move worktree: {}", e).yellow()
            );
            RenameResult {
                old_path: Some(worktree_path),
                new_path: session.path.clone(),
            }
        }
    }
}

/// Fire OnSessionRename hooks (best-effort, errors are logged but not propagated).
fn run_rename_hooks(
    repo_entry: &crate::repo::model::RepoEntry,
    repo_meta: &crate::repo::model::RepoMeta,
    tree: &model::SessionTree,
    old_name: &str,
    new_name: &str,
    rename_result: &RenameResult,
    config: &crate::config::model::EzConfig,
) {
    let session = tree.find_by_name(new_name);
    let rename_context = plugin::protocol::RenameContext {
        old_name: old_name.to_string(),
        new_name: new_name.to_string(),
        old_path: rename_result
            .old_path
            .as_ref()
            .map(|p| p.display().to_string()),
        new_path: rename_result
            .new_path
            .as_ref()
            .map(|p| p.display().to_string()),
    };

    let mut tree_clone = tree.clone();
    if let Err(e) = plugin::run_hooks_with_rename(
        plugin::model::HookType::OnSessionRename,
        repo_entry,
        repo_meta,
        session,
        config,
        &mut tree_clone,
        Some(rename_context),
    ) {
        log::debug!("run_rename_hooks: hook error (swallowed): {}", e);
    }
}

/// Create a child session under a given parent (by ID). Used by the browser action menu.
/// Create a new child session and return the post-hook `Session` (which may have a
/// `path` set by plugins such as git-worktree).
pub fn create_child_session(
    repo_id: &str,
    parent_id: &str,
    name: &str,
    bare: bool,
    env: HashMap<String, String>,
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
        bare,
        last_accessed: None,
    };

    let skip_hooks = bare || !repo_entry.is_git;
    if !skip_hooks {
        handle_branch_conflict(&repo_entry.path, name)?;
    }
    tree.add(session.clone())?;

    if bare {
        log::debug!("bare session '{}': skipping OnSessionCreate hooks", name);
    } else if repo_entry.is_git {
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
    } else {
        log::debug!(
            "create_child_session: non-git repo, setting path to repo root and skipping hooks"
        );
        if let Some(s) = tree.sessions.iter_mut().find(|s| s.id == session_id) {
            s.path = Some(repo_entry.path.clone());
        }
    }

    store::save_sessions(repo_id, &tree)?;

    let created = tree.find_by_id(&session_id).cloned().unwrap_or(session);
    Ok(created)
}

/// When the new session's name matches an existing local branch, prompt the user to
/// choose between reusing the existing branch or recreating it from the updated base.
///
/// Must be called BEFORE `tree.add` so that a cancelled or failed prompt leaves no
/// orphan session record behind.
pub(crate) fn handle_branch_conflict(repo_path: &Path, name: &str) -> Result<()> {
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

    let non_bare: Vec<Session> = to_reap.into_iter().filter(|s| !s.bare).collect();
    if !non_bare.is_empty() {
        spawn_detached_reap(repo_id, &non_bare)?;
    } else {
        log::debug!("delete_session_by_id: all sessions are bare, skipping reap");
    }

    Ok(())
}

/// Rename a session by ID. Used by the browser action menu.
pub fn rename_session_by_id(repo_id: &str, session_id: &str, new_name: &str) -> Result<()> {
    let repo_entry = repo::store::load_index()?
        .repos
        .into_iter()
        .find(|r| r.id == repo_id)
        .ok_or_else(|| EzError::RepoNotFound(repo_id.into()))?;

    let mut tree = store::load_sessions(repo_id)?;

    if tree.find_by_name(new_name).is_some() {
        return Err(EzError::SessionAlreadyExists(new_name.into()));
    }

    let session = tree
        .sessions
        .iter_mut()
        .find(|s| s.id == session_id)
        .ok_or_else(|| EzError::SessionNotFound(session_id.into()))?;

    let old_name = session.name.clone();
    let rename_result =
        perform_session_rename(session, new_name, &repo_entry.path, repo_entry.is_git);

    store::save_sessions(repo_id, &tree)?;

    let config = crate::config::load()?;
    if config.copy_cursor_conversations {
        if let (Some(old_path), Some(new_path)) = (&rename_result.old_path, &rename_result.new_path)
        {
            cursor::copy_cursor_conversations(old_path, new_path);
        }
    }

    let repo_meta = repo::store::load_repo_meta(&repo_entry.id)?;
    run_rename_hooks(
        &repo_entry,
        &repo_meta,
        &tree,
        &old_name,
        new_name,
        &rename_result,
        &config,
    );

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
        .stderr(Stdio::null())
        .env("TMUX", "");

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

    log::debug!(
        "reap_delete: processing {} sessions for repo {}",
        payload.sessions.len(),
        payload.repo_id
    );

    let reap_delay_ms: u64 = config
        .plugin_settings
        .get("tmux")
        .and_then(|m| m.get("reap_delay_ms"))
        .and_then(|v| v.as_integer())
        .map(|v| v as u64)
        .unwrap_or(200);

    std::thread::sleep(std::time::Duration::from_millis(reap_delay_ms));

    // Build a throwaway SessionTree from the snapshot so plugin::run_hooks
    // can resolve parent information.  This tree is never saved to disk.
    let mut tree = SessionTree {
        sessions: payload.sessions.clone(),
    };

    for session in &payload.sessions {
        log::debug!(
            "reap_delete: running OnSessionDelete for session '{}'",
            session.name
        );
        // Swallow hook errors — the record is already removed; this is
        // best-effort external cleanup.
        let result = plugin::run_hooks(
            plugin::model::HookType::OnSessionDelete,
            &repo_entry,
            &repo_meta,
            Some(session),
            &config,
            &mut tree,
        );
        log::debug!(
            "reap_delete: hook result for '{}': {:?}",
            session.name,
            result
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
            bare: false,
            last_accessed: None,
        };
        tree.add(session)?;
        store::save_sessions(repo_id, &tree)?;
    }
    Ok(tree)
}
