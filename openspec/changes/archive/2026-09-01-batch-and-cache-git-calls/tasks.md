## 1. Debug Logging

- [x] 1.1 Add `.format_timestamp_micros()` to `env_logger::Builder` in `src/main.rs` so debug logs have microsecond precision

## 2. WorktreeInfo — Unified worktree list + branch map

- [x] 2.1 Create `WorktreeInfo` struct in `src/session/mod.rs` holding `branches: HashMap<PathBuf, Option<String>>` and `unmanaged: Vec<UnmanagedWorktree>`
- [x] 2.2 Create `build_worktree_info(repo_entry, session_tree) -> WorktreeInfo` function that calls `git worktree list --porcelain` once, parses it via existing `parse_worktree_list_porcelain`, builds the branch HashMap (canonicalized path → branch), and filters unmanaged worktrees (reusing existing filtering logic from `list_unmanaged_worktrees`)
- [x] 2.3 Add `fn get_branch_for_path(&self, path: &Path) -> Option<String>` method on `WorktreeInfo` that does canonicalized HashMap lookup
- [x] 2.4 Update `session_action_loop` in `src/browser/mod.rs` to call `build_worktree_info` once at the top of the loop body, use `worktree_info.get_branch_for_path()` instead of `get_branch()` for each session, and use `worktree_info.unmanaged` instead of calling `list_unmanaged_worktrees()`
- [ ] 2.5 Verify: run `ez --debug` on a repo with multiple sessions, confirm debug log shows exactly one `git worktree list` call and zero `git symbolic-ref` calls per render cycle

## 3. BranchCache — Mtime-based caching for repo-level branch lookups

- [x] 3.1 Create `BranchCache` struct in `src/browser/mod.rs` wrapping `Mutex<HashMap<PathBuf, (Option<String>, SystemTime)>>`
- [x] 3.2 Implement `BranchCache::new()` and `BranchCache::get_branch(&self, path: &Path) -> Option<String>` that: resolves the HEAD file path (handling worktree `.git` files), stat()s the HEAD file for mtime, returns cached value on mtime match, falls back to `git symbolic-ref` on cache miss or mtime change, and updates the cache entry
- [x] 3.3 Create `BranchCache` in `browse()` and thread it as `&BranchCache` through `session_action_loop`, `browse_repo`, `drill_into_directory`, and the view dispatch functions
- [x] 3.4 Update `get_branch()` call sites in `drill_into_directory` (mod.rs:951) to use `branch_cache.get_branch()`
- [ ] 3.5 Verify: run `ez --debug`, open session picker, toggle sort twice — confirm second toggle shows zero `git symbolic-ref` calls in the debug log (all cache hits)

## 4. Parallel Branch Resolution in Views

- [x] 4.1 Update `src/browser/views/repo.rs` to resolve all repo branches in parallel via `std::thread::scope`, using `branch_cache.get_branch()` inside each thread
- [x] 4.2 Update `src/browser/views/tree.rs` to resolve repo-level branches in parallel via `std::thread::scope`, and session-level branches via `build_worktree_info()` per repo
- [x] 4.3 Update `src/browser/views/owner.rs` to resolve branches for the selected owner's repos in parallel via `std::thread::scope`
- [ ] 4.4 Verify: run `ez --debug` in repo view with 10+ repos, confirm branches are resolved concurrently (overlapping timestamps in debug log, total time ≈ single slowest call)

## 5. Integration Testing

- [ ] 5.1 Test session picker with repo that has 10+ sessions — all branches display correctly
- [ ] 5.2 Test session picker with detached HEAD worktree — branch indicator omitted (not a crash)
- [ ] 5.3 Test session picker with non-git repo — no git calls, sessions render without branches
- [ ] 5.4 Test repo picker with LRU sort — branches display correctly, re-render after sort toggle is fast
- [ ] 5.5 Test tree view — repo branches and session branches display correctly
- [ ] 5.6 Test that unmanaged worktrees still appear in the "Not Registered" section correctly
- [ ] 5.7 Test owner view — branches display correctly after selecting an owner
