//! Front-end independent browser logic.
//!
//! The fzf front end (and the future native TUI) share this layer:
//!
//! - [`rows`] / [`session_rows`]: build the rows each view shows (`Row`).
//! - [`preview`]: render preview panes into a `String` (ANSI).
//! - [`actions`]: session/worktree action handlers that return `Vec<Effect>`
//!   instead of writing files or exiting. Each front end applies the effects.
//!
//! Interactive sub-flows (name builder, confirm, input) stay pull-style and
//! take `&dyn InteractiveSelector` inside handlers.

/// `println!` into a `String` (or any `fmt::Write`); writing to a `String` cannot fail.
macro_rules! outln {
    ($out:expr) => {{
        let _ = writeln!($out);
    }};
    ($out:expr, $($arg:tt)*) => {{
        let _ = writeln!($out, $($arg)*);
    }};
}

pub(crate) mod accept;
pub(crate) mod actions;
pub(crate) mod fresh;
mod keybind_help;
pub(crate) mod plugin_view;
pub(crate) mod preview;
mod preview_session;
mod preview_slow;
pub(crate) mod repo_actions;
pub(crate) mod rows;
pub(crate) mod session_rows;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use crate::repo::model::RepoEntry;
use crate::session::model::Session;

use super::selector::SelectItem;

/// One selectable line in a view.
#[derive(Clone, Debug)]
pub struct Row {
    /// What the user sees (may contain ANSI colors).
    pub display: String,
    /// Stable identifier (path, session id, owner, ...), also handed to preview commands.
    pub value: String,
    /// What dispatch needs once the row is chosen.
    pub kind: RowKind,
}

/// The payload a row carries so dispatch needs no re-lookup.
#[derive(Clone, Debug)]
pub enum RowKind {
    /// A configured workspace root.
    Root(PathBuf),
    /// A plain (non-repo) directory.
    Directory(PathBuf),
    /// A repository; `entry` is `None` when it is not registered.
    Repo {
        path: PathBuf,
        entry: Option<RepoEntry>,
    },
    /// A managed session.
    Session {
        repo: RepoEntry,
        session: Box<Session>,
        target_dir: PathBuf,
    },
    /// A git worktree that is not registered as a session.
    UnmanagedWorktree {
        path: PathBuf,
        branch: Option<String>,
    },
    /// Non-selectable separator line.
    Header,
    /// An owner group and the repos it contains.
    Owner { repos: Vec<RepoEntry> },
    /// A label group and the rows it contains.
    Label { children: Vec<Row> },
    /// An item returned by a plugin view.
    PluginItem,
    /// A row restored from the snapshot (display only until fresh rows replace
    /// it); the payload is the kind tag it was saved with.
    Cached(String),
}

impl From<&Row> for SelectItem {
    fn from(row: &Row) -> Self {
        SelectItem {
            display: row.display.clone(),
            value: row.value.clone(),
        }
    }
}

/// Convert rows into fzf selector items.
pub(crate) fn to_items(rows: &[Row]) -> Vec<SelectItem> {
    rows.iter().map(SelectItem::from).collect()
}

/// Severity of a [`Effect::Notify`] message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

/// What needs refreshing after an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshScope {
    /// The session list of the current repo.
    Sessions,
    /// Every list that shows registered repos or their labels.
    Repos,
}

/// A side effect requested by an action handler. Each front end applies them
/// (see `browser::effects` for the fzf executor).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Make the invoking shell cd here (written to `cd_file`).
    Cd(PathBuf),
    /// Commands for the invoking shell to run after ez exits; `relaunch`
    /// asks the shell wrapper to re-invoke ez afterwards.
    PostCmds { cmds: Vec<String>, relaunch: bool },
    /// Shell commands to run inside the ez process right now.
    Exec(Vec<String>),
    /// Open a session's note in the configured editor (blocking, needs the terminal).
    OpenNote { repo_id: String, session_id: String },
    /// Reload rows/preview for the scope.
    Refresh(RefreshScope),
    /// Tell the user something (text may carry ANSI colors).
    Notify(String, Level),
    /// Leave the browser.
    Exit,
}
