## Context

ez-workspaces resolves branch names for display in two paths:
1. **Batch path**: `build_worktree_info()` calls `git worktree list --porcelain` once per repo, parses it via `parse_worktree_list_porcelain()`, and builds a `HashMap<PathBuf, Option<String>>`. For detached worktrees, it currently stores the truncated SHA.
2. **Fallback path**: `BranchCache::get_branch()` calls `git symbolic-ref --short HEAD` and caches with mtime-based invalidation. Returns `None` when HEAD is detached.

During a rebase, git detaches HEAD — both paths fail to produce a branch name. However, git stores the original branch in `<gitdir>/rebase-merge/head-name` (interactive) or `<gitdir>/rebase-apply/head-name` (non-interactive), containing e.g. `refs/heads/feature-x`.

For worktrees, `<gitdir>` is resolved from the `.git` file (e.g., `.git/worktrees/<name>/`). For the main worktree, it's `.git/`.

## Goals / Non-Goals

**Goals:**
- Show the original branch name when a worktree is mid-rebase, in all views (session picker, tree, repo, owner)
- Append `|REBASE` suffix so the user knows the branch is being rebased
- Zero additional subprocess calls — use filesystem reads only
- Work for both interactive (`rebase -i`) and non-interactive rebase

**Non-Goals:**
- Detecting other detached-HEAD states (cherry-pick, bisect, merge) — these can be added later with the same pattern
- Changing the unmanaged worktree display (already shows `(detached)` — acceptable)

## Decisions

### 1. Shared `recover_rebase_branch()` helper

A single function `recover_rebase_branch(gitdir: &Path) -> Option<String>` checks for `rebase-merge/head-name` then `rebase-apply/head-name`, reads the file, strips `refs/heads/`, and appends `|REBASE`. This function is called from both the batch path and the fallback path.

**Alternative**: Inline the check in each call site. Rejected — same logic needed in 2+ places, and this is a natural unit for testing.

### 2. Gitdir resolution reused from `BranchCache::resolve_head_path`

`BranchCache::resolve_head_path()` already resolves `.git` (file or dir) to the gitdir. Extract the gitdir resolution into a shared helper `resolve_gitdir(path: &Path) -> Option<PathBuf>` that both `resolve_head_path` and the rebase recovery can use.

**Alternative**: Duplicate the `.git` file parsing. Rejected — the logic is already there and non-trivial (handles relative paths).

### 3. Branch name format: `feature-x|REBASE`

The rebase recovery function appends `|REBASE` to the branch name (e.g., `chore-docker|REBASE`). This passes through `format_branch_indicator` unchanged, so the display becomes `(chore-docker|REBASE)`. This mirrors git's own prompt format (`feature-x|REBASE`).

**Alternative**: Separate the rebase indicator from the branch name (e.g., a separate field or color). Rejected — the `|REBASE` suffix is a well-known git convention, and keeping it in the branch string avoids changing signatures throughout the call chain.

### 4. Batch path integration

In `build_worktree_info()`, after `parse_worktree_list_porcelain()` returns, iterate the worktrees. For any entry with `is_detached` (currently stores truncated SHA), resolve its gitdir and call `recover_rebase_branch()`. If recovery succeeds, replace the SHA in the branches HashMap with the recovered name.

This requires `parse_worktree_list_porcelain()` to expose the detached state — either via a flag on `UnmanagedWorktree` or by changing the return type. Simplest: add a `detached: bool` field to `UnmanagedWorktree`.

### 5. BranchCache integration

In `BranchCache::get_branch()`, after `git symbolic-ref` returns `None`, resolve the gitdir and call `recover_rebase_branch()`. The result is cached the same way as normal branches (keyed by path, invalidated by HEAD mtime).

## Risks / Trade-offs

- **[Filesystem reads in hot path]** → Two stat + read calls per detached worktree. Mitigated: only runs when detached (rare), and the reads are tiny files (<100 bytes). Cached by `BranchCache` on subsequent renders.
- **[Stale rebase state]** → If a rebase completes between the `worktree list` call and the rebase file check, the file may not exist. Mitigated: `recover_rebase_branch` returns `None` gracefully, falling back to existing behavior.
- **[Non-standard gitdir layouts]** → The `.git` file parsing handles both absolute and relative gitdir paths (already implemented in `resolve_head_path`). Submodule layouts may differ but are not a target.
