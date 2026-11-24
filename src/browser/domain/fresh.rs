//! "Does the target still exist?" checks against fresh data. A front end may
//! show rows from a stale list (a snapshot, or another shell changed things),
//! so actions re-check the target before touching it.

use std::path::Path;

use crate::repo::model::{RepoEntry, RepoIndex};
use crate::session::model::{Session, SessionTree};

/// The current copy of session `id`, if it is still registered.
pub(crate) fn find_session(tree: &SessionTree, id: &str) -> Option<Session> {
    tree.sessions.iter().find(|s| s.id == id).cloned()
}

/// Whether the unmanaged worktree at `path` still exists and is still unregistered.
pub(crate) fn unmanaged_exists(tree: &SessionTree, path: &Path) -> bool {
    path.exists()
        && !tree
            .sessions
            .iter()
            .any(|s| s.path.as_deref() == Some(path))
}

/// The current registry entry for repo `id` (its directory must still exist).
pub(crate) fn find_repo(index: &RepoIndex, id: &str) -> Option<RepoEntry> {
    index
        .repos
        .iter()
        .find(|r| r.id == id && r.path.exists())
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn session(id: &str, path: Option<PathBuf>) -> Session {
        Session {
            id: id.into(),
            name: id.into(),
            parent_id: None,
            path,
            env: HashMap::new(),
            plugin_state: HashMap::new(),
            labels: Vec::new(),
            created_at: Utc::now(),
            is_default: false,
            bare: false,
            last_accessed: None,
        }
    }

    #[test]
    fn deleted_session_is_not_found() {
        let tree = SessionTree {
            sessions: vec![session("a", None)],
            ..Default::default()
        };
        assert_eq!(find_session(&tree, "a").unwrap().id, "a");
        assert!(find_session(&tree, "gone").is_none());
    }

    #[test]
    fn unmanaged_needs_existing_unregistered_path() {
        let dir = tempfile::tempdir().unwrap();
        let wt = dir.path().to_path_buf();
        let empty = SessionTree::default();
        assert!(unmanaged_exists(&empty, &wt));
        assert!(!unmanaged_exists(&empty, &wt.join("missing")));
        let registered = SessionTree {
            sessions: vec![session("s", Some(wt.clone()))],
            ..Default::default()
        };
        assert!(!unmanaged_exists(&registered, &wt));
    }

    #[test]
    fn repo_needs_registry_entry_and_directory() {
        let dir = tempfile::tempdir().unwrap();
        let entry = |id: &str, path: &Path| RepoEntry {
            id: id.into(),
            path: path.to_path_buf(),
            name: id.into(),
            registered_at: Utc::now(),
            is_git: true,
        };
        let index = RepoIndex {
            repos: vec![
                entry("here", dir.path()),
                entry("moved", &dir.path().join("nope")),
            ],
        };
        assert!(find_repo(&index, "here").is_some());
        assert!(find_repo(&index, "moved").is_none());
        assert!(find_repo(&index, "unknown").is_none());
    }
}
