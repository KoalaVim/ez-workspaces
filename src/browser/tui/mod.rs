//! Native (ratatui) front end, enabled with `selector.backend = "native"`.
//!
//! - [`terminal`]: RAII terminal guard, panic hook, suspend/resume.
//! - [`keys`]: fzf key-string parser and key-event normalization.
//! - [`event`]: synchronous poll loop with an `mpsc` results channel.
//! - [`list`]: fuzzy-filtered list state and widget helpers.
//! - [`nav`]: navigation stack (Esc pops, Esc at the root exits).
//! - [`level`]: one stack level (rows source, sort, preview request).
//! - [`preview_pane`]: in-process preview via `ansi-to-tui`, cached.
//! - [`app`]: application state; views, drill-down and session lists.
//! - [`modal`] / `modal_state`: `NativeSelector`, modal prompts over the browser.
//! - [`effects`]: native effect executor (cd/post-cmd files, suspend for exec).
//! - [`capture`]: collects handler stdout/stderr instead of drawing it.
//! - [`stdio`]: keeps all stray stdout/stderr off the alternate screen.
//! - [`workers`]: fixed thread pool for slow work (branches, previews, plugin views).
//! - [`snapshot`]: last shown rows on disk for an instant first frame.

mod app;
mod capture;
mod effects;
mod event;
mod keys;
mod level;
mod list;
mod modal;
mod modal_state;
mod nav;
mod preview_pane;
mod snapshot;
mod stdio;
mod terminal;
mod workers;

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use colored::Colorize;

use self::app::Start;
use self::level::ViewKind;
use super::drill::match_workspace_root;
use super::git::detect_repo_root;
use crate::config::model::EzConfig;
use crate::error::Result;
use crate::repo;

/// The entry-point flags shared with the fzf browser.
pub struct Entry<'a> {
    pub select_by: Option<&'a str>,
    pub workspace: Option<&'a str>,
    pub repo: Option<&'a Path>,
    pub all: bool,
    pub no_color: bool,
}

/// Run the native browser until the user leaves it.
///
/// Esc at the top level returns `Ok(())`; Ctrl-C returns `Err(EzError::Interrupted)`.
pub fn run(
    config: &EzConfig,
    entry: Entry<'_>,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
) -> Result<()> {
    log::debug!(
        "tui::run: cd_file={:?} post_cmd_file={:?}",
        cd_file,
        post_cmd_file
    );
    // `colored` would disable colors for the in-process preview renders when
    // stdout is not a TTY; force them on unless --no-color was given.
    if !entry.no_color {
        colored::control::set_override(true);
    }
    let Some(start) = plan_start(config, &entry)? else {
        return Ok(());
    };
    terminal::install_panic_hook();
    let guard = Rc::new(RefCell::new(terminal::TerminalGuard::enter()?));
    let outputs = app::Outputs {
        cd_file: cd_file.map(Path::to_path_buf),
        post_cmd_file: post_cmd_file.map(Path::to_path_buf),
    };
    let result = app::run(Rc::clone(&guard), config, start, outputs);
    // Restore the terminal before printing anything on stdout.
    drop(guard);
    for line in result? {
        println!("{line}");
    }
    Ok(())
}

/// Resolve flags into the initial navigation stack. `None` means exit now
/// (an error was already printed).
fn plan_start(config: &EzConfig, entry: &Entry<'_>) -> Result<Option<Start>> {
    let view = ViewKind::from_flag(entry.select_by.unwrap_or(&config.default_select_by), config)?;
    let mut drill = None;
    if view == ViewKind::Workspace {
        if let Some(ws) = entry.workspace {
            match match_workspace_root(config, ws) {
                Some(root) => drill = Some(root),
                None => {
                    eprintln!(
                        "{} workspace '{}' not found in config roots",
                        "ez:".red().bold(),
                        ws.trim_end_matches('/')
                    );
                    return Ok(None);
                }
            }
        }
    }
    let sessions = if let Some(path) = entry.repo {
        Some(if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        })
    } else if !entry.all {
        // Inside a registered repo (or one of its worktrees): open its sessions.
        detect_repo_root().and_then(|root| {
            let index = repo::store::load_index().ok()?;
            index.find_by_path(&root).map(|e| e.path.clone())
        })
    } else {
        None
    };
    Ok(Some(Start {
        view,
        drill,
        sessions,
    }))
}
