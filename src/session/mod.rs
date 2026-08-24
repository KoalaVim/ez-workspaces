pub mod current;
pub mod cursor;
mod env;
pub mod from_dirty;
pub mod model;
pub mod name_builder;
pub mod notes;
pub mod store;
pub mod tree;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use chrono::Utc;
use colored::Colorize;
use uuid::Uuid;

use crate::cli::{SessionCommand, SessionLabelCommand, SessionNoteCommand};
use crate::error::{EzError, Result};
use crate::plugin;
use crate::repo;
use model::{Session, SessionTree};

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
        SessionCommand::List {
            repo,
            flat,
            json,
            all,
        } => list_sessions(repo.as_deref(), flat, json, all),
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
            delete_session(name.as_deref(), repo.as_deref(), force, post_cmd_file)
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
        SessionCommand::Note { command } => dispatch_note(command, cd_file),
        SessionCommand::Env { command } => env::dispatch_env(command),
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

fn dispatch_note(cmd: SessionNoteCommand, cd_file: Option<&Path>) -> Result<()> {
    let config = crate::config::load()?;

    let resolve_session =
        |name: Option<&str>, repo_arg: Option<&str>| -> Result<(String, String)> {
            match name {
                Some(n) => {
                    let repo_entry = repo::resolve_repo(repo_arg)?;
                    let tree = store::load_sessions(&repo_entry.id)?;
                    let session = tree
                        .find_by_name(n)
                        .ok_or_else(|| EzError::SessionNotFound(n.into()))?;
                    Ok((repo_entry.id.clone(), session.id.clone()))
                }
                None => {
                    let target = current::resolve_current_session(repo_arg)?;
                    Ok((target.repo_entry.id.clone(), target.session.id.clone()))
                }
            }
        };

    match cmd {
        SessionNoteCommand::Open { name, repo } => {
            let (repo_id, session_id) = resolve_session(name.as_deref(), repo.as_deref())?;
            notes::open_note(&repo_id, &session_id, &config)
        }
        SessionNoteCommand::Cd { name, repo } => {
            let (repo_id, session_id) = resolve_session(name.as_deref(), repo.as_deref())?;
            let dir = notes::ensure_notes_dir(&repo_id, &session_id)?;
            if let Some(cd_path) = cd_file {
                std::fs::write(cd_path, dir.display().to_string())?;
            } else {
                println!("{}", dir.display());
            }
            Ok(())
        }
        SessionNoteCommand::Path { name, repo } => {
            let (repo_id, session_id) = resolve_session(name.as_deref(), repo.as_deref())?;
            let dir = crate::paths::notes_dir(&repo_id, &session_id)?;
            println!("{}", dir.display());
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
        _ => name_builder::prompt_session_name_default(&config, Some(&repo_entry.path))?,
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

fn list_sessions(repo_arg: Option<&str>, flat: bool, json: bool, all: bool) -> Result<()> {
    if all {
        if repo_arg.is_some() {
            return Err(EzError::Config(
                "--all and --repo are mutually exclusive: --all lists every registered repo".into(),
            ));
        }
        return list_all_sessions(flat, json);
    }

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
        let items = sessions_json(&tree.sessions);
        println!(
            "{}",
            serde_json::to_string_pretty(&items).unwrap_or_else(|_| "[]".into())
        );
        return Ok(());
    }

    let repo_meta = repo::store::load_repo_meta(&repo_entry.id).unwrap_or_default();
    let cfg = crate::config::load().unwrap_or_default();
    let attached = plugin::get_attached_sessions(&repo_entry, &repo_meta, &tree, &cfg);
    print_sessions(&tree, flat, &attached);
    Ok(())
}

/// `ez session list --all`: every registered repo in one pass.
fn list_all_sessions(flat: bool, json: bool) -> Result<()> {
    let repos = repo::store::load_index()?.repos;

    if json {
        let items: Vec<serde_json::Value> = repos
            .iter()
            .map(|repo_entry| {
                let sessions = store::load_sessions(&repo_entry.id)
                    .map(|t| t.sessions)
                    .unwrap_or_default();
                serde_json::json!({
                    "id": repo_entry.id,
                    "name": repo_entry.name,
                    "path": repo_entry.path.display().to_string(),
                    "sessions": sessions_json(&sessions),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&items).unwrap_or_else(|_| "[]".into())
        );
        return Ok(());
    }

    if repos.is_empty() {
        println!(
            "{}",
            "No repos registered. Use `ez add` to register one.".yellow()
        );
        return Ok(());
    }

    for repo_entry in &repos {
        let tree = store::load_sessions(&repo_entry.id)?;
        println!(
            "{} {}",
            repo_entry.name.bold().cyan(),
            format!("({})", repo_entry.path.display()).dimmed()
        );
        if tree.sessions.is_empty() {
            println!("  {}", "no sessions".dimmed());
        } else {
            let repo_meta = repo::store::load_repo_meta(&repo_entry.id).unwrap_or_default();
            let cfg = crate::config::load().unwrap_or_default();
            let attached = plugin::get_attached_sessions(repo_entry, &repo_meta, &tree, &cfg);
            print_sessions(&tree, flat, &attached);
        }
        println!();
    }
    Ok(())
}

/// JSON shape shared by `session list --json` and `session list --all --json`.
fn sessions_json(sessions: &[model::Session]) -> Vec<serde_json::Value> {
    sessions
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
        .collect()
}

/// Render one repo's sessions as a flat list or an indented tree.
fn print_sessions(tree: &SessionTree, flat: bool, attached: &HashSet<String>) {
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
            let name_colored = if attached.contains(&session.id) {
                session.name.bold().blue()
            } else {
                session.name.bold().yellow()
            };
            println!("{}{}{}", name_colored, default_marker, bare_indicator);
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
            let name_colored = if attached.contains(&node.session.id) {
                node.session.name.bold().blue()
            } else {
                node.session.name.bold().yellow()
            };
            println!("{}{}{}", prefix, name_colored, default_marker);
        }
    }
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

/// Returns the names of sessions (target + descendants) that have unchecked TODOs in notes.
/// Used by the TUI to warn the user before performing a forced delete.
pub fn cascade_unchecked_todos(repo_id: &str, session_id: &str) -> Result<Vec<String>> {
    let tree = store::load_sessions(repo_id)?;
    let sid = session_id.to_string();
    let session = tree
        .find_by_id(&sid)
        .ok_or_else(|| EzError::SessionNotFound(session_id.into()))?;
    let mut to_check: Vec<&model::Session> = tree.descendants(&session.id).into_iter().collect();
    to_check.push(session);
    Ok(to_check
        .iter()
        .filter_map(|s| {
            let todos = notes::unchecked_todos(repo_id, &s.id);
            if todos.is_empty() {
                None
            } else {
                Some(s.name.clone())
            }
        })
        .collect())
}

fn delete_session(
    name: Option<&str>,
    repo_arg: Option<&str>,
    force: bool,
    post_cmd_file: Option<&Path>,
) -> Result<()> {
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

    if !force {
        let todos: Vec<String> = to_reap
            .iter()
            .filter_map(|s| {
                let todos = notes::unchecked_todos(&repo_entry.id, &s.id);
                if todos.is_empty() {
                    None
                } else {
                    Some(s.name.clone())
                }
            })
            .collect();
        if !todos.is_empty() {
            return Err(EzError::SessionHasUncheckedTodos { sessions: todos });
        }
    }

    for s in &to_reap {
        tree.remove(&s.id)?;
    }
    store::save_sessions(&repo_entry.id, &tree)?;

    for s in &to_reap {
        if let Err(e) = notes::delete_notes_dir(&repo_entry.id, &s.id) {
            log::debug!(
                "delete_session: notes cleanup for '{}' failed: {}",
                s.name,
                e
            );
        }
    }

    println!("{} {}", "Deleted session:".green(), session.name.bold());

    let config = crate::config::load()?;
    let repo_meta = repo::store::load_repo_meta(&repo_entry.id)?;
    let mut hook_tree = SessionTree {
        sessions: to_reap.clone(),
    };
    let mut all_post_commands: Vec<String> = Vec::new();

    for s in &to_reap {
        if s.bare {
            continue;
        }
        log::debug!("delete_session: running OnSessionDelete for '{}'", s.name);
        match plugin::run_hooks(
            plugin::model::HookType::OnSessionDelete,
            &repo_entry,
            &repo_meta,
            Some(s),
            &config,
            &mut hook_tree,
        ) {
            Ok(post_cmds) => all_post_commands.extend(post_cmds),
            Err(e) => log::debug!("delete_session: hook error for '{}': {}", s.name, e),
        }
    }

    if !all_post_commands.is_empty() {
        crate::browser::write_post_commands(post_cmd_file, &all_post_commands)?;
    }

    Ok(())
}

/// Get the currently authenticated GitHub CLI user's login name, if any.
pub(crate) fn get_current_gh_user() -> Option<String> {
    if which::which("gh").is_err() {
        return None;
    }

    let output = Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .stderr(Stdio::null())
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let login = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if login.is_empty() {
                None
            } else {
                Some(login)
            }
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            log::debug!("get_current_gh_user: gh api user failed: {stderr}");
            None
        }
        Err(e) => {
            log::debug!("get_current_gh_user: failed to run gh: {e}");
            None
        }
    }
}

/// Refresh the PR status for a session if it has PR metadata and the status
/// is stale (older than 5 minutes). Updates the session env in-place.
pub(crate) fn refresh_pr_status(tree: &mut SessionTree, session_id: &str) {
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

/// Auto-detect a GitHub PR for a session by its branch name.
/// Skips bare sessions, non-git repos, default sessions, and sessions that
/// already have `ez_pr_number` set. Returns `true` if PR metadata was populated.
pub(crate) fn detect_pr_for_session(
    tree: &mut SessionTree,
    session_id: &str,
    repo_entry: &crate::repo::model::RepoEntry,
) -> bool {
    let (branch, should_detect) = {
        let session = match tree.sessions.iter().find(|s| s.id == session_id) {
            Some(s) => s,
            None => return false,
        };
        if session.bare || session.is_default || !repo_entry.is_git {
            return false;
        }
        if session.env.contains_key("ez_pr_number") {
            return false;
        }
        let path = match &session.path {
            Some(p) if p.exists() => p.clone(),
            _ => return false,
        };
        let branch = git_output(&path, &["rev-parse", "--abbrev-ref", "HEAD"])
            .ok()
            .filter(|b| b != "HEAD");
        let should = branch.is_some();
        (branch, should)
    };

    if !should_detect {
        return false;
    }
    let branch = match branch {
        Some(b) => b,
        None => return false,
    };

    if which::which("gh").is_err() {
        log::debug!("detect_pr_for_session: gh not found, skipping");
        return false;
    }

    log::debug!("detect_pr_for_session: checking for PR on branch '{branch}'");

    let output = Command::new("gh")
        .args([
            "pr",
            "list",
            "--head",
            &branch,
            "--state",
            "all",
            "--json",
            "number,url,state",
            "--limit",
            "1",
        ])
        .current_dir(&repo_entry.path)
        .stderr(Stdio::null())
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let prs: Vec<serde_json::Value> = match serde_json::from_slice(&o.stdout) {
                Ok(v) => v,
                Err(e) => {
                    log::debug!("detect_pr_for_session: failed to parse gh output: {e}");
                    return false;
                }
            };
            let pr = match prs.first() {
                Some(pr) => pr,
                None => {
                    log::debug!("detect_pr_for_session: no PR found for branch '{branch}'");
                    return false;
                }
            };
            let number = pr.get("number").and_then(|v| v.as_u64());
            let url = pr.get("url").and_then(|v| v.as_str());
            let state = pr.get("state").and_then(|v| v.as_str()).unwrap_or("open");

            if let (Some(number), Some(url)) = (number, url) {
                log::debug!(
                    "detect_pr_for_session: found PR #{number} ({state}) for branch '{branch}'"
                );
                if let Some(s) = tree.sessions.iter_mut().find(|s| s.id == session_id) {
                    s.env.insert("ez_pr_number".into(), number.to_string());
                    s.env.insert("ez_pr_url".into(), url.to_string());
                    s.env.insert("ez_pr_status".into(), state.to_lowercase());
                    s.env
                        .insert("ez_pr_status_updated".into(), Utc::now().to_rfc3339());
                    if let Some(gh_user) = get_current_gh_user() {
                        s.env.insert("ez_pr_gh_user".into(), gh_user);
                    }
                }
                return true;
            }
            false
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            log::debug!("detect_pr_for_session: gh pr list failed: {stderr}");
            false
        }
        Err(e) => {
            log::debug!("detect_pr_for_session: failed to run gh: {e}");
            false
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

    let hook_post_commands = plugin::run_hooks(
        plugin::model::HookType::OnSessionEnter,
        &repo_entry,
        &repo_meta,
        Some(&session),
        &config,
        &mut tree,
    )?;
    if !hook_post_commands.is_empty() {
        crate::browser::write_post_commands(post_cmd_file, &hook_post_commands)?;
    }

    detect_pr_for_session(&mut tree, &session.id, &repo_entry);

    let now = Utc::now().to_rfc3339();
    if let Some(s) = tree.sessions.iter_mut().find(|s| s.id == session.id) {
        s.last_accessed = Some(now.clone());
    }
    store::save_sessions(&repo_entry.id, &tree)?;

    // Re-fetch the session from the tree to pick up any mutations from hooks
    let session = tree
        .find_by_name(name)
        .ok_or_else(|| EzError::SessionNotFound(name.into()))?
        .clone();

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
pub fn delete_session_by_id(
    repo_id: &str,
    session_id: &str,
    force: bool,
    post_cmd_file: Option<&Path>,
) -> Result<()> {
    // Verify the repo exists before doing anything.
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

    if !force {
        let todos: Vec<String> = to_reap
            .iter()
            .filter_map(|s| {
                let todos = notes::unchecked_todos(repo_id, &s.id);
                if todos.is_empty() {
                    None
                } else {
                    Some(s.name.clone())
                }
            })
            .collect();
        if !todos.is_empty() {
            return Err(EzError::SessionHasUncheckedTodos { sessions: todos });
        }
    }

    // Persist removal synchronously before any hook can tear down the terminal.
    for s in &to_reap {
        tree.remove(&s.id)?;
    }
    store::save_sessions(repo_id, &tree)?;

    for s in &to_reap {
        if let Err(e) = notes::delete_notes_dir(repo_id, &s.id) {
            log::debug!(
                "delete_session_by_id: notes cleanup for '{}' failed: {}",
                s.name,
                e
            );
        }
    }

    let config = crate::config::load()?;
    let repo_meta = repo::store::load_repo_meta(&repo_entry.id)?;
    let mut hook_tree = SessionTree {
        sessions: to_reap.clone(),
    };
    let mut all_post_commands: Vec<String> = Vec::new();

    for s in &to_reap {
        if s.bare {
            continue;
        }
        log::debug!(
            "delete_session_by_id: running OnSessionDelete for '{}'",
            s.name
        );
        match plugin::run_hooks(
            plugin::model::HookType::OnSessionDelete,
            &repo_entry,
            &repo_meta,
            Some(s),
            &config,
            &mut hook_tree,
        ) {
            Ok(post_cmds) => all_post_commands.extend(post_cmds),
            Err(e) => log::debug!("delete_session_by_id: hook error for '{}': {}", s.name, e),
        }
    }

    if !all_post_commands.is_empty() {
        crate::browser::write_post_commands(post_cmd_file, &all_post_commands)?;
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

/// Remove a git worktree that is not tracked as an ez session.
///
/// Strategy: try `git worktree remove [--force]`. If that fails (e.g. the
/// worktree's `.git` link is broken), fall back to removing the directory
/// manually and running `git worktree prune` to clean up stale refs.
pub fn delete_unmanaged_worktree(repo_path: &Path, worktree_path: &Path, force: bool) -> Result<()> {
    let wt_str = worktree_path.to_string_lossy().to_string();

    // Attempt 1: normal remove
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(&wt_str);

    log::debug!(
        "delete_unmanaged_worktree: git {} in {}",
        args.join(" "),
        repo_path.display()
    );

    let output = Command::new("git")
        .args(&args)
        .current_dir(repo_path)
        .output()?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    log::debug!("delete_unmanaged_worktree: first attempt failed: {}", stderr);

    // Attempt 2: force retry (handles cwd-inside-worktree case)
    let retry = Command::new("git")
        .args(["worktree", "remove", "--force", &wt_str])
        .current_dir(repo_path)
        .output()?;

    if retry.status.success() {
        return Ok(());
    }

    let retry_err = String::from_utf8_lossy(&retry.stderr).trim().to_string();
    log::debug!("delete_unmanaged_worktree: force retry failed: {}", retry_err);

    // Attempt 3: broken worktree (missing .git link) — remove dir + prune
    if worktree_path.exists() {
        log::debug!(
            "delete_unmanaged_worktree: falling back to rm + prune for {}",
            worktree_path.display()
        );
        std::fs::remove_dir_all(worktree_path)?;
        let _ = Command::new("git")
            .args(["worktree", "prune"])
            .current_dir(repo_path)
            .output();
        return Ok(());
    }

    Err(EzError::Git(format!(
        "git worktree remove failed: {}",
        if retry_err.is_empty() { stderr } else { retry_err }
    )))
}

/// A git worktree that exists on disk but is not tracked as an ez session.
#[derive(Debug, Clone)]
pub struct UnmanagedWorktree {
    pub path: PathBuf,
    pub branch: Option<String>,
}

/// Detect git worktrees not tracked as ez sessions.
///
/// Runs `git worktree list --porcelain`, subtracts the main repo path and
/// any path already used by a session. Skips non-git repos.
pub fn list_unmanaged_worktrees(
    repo_entry: &repo::model::RepoEntry,
    tree: &SessionTree,
) -> Vec<UnmanagedWorktree> {
    if !repo_entry.is_git {
        return Vec::new();
    }

    let output = match Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(&repo_entry.path)
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            log::debug!("list_unmanaged_worktrees: git worktree list failed: {stderr}");
            return Vec::new();
        }
        Err(e) => {
            log::debug!("list_unmanaged_worktrees: failed to run git: {e}");
            return Vec::new();
        }
    };

    let worktrees = parse_worktree_list_porcelain(&output);

    let repo_canonical = repo_entry.path.canonicalize().ok();

    // Also exclude the git-common-dir (handles submodules where the bare repo
    // at .git/modules/<name> appears as a worktree entry).
    let git_common_canonical = git_output(&repo_entry.path, &["rev-parse", "--git-common-dir"])
        .ok()
        .map(|d| {
            let p = PathBuf::from(&d);
            if p.is_absolute() {
                p
            } else {
                repo_entry.path.join(p)
            }
        })
        .and_then(|p| p.canonicalize().ok());

    let managed_paths: Vec<PathBuf> = tree
        .sessions
        .iter()
        .filter_map(|s| s.path.as_ref())
        .filter_map(|p| p.canonicalize().ok())
        .collect();

    worktrees
        .into_iter()
        .filter(|wt| {
            let canonical = match wt.path.canonicalize() {
                Ok(c) => c,
                Err(_) => {
                    log::debug!(
                        "list_unmanaged_worktrees: prunable (path gone): {}",
                        wt.path.display()
                    );
                    return false;
                }
            };
            if repo_canonical.as_ref() == Some(&canonical) {
                return false;
            }
            if git_common_canonical.as_ref() == Some(&canonical) {
                log::debug!(
                    "list_unmanaged_worktrees: skipping git-common-dir: {}",
                    wt.path.display()
                );
                return false;
            }
            if managed_paths.contains(&canonical) {
                return false;
            }
            true
        })
        .collect()
}

/// Parse `git worktree list --porcelain` output into worktree entries.
///
/// Each block is separated by a blank line and contains lines like:
///   worktree /path/to/wt
///   HEAD abc123...
///   branch refs/heads/feature
/// or:
///   worktree /path/to/wt
///   HEAD abc123...
///   detached
fn parse_worktree_list_porcelain(output: &str) -> Vec<UnmanagedWorktree> {
    let mut result = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;
    let mut is_detached = false;
    let mut head_sha: Option<String> = None;

    for line in output.lines() {
        if line.is_empty() {
            if let Some(path) = current_path.take() {
                let branch = if is_detached {
                    head_sha.as_ref().map(|sha| {
                        if sha.len() > 7 {
                            sha[..7].to_string()
                        } else {
                            sha.clone()
                        }
                    })
                } else {
                    current_branch.take()
                };
                result.push(UnmanagedWorktree { path, branch });
            }
            current_branch = None;
            is_detached = false;
            head_sha = None;
            continue;
        }

        if let Some(rest) = line.strip_prefix("worktree ") {
            current_path = Some(PathBuf::from(rest));
        } else if let Some(rest) = line.strip_prefix("HEAD ") {
            head_sha = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("branch ") {
            current_branch = rest.strip_prefix("refs/heads/").map(|s| s.to_string());
        } else if line == "detached" {
            is_detached = true;
        }
    }

    // Handle last block (porcelain output may not end with a blank line)
    if let Some(path) = current_path.take() {
        let branch = if is_detached {
            head_sha.as_ref().map(|sha| {
                if sha.len() > 7 {
                    sha[..7].to_string()
                } else {
                    sha.clone()
                }
            })
        } else {
            current_branch.take()
        };
        result.push(UnmanagedWorktree { path, branch });
    }

    result
}

/// Register a worktree as a session inline (from the browser), without running
/// OnSessionCreate hooks. Returns the created session.
pub fn register_worktree_inline(
    repo_id: &str,
    worktree_path: &Path,
    branch: Option<&str>,
) -> Result<Session> {
    let mut tree = store::load_sessions(repo_id)?;

    let base_name = branch
        .map(|b| b.to_string())
        .or_else(|| {
            worktree_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "worktree".to_string());

    let session_name = if tree.find_by_name(&base_name).is_some() {
        let mut suffix = 2;
        loop {
            let candidate = format!("{base_name}-{suffix}");
            if tree.find_by_name(&candidate).is_none() {
                break candidate;
            }
            suffix += 1;
        }
    } else {
        base_name.clone()
    };

    let parent_id = tree.find_default().map(|s| s.id.clone());

    let mut plugin_state = HashMap::new();
    plugin_state.insert(
        "worktree_path".to_string(),
        toml::Value::String(worktree_path.display().to_string()),
    );
    if let Some(b) = branch {
        plugin_state.insert("branch".to_string(), toml::Value::String(b.to_string()));
    }

    let session = Session {
        id: Uuid::new_v4().to_string(),
        name: session_name,
        parent_id,
        path: Some(worktree_path.to_path_buf()),
        env: HashMap::new(),
        plugin_state,
        labels: Vec::new(),
        created_at: Utc::now(),
        is_default: false,
        bare: false,
        last_accessed: None,
    };

    tree.add(session.clone())?;
    store::save_sessions(repo_id, &tree)?;

    log::debug!(
        "register_worktree_inline: registered '{}' -> {}",
        session.name,
        worktree_path.display()
    );

    Ok(session)
}

#[cfg(test)]
mod worktree_tests {
    use super::*;

    #[test]
    fn parse_porcelain_multiple_worktrees() {
        let output = "\
worktree /Users/me/repo
HEAD abc1234567890
branch refs/heads/main

worktree /Users/me/.ez/repo/feature
HEAD def4567890123
branch refs/heads/feature

worktree /Users/me/.ez/repo/experiment
HEAD 9876543210abc
branch refs/heads/experiment
";
        let result = parse_worktree_list_porcelain(output);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].path, PathBuf::from("/Users/me/repo"));
        assert_eq!(result[0].branch.as_deref(), Some("main"));
        assert_eq!(result[1].path, PathBuf::from("/Users/me/.ez/repo/feature"));
        assert_eq!(result[1].branch.as_deref(), Some("feature"));
        assert_eq!(
            result[2].path,
            PathBuf::from("/Users/me/.ez/repo/experiment")
        );
        assert_eq!(result[2].branch.as_deref(), Some("experiment"));
    }

    #[test]
    fn parse_porcelain_detached_head() {
        let output = "\
worktree /Users/me/repo
HEAD abc1234567890
branch refs/heads/main

worktree /Users/me/.ez/repo/detached
HEAD deadbeef12345
detached
";
        let result = parse_worktree_list_porcelain(output);
        assert_eq!(result.len(), 2);
        assert_eq!(result[1].branch.as_deref(), Some("deadbee"));
    }

    #[test]
    fn parse_porcelain_no_trailing_newline() {
        let output = "worktree /Users/me/repo\nHEAD abc1234567890\nbranch refs/heads/main";
        let result = parse_worktree_list_porcelain(output);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].branch.as_deref(), Some("main"));
    }

    #[test]
    fn parse_porcelain_empty() {
        let result = parse_worktree_list_porcelain("");
        assert!(result.is_empty());
    }
}
