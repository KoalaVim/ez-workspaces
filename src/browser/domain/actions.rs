//! Session-list action handlers. Each returns `Vec<Effect>` rather than
//! writing files or exiting; the front end applies the effects.
//!
//! Interactive sub-flows (name builder, confirm, input) still pull from the
//! `InteractiveSelector` held by [`ActionCtx`] (wizards stay pull-style).
//! Errors (including `EzError::Interrupted` from Ctrl-C) propagate with `?`;
//! Esc inside a sub-flow yields an empty effect list.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use colored::Colorize;

use crate::config::model::EzConfig;
use crate::error::Result;
use crate::plugin::{self, PluginBindInfo};
use crate::repo;
use crate::repo::model::RepoEntry;
use crate::session;
use crate::session::model::Session;

use super::super::format::parse_label_input;
use super::super::git::is_dirty;
use super::super::selector::InteractiveSelector;
use super::accept::{accept_session, bind_response_effects, on_create_is_noop};
use super::{Effect, Level, RefreshScope};

/// Everything a session-list handler needs.
pub(crate) struct ActionCtx<'a> {
    pub repo_entry: &'a RepoEntry,
    pub config: &'a EzConfig,
    pub selector: &'a dyn InteractiveSelector,
    pub plugin_binds: &'a [PluginBindInfo],
    /// Present when the shell wrapper accepts post-exit commands.
    pub post_cmd_file: Option<&'a Path>,
}

impl ActionCtx<'_> {
    fn post_cmds_supported(&self) -> bool {
        self.post_cmd_file.is_some()
    }

    fn accept(&self, on: &str, selected: &Session, target_dir: &Path) -> Vec<Effect> {
        accept_session(
            on,
            self.repo_entry,
            selected,
            target_dir,
            self.config,
            self.post_cmds_supported(),
        )
    }

    fn target_dir_of(&self, s: &Session) -> PathBuf {
        s.path
            .as_ref()
            .cloned()
            .unwrap_or_else(|| self.repo_entry.path.clone())
    }

    /// After creating a session: either report it, or navigate per `on_create`.
    fn after_create(&self, created: &Session, message: String) -> Vec<Effect> {
        if on_create_is_noop(&self.config.on_create) {
            vec![
                Effect::Notify(message, Level::Info),
                Effect::Refresh(RefreshScope::Sessions),
            ]
        } else {
            let target_dir = self.target_dir_of(created);
            self.accept(&self.config.on_create, created, &target_dir)
        }
    }
}

/// Update `last_accessed` timestamp on a session and its repo after browser selection.
/// Also triggers PR auto-detection if the session has no PR metadata.
pub(crate) fn update_last_accessed(repo_entry: &RepoEntry, session_id: &str) {
    let now = chrono::Utc::now().to_rfc3339();
    if let Ok(mut tree) = session::store::load_sessions(&repo_entry.id) {
        session::detect_pr_for_session(&mut tree, session_id, repo_entry);
        if let Some(s) = tree.sessions.iter_mut().find(|s| s.id == session_id) {
            s.last_accessed = Some(now.clone());
        }
        let _ = session::store::save_sessions(&repo_entry.id, &tree);
    }
    if let Ok(mut meta) = repo::store::load_repo_meta(&repo_entry.id) {
        meta.last_accessed = Some(now);
        let _ = repo::store::save_repo_meta(&repo_entry.id, &meta);
    }
}

/// Enter pressed on a managed session.
pub(crate) fn select_session(ctx: &ActionCtx<'_>, selected: &Session) -> Vec<Effect> {
    update_last_accessed(ctx.repo_entry, &selected.id);
    let target_dir = ctx.target_dir_of(selected);
    ctx.accept(&ctx.config.on_enter, selected, &target_dir)
}

/// Enter pressed on an unregistered worktree: register it, then accept it.
pub(crate) fn select_unmanaged(
    ctx: &ActionCtx<'_>,
    wt_path: &Path,
    branch: Option<&str>,
) -> Vec<Effect> {
    match session::register_worktree_inline(&ctx.repo_entry.id, wt_path, branch) {
        Ok(created) => {
            let mut effects = vec![Effect::Notify(
                format!("{} {}", "Registered:".green(), created.name.bold()),
                Level::Info,
            )];
            update_last_accessed(ctx.repo_entry, &created.id);
            effects.extend(ctx.accept(&ctx.config.on_enter, &created, wt_path));
            effects
        }
        Err(e) => vec![Effect::Notify(
            format!("{} {}", "Register failed:".red(), e),
            Level::Error,
        )],
    }
}

