//! Git helpers shared by the browser, views, previews and session code.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

/// Branch lookups queued by a lazy cache (see [`BranchCache::lazy`]).
#[derive(Default)]
struct Queue {
    waiting: Vec<PathBuf>,
    scheduled: HashSet<PathBuf>,
}

pub(crate) struct BranchCache {
    cache: Mutex<HashMap<PathBuf, (Option<String>, SystemTime)>>,
    /// Lazy caches never run git from `get_branch`; misses are queued instead.
    lazy: bool,
    queue: Mutex<Queue>,
}

impl BranchCache {
    pub(crate) fn new() -> Self {
        BranchCache {
            cache: Mutex::new(HashMap::new()),
            lazy: false,
            queue: Mutex::new(Queue::default()),
        }
    }

    /// A cache whose `get_branch` returns what is known (possibly stale, or
    /// `None`) and queues the lookup for [`take_pending`](Self::take_pending),
    /// so row building never waits on git. Workers call [`resolve`](Self::resolve).
    pub(crate) fn lazy() -> Self {
        BranchCache {
            lazy: true,
            ..Self::new()
        }
    }

    pub(crate) fn get_branch(&self, path: &Path) -> Option<String> {
        if self.lazy {
            self.peek(path)
        } else {
            self.resolve(path)
        }
    }

    /// Paths whose branch has to be resolved; each path is handed out once
    /// until its [`resolve`](Self::resolve) finished.
    pub(crate) fn take_pending(&self) -> Vec<PathBuf> {
        std::mem::take(&mut self.queue.lock().unwrap().waiting)
    }

    fn peek(&self, path: &Path) -> Option<String> {
        let mtime = Self::resolve_head_path(path)
            .and_then(|head| std::fs::metadata(head).ok())
            .and_then(|m| m.modified().ok())?;
        let cached = self.cache.lock().unwrap().get(path).cloned();
        match cached {
            Some((branch, cached_mtime)) if cached_mtime == mtime => branch,
            stale => {
                let mut queue = self.queue.lock().unwrap();
                if queue.scheduled.insert(path.to_path_buf()) {
                    queue.waiting.push(path.to_path_buf());
                }
                stale.and_then(|(branch, _)| branch)
            }
        }
    }

    /// Look the branch up (runs git on a miss) and cache it.
    pub(crate) fn resolve(&self, path: &Path) -> Option<String> {
        let result = self.lookup(path);
        self.queue.lock().unwrap().scheduled.remove(path);
        result
    }

    fn lookup(&self, path: &Path) -> Option<String> {
        let head_path = Self::resolve_head_path(path)?;
        let mtime = std::fs::metadata(&head_path).ok()?.modified().ok()?;

        {
            let cache = self.cache.lock().unwrap();
            if let Some((cached_branch, cached_mtime)) = cache.get(path) {
                if *cached_mtime == mtime {
                    log::debug!("BranchCache: hit for {}", path.display());
                    return cached_branch.clone();
                }
            }
        }

        log::debug!("BranchCache: miss for {}", path.display());
        let branch = git_cmd(path, &["symbolic-ref", "--short", "HEAD"]).or_else(|| {
            let gitdir = resolve_gitdir(path)?;
            recover_rebase_branch(&gitdir)
        });
        self.cache
            .lock()
            .unwrap()
            .insert(path.to_path_buf(), (branch.clone(), mtime));
        branch
    }

    fn resolve_head_path(path: &Path) -> Option<PathBuf> {
        resolve_gitdir(path).map(|gd| gd.join("HEAD"))
    }
}

/// Resolves the `.git` directory for a repository or worktree at `path`.
///
/// If `path/.git` is a file (worktree layout), reads the `gitdir: ...`
/// pointer inside it. If it is a directory, returns it directly.
pub(crate) fn resolve_gitdir(path: &Path) -> Option<PathBuf> {
    let dot_git = path.join(".git");
    if dot_git.is_file() {
        // Worktree: .git is a file containing "gitdir: /path/to/.git/worktrees/<name>"
        let content = std::fs::read_to_string(&dot_git).ok()?;
        let gitdir = content.strip_prefix("gitdir: ")?.trim();
        let gitdir_path = if PathBuf::from(gitdir).is_absolute() {
            PathBuf::from(gitdir)
        } else {
            path.join(gitdir)
        };
        Some(gitdir_path)
    } else if dot_git.is_dir() {
        Some(dot_git)
    } else {
        None
    }
}

/// Recovers the original branch name during a rebase from git's rebase
/// state files (`rebase-merge/head-name` for interactive rebases, or
/// `rebase-apply/head-name` for non-interactive ones), since HEAD is
/// detached while a rebase is in progress.
///
/// Returns `Some("branch-name|REBASE")` if a rebase is in progress and the
/// original branch name can be recovered, otherwise `None`.
pub(crate) fn recover_rebase_branch(gitdir: &Path) -> Option<String> {
    let head_name = gitdir.join("rebase-merge").join("head-name");
    let content = if head_name.is_file() {
        std::fs::read_to_string(&head_name).ok()?
    } else {
        let head_name = gitdir.join("rebase-apply").join("head-name");
        std::fs::read_to_string(&head_name).ok()?
    };
    let branch = content.trim().strip_prefix("refs/heads/")?.to_string();
    Some(format!("{branch}|REBASE"))
}

