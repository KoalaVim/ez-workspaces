## 1. Core: Add session-worktree lookup

- [x] 1.1 Add a helper function `find_repo_by_session_path` that scans all repos' sessions and returns the owning `RepoEntry` if any session's path matches the given path
- [x] 1.2 In `browse_repo`, before calling `repo::add_repo`, call the new helper and use the owning repo if found

## 2. Verification

- [x] 2.1 Run `make check` and fix any warnings or errors
