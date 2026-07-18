use std::path::{Path, PathBuf};
use std::process::Command;

use colored::Colorize;

use super::model::Session;
use super::store;
use crate::browser::selector::confirm_prompt;
use crate::error::{EzError, Result};
use crate::repo::{self, model::RepoEntry};

pub(crate) struct CurrentSessionTarget {
    pub repo_entry: RepoEntry,
    pub session: Session,
    source: CurrentSessionSource,
}

enum CurrentSessionSource {
    Tmux(PathBuf),
    Worktree(PathBuf),
}

impl CurrentSessionSource {
    fn label(&self) -> &'static str {
        match self {
            Self::Tmux(_) => "tmux @ez_session_path",
            Self::Worktree(_) => "current directory",
        }
    }

    fn path(&self) -> &Path {
        match self {
            Self::Tmux(path) | Self::Worktree(path) => path,
        }
    }
}

pub(crate) fn resolve_current_session(repo_arg: Option<&str>) -> Result<CurrentSessionTarget> {
    let repos = candidate_repos(repo_arg)?;

    // Try tmux @ez_session_name first (set by the tmux plugin) — most precise.
    if let (Some(tmux_repo_id), Some(tmux_session_name)) = (
        tmux_user_option("@ez_repo_id"),
        tmux_user_option("@ez_session_name"),
    ) {
        log::debug!(
            "resolving current session from tmux @ez_session_name: repo={} session={}",
            tmux_repo_id,
            tmux_session_name
        );
        if let Some((repo_entry, session)) =
            find_session_by_name(&repos, &tmux_repo_id, &tmux_session_name)?
        {
            let path = session
                .path
                .clone()
                .unwrap_or_else(|| repo_entry.path.clone());
            return Ok(CurrentSessionTarget {
                repo_entry,
                session,
                source: CurrentSessionSource::Tmux(path),
            });
        }
        log::debug!("tmux @ez_session_name did not match any registered session");
    }

    // Fall back to @ez_session_path matching.
    if let Some(path) = tmux_user_option("@ez_session_path").map(std::path::PathBuf::from) {
        log::debug!(
            "resolving current session from tmux @ez_session_path: {}",
            path.display()
        );
        if let Some((repo_entry, session)) = find_session_by_path(&repos, &path)? {
            return Ok(CurrentSessionTarget {
                repo_entry,
                session,
                source: CurrentSessionSource::Tmux(path),
            });
        }
        log::debug!(
            "tmux @ez_session_path did not match any registered session: {}",
            path.display()
        );
    }

    let cwd = std::env::current_dir()?;
    log::debug!(
        "resolving current session from current directory: {}",
        cwd.display()
    );
    if let Some((repo_entry, session)) = find_session_by_path(&repos, &cwd)? {
        return Ok(CurrentSessionTarget {
            repo_entry,
            session,
            source: CurrentSessionSource::Worktree(cwd),
        });
    }

    Err(EzError::SessionNotFound(
        "current session (tmux @ez_session_path and current directory did not match any registered session)".into(),
    ))
}

pub(crate) fn confirm_delete_current_session(target: &CurrentSessionTarget) -> Result<()> {
    let session_path = target
        .session
        .path
        .as_deref()
        .unwrap_or(target.repo_entry.path.as_path());
    let prompt = format!(
        "{} {}
{} {}
{} {}
{} {}
{}",
        "Delete current session?".yellow().bold(),
        target.session.name.bold(),
        "Repo:".cyan(),
        target.repo_entry.name.bold(),
        "Detected by:".cyan(),
        target.source.label(),
        "Matched path:".cyan(),
        target.source.path().display(),
        format!("Session path: {}", session_path.display()).dimmed()
    );

    if confirm_prompt(&prompt, false)? {
        Ok(())
    } else {
        Err(EzError::Cancelled)
    }
}

fn candidate_repos(repo_arg: Option<&str>) -> Result<Vec<RepoEntry>> {
    match repo_arg {
        Some(arg) => Ok(vec![repo::resolve_repo(Some(arg))?]),
        None => Ok(repo::store::load_index()?.repos),
    }
}

fn tmux_user_option(option: &str) -> Option<String> {
    std::env::var_os("TMUX")?;

    let output = match Command::new("tmux")
        .args(["show-options", "-v", "-q", option])
        .output()
    {
        Ok(output) => output,
        Err(err) => {
            log::debug!("failed to run tmux while reading {option}: {err}");
            return None;
        }
    };

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn find_session_by_name(
    repos: &[RepoEntry],
    repo_id: &str,
    session_name: &str,
) -> Result<Option<(RepoEntry, Session)>> {
    for repo_entry in repos {
        if repo_entry.id != repo_id {
            continue;
        }
        let tree = store::load_sessions(&repo_entry.id)?;
        if let Some(session) = tree.find_by_name(session_name) {
            return Ok(Some((repo_entry.clone(), session.clone())));
        }
    }
    Ok(None)
}

fn find_session_by_path(repos: &[RepoEntry], path: &Path) -> Result<Option<(RepoEntry, Session)>> {
    let current_path = normalize_path(path);
    let mut best: Option<(RepoEntry, Session, usize)> = None;

    for repo_entry in repos {
        let tree = store::load_sessions(&repo_entry.id)?;
        for session in tree.sessions {
            let Some(session_path) = session_path(&session, repo_entry) else {
                continue;
            };
            let normalized_session_path = normalize_path(session_path);
            if path_matches_current(&current_path, &normalized_session_path) {
                let depth = normalized_session_path.components().count();
                if best
                    .as_ref()
                    .map(|(_, _, best_depth)| depth > *best_depth)
                    .unwrap_or(true)
                {
                    best = Some((repo_entry.clone(), session, depth));
                }
            }
        }
    }

    Ok(best.map(|(repo_entry, session, _)| (repo_entry, session)))
}

fn session_path<'a>(session: &'a Session, repo_entry: &'a RepoEntry) -> Option<&'a Path> {
    session
        .path
        .as_deref()
        .or_else(|| session.is_default.then_some(repo_entry.path.as_path()))
}

fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn path_matches_current(current_path: &Path, session_path: &Path) -> bool {
    current_path == session_path || current_path.starts_with(session_path)
}

#[cfg(test)]
mod tests {
    use super::path_matches_current;
    use std::path::Path;

    #[test]
    fn path_match_accepts_session_root() {
        assert!(path_matches_current(
            Path::new("/tmp/repo-worktree"),
            Path::new("/tmp/repo-worktree")
        ));
    }

    #[test]
    fn path_match_accepts_descendant_of_session_root() {
        assert!(path_matches_current(
            Path::new("/tmp/repo-worktree/src/module"),
            Path::new("/tmp/repo-worktree")
        ));
    }

    #[test]
    fn path_match_rejects_common_prefix_that_is_not_parent() {
        assert!(!path_matches_current(
            Path::new("/tmp/repo-worktree-extra"),
            Path::new("/tmp/repo-worktree")
        ));
    }
}