/// The delete keybind pressed on an unregistered worktree.
pub(crate) fn delete_unmanaged(
    ctx: &ActionCtx<'_>,
    wt_path: &Path,
    branch: Option<&str>,
) -> Result<Vec<Effect>> {
    let dirty = is_dirty(wt_path);
    let branch_label = branch.unwrap_or("(unknown)");
    let msg = if dirty {
        format!(
            "Worktree '{}' has uncommitted changes. Delete anyway?",
            branch_label
        )
    } else {
        format!("Delete unregistered worktree '{}'?", branch_label)
    };
    if !ctx.selector.confirm(&msg, false)? {
        return Ok(Vec::new());
    }
    Ok(
        match session::delete_unmanaged_worktree(&ctx.repo_entry.path, wt_path, true) {
            Ok(()) => vec![
                Effect::Notify(
                    format!("{} {}", "Deleted worktree:".green(), branch_label.bold()),
                    Level::Info,
                ),
                Effect::Refresh(RefreshScope::Sessions),
            ],
            Err(e) => vec![Effect::Notify(
                format!("{} {}", "Delete failed:".red(), e),
                Level::Error,
            )],
        },
    )
}

/// Handle an action keybind pressed on a managed session.
///
/// Returns `None` when `key` is neither a session action nor a plugin bind,
/// so the caller can try its own keys (view switches, sort toggle, ...).
pub(crate) fn handle_session_action(
    ctx: &ActionCtx<'_>,
    key: &str,
    selected: &Session,
) -> Result<Option<Vec<Effect>>> {
    let keybinds = &ctx.config.keybinds;
    let effects = if key == keybinds.new_session {
        new_child(ctx, selected)?
    } else if key == keybinds.new_bare_session {
        new_bare(ctx, selected)?
    } else if key == keybinds.session_from_dirty {
        from_dirty(ctx, selected)?
    } else if key == keybinds.delete_session {
        delete(ctx, selected)?
    } else if key == keybinds.rename_session {
        rename(ctx, selected)?
    } else if key == keybinds.edit_labels {
        edit_labels(ctx, selected)?
    } else if key == keybinds.cd_session {
        vec![Effect::Cd(ctx.target_dir_of(selected)), Effect::Exit]
    } else if key == keybinds.note_open {
        vec![Effect::OpenNote {
            repo_id: ctx.repo_entry.id.clone(),
            session_id: selected.id.clone(),
        }]
    } else if key == keybinds.note_cd {
        note_cd(ctx, selected)
    } else if let Some(pb) = ctx.plugin_binds.iter().find(|pb| key == pb.key) {
        plugin_bind(ctx, pb, selected)?
    } else {
        return Ok(None);
    };
    Ok(Some(effects))
}

fn new_child(ctx: &ActionCtx<'_>, selected: &Session) -> Result<Vec<Effect>> {
    match session::name_builder::prompt_session_name(
        ctx.selector,
        ctx.config,
        Some(&ctx.repo_entry.path),
    )? {
        session::name_builder::NamePromptResult::Done { name, pr_metadata } => {
            let env = pr_metadata
                .as_ref()
                .map(|pr| pr.to_session_env())
                .unwrap_or_default();
            let created =
                session::create_child_session(&ctx.repo_entry.id, &selected.id, &name, false, env)?;
            let message = format!(
                "{} {} → {}",
                "Created:".green(),
                name.bold(),
                selected.name.dimmed()
            );
            Ok(ctx.after_create(&created, message))
        }
        session::name_builder::NamePromptResult::Cancelled => Ok(Vec::new()),
    }
}

fn new_bare(ctx: &ActionCtx<'_>, selected: &Session) -> Result<Vec<Effect>> {
    match session::name_builder::prompt_session_name(
        ctx.selector,
        ctx.config,
        Some(&ctx.repo_entry.path),
    )? {
        session::name_builder::NamePromptResult::Done { name, .. } => {
            let created = session::create_child_session(
                &ctx.repo_entry.id,
                &selected.id,
                &name,
                true,
                HashMap::new(),
            )?;
            let message = format!(
                "{} {} {} → {}",
                "Created bare:".green(),
                name.bold(),
                "[bare]".dimmed(),
                selected.name.dimmed()
            );
            Ok(ctx.after_create(&created, message))
        }
        session::name_builder::NamePromptResult::Cancelled => Ok(Vec::new()),
    }
}

