pub mod domain;
mod drill;
mod effects;
mod format;
mod git;
pub mod selector;
mod session_loop;
mod tui;
pub mod views;

use std::path::Path;

use crate::config;
use crate::config::model::SelectorBackend;
use crate::error::{EzError, Result};
use crate::repo;
use crate::session;
use selector::{FzfSelector, InteractiveSelector};

pub(crate) use domain::accept::on_create_is_noop;
pub use domain::preview::preview;
pub(crate) use drill::drill_into_directory;
pub(crate) use effects::{accept_session, write_cd_target, write_post_commands};
pub(crate) use format::parse_label_input;
use git::detect_repo_root;
pub(crate) use git::{
    branch_exists, git_run, is_dirty, recover_rebase_branch, resolve_gitdir, BranchCache,
};
use session_loop::session_action_loop;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SortMode {
    Alpha,
    Lru,
}

impl SortMode {
    pub fn toggle(self) -> Self {
        match self {
            SortMode::Alpha => SortMode::Lru,
            SortMode::Lru => SortMode::Alpha,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SortMode::Alpha => "A-Z",
            SortMode::Lru => "LRU",
        }
    }

    pub fn from_config(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "lru" => SortMode::Lru,
            _ => SortMode::Alpha,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SessionLoopResult {
    Accepted,
    Cancelled,
    ViewAll,
}

pub struct BrowseOptions<'a> {
    pub cd_file: Option<&'a Path>,
    pub post_cmd_file: Option<&'a Path>,
    pub workspace: Option<&'a str>,
    pub repo_flag: Option<&'a Path>,
    pub select_by: Option<&'a str>,
    pub all: bool,
    pub on_enter: Option<&'a str>,
    pub on_create: Option<&'a str>,
    pub no_color: bool,
}

/// Main interactive browser entry point (bare `ez` command).
pub fn browse(options: BrowseOptions<'_>) -> Result<()> {
    let BrowseOptions {
        cd_file,
        post_cmd_file,
        workspace,
        repo_flag,
        select_by,
        all,
        on_enter,
        on_create,
        no_color,
    } = options;
    let mut config = config::load()?;
    if let Some(v) = on_enter {
        config.on_enter = v.into();
        if on_create.is_none() {
            config.on_create = v.into();
        }
    }
    if let Some(v) = on_create {
        config.on_create = v.into();
    }
    match SelectorBackend::parse(&config.selector.backend)? {
        SelectorBackend::Fzf => {}
        SelectorBackend::Native => {
            log::debug!("browse: native backend");
            let entry = tui::Entry {
                select_by,
                workspace,
                repo: repo_flag,
                all,
                no_color,
            };
            return tui::run(&config, entry, cd_file, post_cmd_file);
        }
    }
    let selector = FzfSelector::new(&config.fzf)?;
    let branch_cache = BranchCache::new();

    // --repo: jump straight to session picker for a specific repo
    if let Some(repo_path) = repo_flag {
        let repo_path = if repo_path.is_absolute() {
            repo_path.to_path_buf()
        } else {
            std::env::current_dir()?.join(repo_path)
        };
        if browse_repo(
            &repo_path,
            &selector,
            cd_file,
            post_cmd_file,
            &config,
            &branch_cache,
        )? {
            return Ok(());
        }
        // Esc/cancel: fall through to global view
    }

    // Auto-detect: if cwd is inside a registered repo (or one of its worktrees),
    // jump straight to that repo's session picker. Skipped with --all.
    if !all {
        if let Some(repo_root) = detect_repo_root() {
            let index = repo::store::load_index()?;
            if let Some(entry) = index.find_by_path(&repo_root) {
                match session_action_loop(
                    entry,
                    &selector,
                    cd_file,
                    post_cmd_file,
                    &config,
                    &branch_cache,
                )? {
                    SessionLoopResult::Accepted => return Ok(()),
                    SessionLoopResult::Cancelled | SessionLoopResult::ViewAll => {}
                }
            }
        }
    }

    // Decide starting view: CLI flag > config default > Workspace.
    let mode = match select_by {
        Some(v) => views::ViewMode::from_flag(v, &config)?,
        None => views::ViewMode::from_flag(&config.default_select_by, &config)?,
    };

    views::run(
        mode,
        &selector,
        &config,
        workspace,
        cd_file,
        post_cmd_file,
        &branch_cache,
    )
}

/// Find the registry entry for `repo_path`, registering the repo if needed.
/// A path that is a session worktree resolves to its owning repo.
pub(crate) fn resolve_repo_entry(repo_path: &Path) -> Result<repo::model::RepoEntry> {
    let index = repo::store::load_index()?;
    let repo_entry = if let Some(entry) = index.find_by_path(repo_path) {
        entry.clone()
    } else if let Some(owner) = find_repo_owning_session_path(&index, repo_path) {
        log::debug!(
            "browse_repo: path {} is a session worktree under repo '{}', skipping registration",
            repo_path.display(),
            owner.name
        );
        owner
    } else {
        repo::add_repo(Some(repo_path))?;
        let index = repo::store::load_index()?;
        index.find_by_path(repo_path).cloned().ok_or_else(|| {
            EzError::Path(format!(
                "registered {} but could not find it in the index",
                repo_path.display()
            ))
        })?
    };
    Ok(repo_entry)
}

/// Register repo if needed and enter session action loop.
/// Returns `true` when a session was accepted (caller should exit the browser),
/// `false` when the user cancelled (caller should go back).
pub(crate) fn browse_repo(
    repo_path: &Path,
    selector: &dyn InteractiveSelector,
    cd_file: Option<&Path>,
    post_cmd_file: Option<&Path>,
    config: &config::model::EzConfig,
    branch_cache: &BranchCache,
) -> Result<bool> {
    let repo_entry = resolve_repo_entry(repo_path)?;

    match session_action_loop(
        &repo_entry,
        selector,
        cd_file,
        post_cmd_file,
        config,
        branch_cache,
    )? {
        SessionLoopResult::Accepted => Ok(true),
        SessionLoopResult::Cancelled | SessionLoopResult::ViewAll => Ok(false),
    }
}

/// Check if a path is already tracked as a session worktree under any registered repo.
/// Returns the owning repo entry if found.
fn find_repo_owning_session_path(
    index: &repo::model::RepoIndex,
    path: &Path,
) -> Option<repo::model::RepoEntry> {
    let canonical = crate::paths::normalize(path);
    for entry in &index.repos {
        if let Ok(tree) = session::store::load_sessions(&entry.id) {
            for s in &tree.sessions {
                if let Some(ref session_path) = s.path {
                    let matches = crate::paths::normalize(session_path) == canonical;
                    if matches {
                        log::debug!(
                            "find_repo_owning_session_path: {} matches session '{}' in repo '{}'",
                            path.display(),
                            s.name,
                            entry.name
                        );
                        return Some(entry.clone());
                    }
                }
            }
        }
    }
    None
}
