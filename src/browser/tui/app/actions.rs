//! Row actions of the native browser. Every action:
//!
//! 1. re-checks its target against fresh data (refuses with a notice and a
//!    refresh when it vanished),
//! 2. runs the shared domain handler with the modal `NativeSelector`, with
//!    stdout/stderr captured so handler output cannot corrupt the screen,
//! 3. applies the returned effects with the native executor.

use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use super::keymap::RowCtx;
use super::App;
use crate::browser::domain::actions::{
    delete_unmanaged, handle_session_action, select_session, select_unmanaged, ActionCtx,
};
use crate::browser::domain::plugin_view::view_select_effects;
use crate::browser::domain::{fresh, repo_actions, Effect, Level as Severity, Row, RowKind};
use crate::browser::selector::InteractiveSelector;
use crate::browser::tui::effects::{self, Release, Target};
use crate::browser::tui::event::{Flow, Handler};
use crate::browser::tui::level::Source;
use crate::config::model::EzConfig;
use crate::error::{EzError, Result};
use crate::plugin::{self, PluginBindInfo, PluginViewInfo};
use crate::repo;
use crate::repo::model::RepoEntry;
use crate::session;
use crate::session::model::{Session, SessionTree};

/// What a handler closure gets to work with.
pub(super) struct Env<'a> {
    pub selector: &'a dyn InteractiveSelector,
    pub config: &'a EzConfig,
    pub binds: &'a [PluginBindInfo],
    pub post_cmd_file: Option<&'a Path>,
}

impl Env<'_> {
    fn ctx<'b>(&'b self, repo_entry: &'b RepoEntry) -> ActionCtx<'b> {
        ActionCtx {
            repo_entry,
            config: self.config,
            selector: self.selector,
            plugin_binds: self.binds,
            post_cmd_file: self.post_cmd_file,
        }
    }
}

impl App {
    /// The repo and highlighted row of the session list on top.
    fn session_row(&self) -> Option<(RepoEntry, Row)> {
        let level = self.nav.top();
        Some((level.repo.clone()?, level.list.selected()?.clone()))
    }

    /// Enter on a session or unmanaged worktree.
    pub(super) fn accept_selected(&mut self) -> Result<Flow> {
        let Some((repo_entry, row)) = self.session_row() else {
            return Ok(Flow::Continue);
        };
        match row.kind {
            RowKind::Session { session, .. } => {
                let Some(current) = self.fresh_session(&repo_entry, &session.id)? else {
                    self.refuse(&format!("Session '{}' no longer exists", session.name));
                    return Ok(Flow::Continue);
                };
                self.run_handler(|env| Ok(select_session(&env.ctx(&repo_entry), &current)))
            }
            RowKind::UnmanagedWorktree { path, branch } => {
                if !self.unmanaged_exists(&repo_entry, &path)? {
                    self.refuse(&format!("Worktree {} no longer exists", path.display()));
                    return Ok(Flow::Continue);
                }
                self.run_handler(|env| {
                    Ok(select_unmanaged(
                        &env.ctx(&repo_entry),
                        &path,
                        branch.as_deref(),
                    ))
                })
            }
            _ => Ok(Flow::Continue),
        }
    }

    /// An action key was pressed on the highlighted row.
    pub(super) fn row_action(&mut self, ctx: RowCtx, spec: &str) -> Result<Flow> {
        match ctx {
            RowCtx::Sessions => self.session_action(spec),
            RowCtx::RepoView => self.repo_action(spec),
            RowCtx::Drill => self.clone_action(),
        }
    }

    fn session_action(&mut self, spec: &str) -> Result<Flow> {
        let Some((repo_entry, row)) = self.session_row() else {
            return Ok(Flow::Continue);
        };
        match row.kind {
            RowKind::Session { session, .. } => {
                let Some(current) = self.fresh_session(&repo_entry, &session.id)? else {
                    self.refuse(&format!("Session '{}' no longer exists", session.name));
                    return Ok(Flow::Continue);
                };
                self.run_handler(|env| {
                    Ok(
                        handle_session_action(&env.ctx(&repo_entry), spec, &current)?
                            .unwrap_or_default(),
                    )
                })
            }
            RowKind::UnmanagedWorktree { path, branch }
                if spec == self.config.keybinds.delete_session =>
            {
                if !self.unmanaged_exists(&repo_entry, &path)? {
                    self.refuse(&format!("Worktree {} no longer exists", path.display()));
                    return Ok(Flow::Continue);
                }
                self.run_handler(|env| {
                    delete_unmanaged(&env.ctx(&repo_entry), &path, branch.as_deref())
                })
            }
            _ => Ok(Flow::Continue),
        }
    }

    /// Label editing and removal in the repo view.
    fn repo_action(&mut self, spec: &str) -> Result<Flow> {
        let Some(row) = self.nav.top().list.selected().cloned() else {
            return Ok(Flow::Continue);
        };
        let RowKind::Repo {
            entry: Some(entry), ..
        } = row.kind
        else {
            return Ok(Flow::Continue);
        };
        let index = repo::store::load_index()?;
        let Some(current) = fresh::find_repo(&index, &entry.id) else {
            self.refuse(&format!("Repo '{}' no longer exists", entry.name));
            return Ok(Flow::Continue);
        };
        let kb = &self.config.keybinds;
        if spec == kb.edit_labels {
            self.run_handler(|env| repo_actions::edit_labels(env.selector, &current))
        } else if spec == kb.remove_repo {
            self.run_handler(|env| repo_actions::remove(env.selector, &current))
        } else {
            Ok(Flow::Continue)
        }
    }