fn from_dirty(ctx: &ActionCtx<'_>, selected: &Session) -> Result<Vec<Effect>> {
    match session::name_builder::prompt_session_name(
        ctx.selector,
        ctx.config,
        Some(&ctx.repo_entry.path),
    )? {
        session::name_builder::NamePromptResult::Done { name, .. } => {
            match session::from_dirty::session_from_dirty_inner(&name, None, None) {
                Ok(created) => {
                    let message = format!(
                        "{} {} → {}",
                        "Created from dirty:".green(),
                        name.bold(),
                        selected.name.dimmed()
                    );
                    Ok(ctx.after_create(&created, message))
                }
                Err(e) => Ok(vec![Effect::Notify(
                    format!("{} {}", "From dirty failed:".red(), e),
                    Level::Error,
                )]),
            }
        }
        session::name_builder::NamePromptResult::Cancelled => Ok(Vec::new()),
    }
}

fn delete(ctx: &ActionCtx<'_>, selected: &Session) -> Result<Vec<Effect>> {
    let repo_id = &ctx.repo_entry.id;
    let dirty = session::cascade_dirty(repo_id, &selected.id)?;
    let unchecked = session::cascade_unchecked_todos(repo_id, &selected.id)?;
    let mut warnings: Vec<String> = Vec::new();
    if !dirty.is_empty() {
        warnings.push(format!("Worktree(s) {:?} have uncommitted changes", dirty));
    }
    if !unchecked.is_empty() {
        warnings.push(format!(
            "Session(s) {:?} have {}",
            unchecked,
            "unchecked TODOs in notes".yellow()
        ));
    }
    let msg = if warnings.is_empty() {
        format!("Delete session '{}'?", selected.name)
    } else {
        format!("{}. Delete anyway?", warnings.join(". "))
    };
    if !ctx.selector.confirm(&msg, false)? {
        return Ok(Vec::new());
    }
    // Plugin delete hooks append their post-exit commands to the post-cmd file directly.
    session::delete_session_by_id(repo_id, &selected.id, true, ctx.post_cmd_file)?;
    Ok(vec![
        Effect::Notify(
            format!("{} {}", "Deleted:".green(), selected.name.bold()),
            Level::Info,
        ),
        Effect::Refresh(RefreshScope::Sessions),
    ])
}

fn rename(ctx: &ActionCtx<'_>, selected: &Session) -> Result<Vec<Effect>> {
    let new_name = ctx.selector.input("New name", Some(&selected.name))?;
    if new_name.is_empty() || new_name == selected.name {
        return Ok(Vec::new());
    }
    session::rename_session_by_id(&ctx.repo_entry.id, &selected.id, &new_name)?;
    Ok(vec![
        Effect::Notify(
            format!(
                "{} {} → {}",
                "Renamed:".green(),
                selected.name.bold(),
                new_name.bold()
            ),
            Level::Info,
        ),
        Effect::Refresh(RefreshScope::Sessions),
    ])
}

fn edit_labels(ctx: &ActionCtx<'_>, selected: &Session) -> Result<Vec<Effect>> {
    let current = selected.labels.join(",");
    let input = ctx
        .selector
        .input("Labels (comma-sep; prefix - to remove)", Some(&current))?;
    let (add, remove) = parse_label_input(&input);
    let result = session::set_session_labels(&ctx.repo_entry.id, &selected.id, &add, &remove)?;
    Ok(vec![
        Effect::Notify(
            format!(
                "{} {} → {}",
                "Labels on".green(),
                selected.name.bold(),
                if result.is_empty() {
                    "(none)".dimmed().to_string()
                } else {
                    result.join(", ").magenta().to_string()
                }
            ),
            Level::Info,
        ),
        Effect::Refresh(RefreshScope::Sessions),
    ])
}

fn note_cd(ctx: &ActionCtx<'_>, selected: &Session) -> Vec<Effect> {
    match session::notes::ensure_notes_dir(&ctx.repo_entry.id, &selected.id) {
        Ok(dir) => vec![Effect::Cd(dir), Effect::Exit],
        Err(e) => vec![Effect::Notify(
            format!("{} {}", "Note cd failed:".red(), e),
            Level::Error,
        )],
    }
}

fn plugin_bind(
    ctx: &ActionCtx<'_>,
    pb: &PluginBindInfo,
    selected: &Session,
) -> Result<Vec<Effect>> {
    let response = plugin::run_bind_hook(
        &pb.plugin_name,
        &pb.bind_name,
        &pb.key,
        "session",
        &selected.id,
        &selected.name,
        ctx.repo_entry,
        Some(selected),
        ctx.config,
    )?;
    let mut effects = bind_response_effects(&response, ctx.post_cmds_supported());
    if response.cd_target.is_some() || !response.post_shell_commands.is_empty() {
        if !response.accept {
            effects.push(Effect::PostCmds {
                cmds: Vec::new(),
                relaunch: true,
            });
        }
        effects.push(Effect::Exit);
    }
    Ok(effects)
}
