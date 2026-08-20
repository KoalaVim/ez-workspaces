## 1. Worktree Detection

- [x] 1.1 Add `UnmanagedWorktree` struct (path: `PathBuf`, branch: `Option<String>`) and `list_unmanaged_worktrees(repo_entry, session_tree) -> Vec<UnmanagedWorktree>` function in `src/session/mod.rs`
- [x] 1.2 Implement `git worktree list --porcelain` parsing: extract worktree path and branch (or detached HEAD short SHA) from each block
- [x] 1.3 Filter results: exclude main repo path, exclude paths matching any `session.path` in the tree, exclude paths that don't exist on disk (prunable)
- [x] 1.4 Skip detection for non-git repos (`repo_entry.is_git == false`)

## 2. Inline Registration

- [x] 2.1 Add `register_worktree_inline(repo_id, worktree_path, branch_name) -> Result<Session>` function in `src/session/mod.rs` that creates a session with the worktree path and branch as plugin_state, parented to the default session, and saves it
- [x] 2.2 Handle duplicate detection: if a session with the same name already exists, append a numeric suffix

## 3. Browser Integration

- [x] 3.1 In `session_action_loop` (`src/browser/mod.rs`), call `list_unmanaged_worktrees` after building managed session items
- [x] 3.2 Append a non-interactive "Not Registered" header item and dimmed worktree items to the fzf select list
- [x] 3.3 Handle selection of non-managed worktree: call `register_worktree_inline`, then run `accept_session` with the `on_enter` action
- [x] 3.4 Handle selection of the "Not Registered" header: no-op, continue the loop
- [x] 3.5 Handle keybind actions on non-managed items: ignore session-specific actions (delete, rename, labels, notes), continue the loop

## 4. Testing & Polish

- [x] 4.1 Add unit tests for `git worktree list --porcelain` parsing (multiple worktrees, detached HEAD, prunable entries)
- [x] 4.2 Verify `make check` passes (fmt, clippy, tests) with zero warnings
- [x] 4.3 Update `AGENTS.md` with new function signatures
- [x] 4.4 Update `docs/user-guide.md` with "Not Registered" section description
