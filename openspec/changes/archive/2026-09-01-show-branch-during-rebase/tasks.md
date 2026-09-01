## 1. Shared Helpers

- [x] 1.1 Extract `resolve_gitdir(path: &Path) -> Option<PathBuf>` from `BranchCache::resolve_head_path` in `src/browser/mod.rs` — resolves `.git` (file or dir) to the gitdir path, reuse in `resolve_head_path`
- [x] 1.2 Create `recover_rebase_branch(gitdir: &Path) -> Option<String>` in `src/browser/mod.rs` — checks `rebase-merge/head-name` then `rebase-apply/head-name`, reads the file, strips `refs/heads/`, appends `|REBASE`

## 2. BranchCache Integration

- [x] 2.1 In `BranchCache::get_branch()`, after `git symbolic-ref` returns `None`, call `resolve_gitdir(path)` then `recover_rebase_branch(gitdir)` and cache the result
- [x] 2.2 Verify: start an interactive rebase on a worktree, run `ez --debug`, confirm the branch shows as `(branch-name|REBASE)` and debug log shows recovery

## 3. Batch Path Integration

- [x] 3.1 Add `detached: bool` field to `UnmanagedWorktree` in `src/session/mod.rs`
- [x] 3.2 Set `detached: true` in `parse_worktree_list_porcelain` when the `detached` line is encountered
- [x] 3.3 In `build_worktree_info()`, after building the branches HashMap, iterate entries where the worktree was detached — resolve gitdir and call `recover_rebase_branch()`, replacing the truncated SHA if recovery succeeds

## 4. Testing

- [x] 4.1 Add unit test for `recover_rebase_branch` with mocked rebase-merge/head-name file
- [x] 4.2 Add unit test for `recover_rebase_branch` with mocked rebase-apply/head-name file
- [x] 4.3 Add unit test for `recover_rebase_branch` with no rebase state (returns None)
- [x] 4.4 Manual test: interactive rebase on a session worktree, verify branch shows in session picker with `|REBASE` suffix
- [x] 4.5 Manual test: complete the rebase, verify branch shows normally again (cache invalidated by mtime change)
