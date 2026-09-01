## Why

During an interactive rebase, git detaches HEAD — so `git symbolic-ref --short HEAD` fails and `git worktree list --porcelain` reports `detached` instead of `branch refs/heads/...`. This causes ez-workspaces to show no branch (or a truncated SHA) for sessions that are mid-rebase, even though the branch name is recoverable from git's rebase state files (`rebase-merge/head-name` or `rebase-apply/head-name`).

## What Changes

- When a worktree is detached, check for an active rebase and recover the original branch name from git's rebase state files.
- Apply this recovery in both the `WorktreeInfo` batch path (porcelain parser) and the `BranchCache`/`get_branch` fallback path.
- Optionally indicate the rebase state in the branch display (e.g., `feature|REBASE`).

## Capabilities

### New Capabilities
- `rebase-branch-recovery`: Detect active rebase state in worktrees and recover the original branch name from `rebase-merge/head-name` or `rebase-apply/head-name`.

### Modified Capabilities
- `session-branch-display`: Branch indicator now shows branch name during rebase instead of hiding it or showing a SHA.

## Impact

- `src/session/mod.rs`: `parse_worktree_list_porcelain` and `build_worktree_info` — recover branch from rebase state when worktree reports `detached`.
- `src/browser/mod.rs`: `BranchCache::get_branch` and `get_branch` — fallback to rebase state when `symbolic-ref` fails.
- `src/browser/mod.rs`: `format_branch_indicator` — optionally render rebase suffix.
- No new dependencies.
