//! Start-point selection for sessions created under the default session.
//!
//! By default the git-worktree plugin starts children of the default session
//! from `origin/main` / `origin/master`. When the default checkout sits on some
//! other branch (e.g. a long-lived fork line), offer to base the new session on
//! that branch instead, by setting the `ez_start_point` override.

use std::collections::HashMap;
use std::path::Path;

use super::git_output;
use super::model::SessionTree;
use crate::error::Result;

/// Session env key the git-worktree plugin reads as an explicit start point.
pub(crate) const START_POINT_KEY: &str = "ez_start_point";

/// Branches that keep the plugin's default `origin/<branch>` behavior.
const MAINLINE_BRANCHES: &[&str] = &["main", "master"];

/// If the new session's parent is the default session and the default checkout
/// is on a branch other than main/master, ask whether to start the new session
/// from that branch. On yes, records it as `ez_start_point` in `env`.
///
/// No-op when a start point is already set (PR checkout, from-dirty), when the
/// branch `name` already exists (it is reused, so the start point is moot), when
/// the parent is not the default session, or when the default checkout is
/// detached or on main/master.
pub(crate) fn offer_default_branch_base(
    repo_path: &Path,
    tree: &SessionTree,
    parent_id: Option<&str>,
    name: &str,
    env: &mut HashMap<String, String>,
) -> Result<()> {
    if env.contains_key(START_POINT_KEY) {
        log::debug!("base: start point already set, not offering default branch");
        return Ok(());
    }
    if crate::browser::branch_exists(repo_path, name) {
        log::debug!("base: branch '{name}' exists and will be reused, start point moot");
        return Ok(());
    }
    // Mirror the plugin: no parent and a default parent both mean "start from
    // the mainline"; any other parent means "start from the parent's HEAD".
    let default = tree.find_default();
    if let Some(id) = parent_id {
        if default.is_none_or(|d| d.id != id) {
            log::debug!("base: parent is not the default session, plugin uses parent HEAD");
            return Ok(());
        }
    }

    let default_path = default.and_then(|d| d.path.as_deref()).unwrap_or(repo_path);
    let Ok(branch) = git_output(default_path, &["symbolic-ref", "--short", "-q", "HEAD"]) else {
        log::debug!("base: default checkout is detached, keeping origin/main|master");
        return Ok(());
    };
    if MAINLINE_BRANCHES.contains(&branch.as_str()) {
        log::debug!("base: default checkout on '{branch}', keeping origin/{branch}");
        return Ok(());
    }

    let use_branch = crate::browser::selector::confirm_prompt(
        &format!(
            "The default session is on '{branch}', not main/master.\n  \
             [Y] start '{name}' from '{branch}'  (default)\n  \
             [n] start '{name}' from origin/main|master\n\
             Base on '{branch}'?"
        ),
        true,
    )?;
    log::debug!("base: default checkout on '{branch}', use it as base: {use_branch}");
    if use_branch {
        env.insert(START_POINT_KEY.to_string(), branch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::process::Command;
    use std::rc::Rc;

    use chrono::Utc;

    use super::*;
    use crate::browser::selector::set_confirm_override;
    use crate::session::model::Session;

    fn git(dir: &Path, args: &[&str]) {
        let ok = Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .current_dir(dir)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?} failed");
    }

    /// A repo with one commit, checked out on `branch`.
    fn repo_on(branch: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        git(dir.path(), &["init", "-q", "-b", branch]);
        git(dir.path(), &["commit", "-q", "--allow-empty", "-m", "init"]);
        dir
    }

    fn session(id: &str, is_default: bool) -> Session {
        Session {
            id: id.into(),
            name: id.into(),
            parent_id: None,
            path: None,
            env: HashMap::new(),
            plugin_state: HashMap::new(),
            labels: Vec::new(),
            created_at: Utc::now(),
            is_default,
            bare: false,
            last_accessed: None,
        }
    }

    fn tree() -> SessionTree {
        SessionTree {
            sessions: vec![session("default", true), session("other", false)],
        }
    }

    /// Runs the offer with a confirm hook answering `answer`; returns the
    /// resulting env and whether the prompt was shown.
    fn offer(repo: &Path, parent: Option<&str>, answer: bool) -> (HashMap<String, String>, bool) {
        let asked = Rc::new(Cell::new(false));
        let flag = asked.clone();
        set_confirm_override(Some(Rc::new(move |_, _| {
            flag.set(true);
            Ok(answer)
        })));
        let mut env = HashMap::new();
        offer_default_branch_base(repo, &tree(), parent, "new-sess", &mut env).unwrap();
        set_confirm_override(None);
        (env, asked.get())
    }

    #[test]
    fn non_mainline_default_branch_is_offered_and_used() {
        let repo = repo_on("fork-v1");
        let (env, asked) = offer(repo.path(), Some("default"), true);
        assert!(asked);
        assert_eq!(
            env.get(START_POINT_KEY).map(String::as_str),
            Some("fork-v1")
        );
    }

    #[test]
    fn declining_keeps_plugin_default() {
        let repo = repo_on("fork-v1");
        let (env, asked) = offer(repo.path(), None, false);
        assert!(asked);
        assert!(env.is_empty());
    }

    #[test]
    fn mainline_default_branch_is_not_offered() {
        for branch in ["main", "master"] {
            let repo = repo_on(branch);
            let (env, asked) = offer(repo.path(), Some("default"), true);
            assert!(!asked, "{branch}");
            assert!(env.is_empty(), "{branch}");
        }
    }

    #[test]
    fn non_default_parent_is_not_offered() {
        let repo = repo_on("fork-v1");
        let (env, asked) = offer(repo.path(), Some("other"), true);
        assert!(!asked);
        assert!(env.is_empty());
    }

    #[test]
    fn existing_branch_or_start_point_is_not_offered() {
        let repo = repo_on("fork-v1");
        git(repo.path(), &["branch", "new-sess"]);
        let (_, asked) = offer(repo.path(), Some("default"), true);
        assert!(!asked);

        let repo = repo_on("fork-v1");
        let mut env = HashMap::from([(START_POINT_KEY.to_string(), "origin/pr".to_string())]);
        offer_default_branch_base(repo.path(), &tree(), None, "new-sess", &mut env).unwrap();
        assert_eq!(env[START_POINT_KEY], "origin/pr");
    }
}