/// Run a git command and capture stdout.
pub(crate) fn git_cmd(path: &Path, args: &[&str]) -> Option<String> {
    std::process::Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            } else {
                None
            }
        })
}

/// Branch checked out at `path`, read straight from its `HEAD` file (no git
/// process). Falls back to the branch being rebased for a detached `HEAD`.
pub(crate) fn read_head_branch(path: &Path) -> Option<String> {
    let gitdir = resolve_gitdir(path)?;
    let head = std::fs::read_to_string(gitdir.join("HEAD")).ok()?;
    match head.trim().strip_prefix("ref: refs/heads/") {
        Some(branch) => Some(branch.to_string()),
        None => recover_rebase_branch(&gitdir),
    }
}

/// Get the current branch of a git repo.
pub(crate) fn get_branch(path: &Path) -> Option<String> {
    git_cmd(path, &["symbolic-ref", "--short", "HEAD"])
}

/// True if the git worktree at `path` has uncommitted changes.
pub(crate) fn is_dirty(path: &Path) -> bool {
    git_cmd(path, &["status", "--porcelain"])
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

/// Run a git command, returning whether it exited successfully (output is discarded).
pub(crate) fn git_run(path: &Path, args: &[&str]) -> bool {
    std::process::Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// True if a local branch with this exact name exists in the repo at `path`.
pub(crate) fn branch_exists(path: &Path, name: &str) -> bool {
    git_run(
        path,
        &[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{name}"),
        ],
    )
}

/// Resolve the main repository root from the current directory.
/// Works from both the repo itself and any of its worktrees by using
/// `git rev-parse --git-common-dir` which always points to the main
/// repo's `.git` directory.
pub(crate) fn detect_repo_root() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let common_dir = git_cmd(&cwd, &["rev-parse", "--git-common-dir"])?;
    let common_path = PathBuf::from(&common_dir);
    let abs = if common_path.is_absolute() {
        common_path
    } else {
        cwd.join(&common_path)
    };
    // --git-common-dir returns the .git dir; the repo root is its parent
    crate::paths::normalize(&abs)
        .parent()
        .map(|p| p.to_path_buf())
}

#[cfg(test)]
mod rebase_tests {
    use super::*;
    use std::fs;

    #[test]
    fn read_head_branch_follows_worktree_gitdir_and_detached_head() {
        let dir = tempfile::tempdir().unwrap();
        let gitdir = dir.path().join("gitdir");
        fs::create_dir(&gitdir).unwrap();
        let wt = dir.path().join("wt");
        fs::create_dir(&wt).unwrap();
        fs::write(wt.join(".git"), format!("gitdir: {}\n", gitdir.display())).unwrap();

        fs::write(gitdir.join("HEAD"), "ref: refs/heads/feat/x\n").unwrap();
        assert_eq!(read_head_branch(&wt), Some("feat/x".to_string()));

        fs::write(gitdir.join("HEAD"), "0123456789abcdef\n").unwrap();
        assert_eq!(read_head_branch(&wt), None);
    }

    #[test]
    fn recover_rebase_branch_interactive() {
        let dir = tempfile::tempdir().unwrap();
        let rebase_merge = dir.path().join("rebase-merge");
        fs::create_dir(&rebase_merge).unwrap();
        fs::write(rebase_merge.join("head-name"), "refs/heads/feature-x\n").unwrap();
        let result = recover_rebase_branch(dir.path());
        assert_eq!(result, Some("feature-x|REBASE".to_string()));
    }

    #[test]
    fn recover_rebase_branch_non_interactive() {
        let dir = tempfile::tempdir().unwrap();
        let rebase_apply = dir.path().join("rebase-apply");
        fs::create_dir(&rebase_apply).unwrap();
        fs::write(rebase_apply.join("head-name"), "refs/heads/fix-bug\n").unwrap();
        let result = recover_rebase_branch(dir.path());
        assert_eq!(result, Some("fix-bug|REBASE".to_string()));
    }

    #[test]
    fn recover_rebase_branch_no_rebase() {
        let dir = tempfile::tempdir().unwrap();
        let result = recover_rebase_branch(dir.path());
        assert_eq!(result, None);
    }
}

#[cfg(test)]
mod lazy_tests {
    use super::*;

    fn init_repo(dir: &Path) {
        for args in [
            vec!["init", "-q", "-b", "topic"],
            vec![
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                "x",
            ],
        ] {
            assert!(git_run(dir, &args));
        }
    }

    #[test]
    fn lazy_cache_queues_once_and_serves_resolved_values() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        let cache = BranchCache::lazy();
        assert_eq!(cache.get_branch(dir.path()), None);
        assert_eq!(cache.get_branch(dir.path()), None);
        assert_eq!(cache.take_pending(), vec![dir.path().to_path_buf()]);
        assert!(cache.take_pending().is_empty());
        assert_eq!(cache.resolve(dir.path()).as_deref(), Some("topic"));
        assert_eq!(cache.get_branch(dir.path()).as_deref(), Some("topic"));
        assert!(cache.take_pending().is_empty());
    }
}