    /// Clone a repo into the directory being browsed, then open its sessions.
    fn clone_action(&mut self) -> Result<Flow> {
        let Source::Drill(dir) = self.nav.top().source.clone() else {
            return Ok(Flow::Continue);
        };
        self.freeze_frame()?;
        let url = self.selector.input("Git URL to clone", None)?;
        if url.trim().is_empty() {
            return Ok(Flow::Continue);
        }
        let target = repo_actions::clone_target(&dir, &url);
        log::debug!("tui: cloning {} into {}", url.trim(), target.display());
        // git needs the real terminal (progress, credential prompts).
        let mut result = Ok(());
        self.shared
            .run_released(&mut || result = repo::clone_repo(url.trim(), Some(&target)))?;
        match result {
            Ok(()) => {
                self.refresh();
                self.push_sessions(&crate::paths::normalize(&target));
            }
            Err(e) => self.notice = Some((format!("Clone failed: {e}"), Severity::Error)),
        }
        Ok(Flow::Continue)
    }

    fn fresh_tree(&self, repo_entry: &RepoEntry) -> Result<SessionTree> {
        session::store::load_sessions(&repo_entry.id)
    }

    fn fresh_session(&self, repo_entry: &RepoEntry, id: &str) -> Result<Option<Session>> {
        Ok(fresh::find_session(&self.fresh_tree(repo_entry)?, id))
    }

    fn unmanaged_exists(&self, repo_entry: &RepoEntry, path: &Path) -> Result<bool> {
        Ok(fresh::unmanaged_exists(&self.fresh_tree(repo_entry)?, path))
    }

    /// Paint the current screen and keep it as the backdrop modals are drawn over.
    fn freeze_frame(&mut self) -> Result<()> {
        let shared = Rc::clone(&self.shared);
        let mut term = shared.borrow_mut();
        let frame = term.terminal().draw(|f| self.draw(f))?;
        self.core.set_backdrop(frame.buffer.clone());
        Ok(())
    }

    /// Enter on a plugin view item: run `OnViewSelect` and apply its response.
    pub(super) fn plugin_select(&mut self, info: &PluginViewInfo, row: &Row) -> Result<Flow> {
        self.run_handler(|env| {
            let response = plugin::run_view_select_hook(
                &info.plugin_name,
                &info.view_name,
                &row.value,
                &row.display,
                env.config,
            )?;
            Ok(view_select_effects(&response, env.post_cmd_file.is_some()))
        })
    }

    /// Run a domain handler with output captured, then apply its effects.
    fn run_handler<F>(&mut self, work: F) -> Result<Flow>
    where
        F: FnOnce(&Env<'_>) -> Result<Vec<Effect>>,
    {
        self.freeze_frame()?;
        self.core.capture.begin()?;
        let selector = Rc::clone(&self.selector);
        let config = Arc::clone(&self.config);
        let binds = Rc::clone(&self.plugin_binds);
        let result = {
            let env = Env {
                selector: &*selector,
                config: &config,
                binds: &binds,
                post_cmd_file: self.outputs.post_cmd_file.as_deref(),
            };
            work(&env)
        };
        let captured = self.core.capture.end();
        let mut effects = match result {
            Ok(effects) => effects,
            Err(EzError::Interrupted) => return Err(EzError::Interrupted),
            Err(EzError::Cancelled) => Vec::new(),
            Err(e) => {
                log::debug!("tui: action failed: {e}");
                vec![
                    Effect::Notify(e.to_string(), Severity::Error),
                    Effect::Refresh(crate::browser::domain::RefreshScope::Sessions),
                ]
            }
        };
        let last_line = captured.lines().rev().find(|l| !l.trim().is_empty());
        if let Some(line) = last_line {
            log::debug!("tui: handler output: {captured:?}");
            // Handlers already report their result; surface stray output only
            // when they did not (for example a validation message).
            if !effects.iter().any(|e| matches!(e, Effect::Notify(..))) {
                effects.push(Effect::Notify(line.trim().to_string(), Severity::Warn));
            }
        }
        self.apply(&effects)
    }

    fn apply(&mut self, effects: &[Effect]) -> Result<Flow> {
        let applied = {
            let target = Target {
                cd_file: self.outputs.cd_file.as_deref(),
                post_cmd_file: self.outputs.post_cmd_file.as_deref(),
                config: &self.config,
            };
            effects::apply(effects, &self.shared, &target)?
        };
        self.after_exit.extend(applied.stdout_after);
        if let Some((_, level)) = applied.notices.last() {
            let text: Vec<&str> = applied.notices.iter().map(|(t, _)| t.as_str()).collect();
            self.notice = Some((text.join("  "), *level));
        }
        if applied.refresh {
            self.refresh();
        }
        Ok(if applied.exit {
            Flow::Quit
        } else {
            Flow::Continue
        })
    }
}
