## ADDED Requirements

### Requirement: Recover branch name from rebase state

When a git worktree has a detached HEAD due to an active rebase, the system SHALL recover the original branch name by reading git's rebase state files. The system SHALL check for `rebase-merge/head-name` (interactive rebase) and `rebase-apply/head-name` (non-interactive rebase) in the worktree's git directory.

#### Scenario: Interactive rebase in progress

- **WHEN** a worktree's HEAD is detached and `<gitdir>/rebase-merge/head-name` exists containing `refs/heads/feature-x`
- **THEN** the system recovers `feature-x` as the branch name

#### Scenario: Non-interactive rebase in progress

- **WHEN** a worktree's HEAD is detached and `<gitdir>/rebase-apply/head-name` exists containing `refs/heads/fix-bug`
- **THEN** the system recovers `fix-bug` as the branch name

#### Scenario: Detached HEAD without rebase

- **WHEN** a worktree's HEAD is detached and neither `rebase-merge/head-name` nor `rebase-apply/head-name` exists
- **THEN** no branch name is recovered (existing behavior: truncated SHA or None)

#### Scenario: Worktree gitdir resolution

- **WHEN** a worktree's `.git` is a file (standard worktree layout) containing `gitdir: /path/to/.git/worktrees/<name>`
- **THEN** the system checks for rebase state files in `/path/to/.git/worktrees/<name>/rebase-merge/` and `/path/to/.git/worktrees/<name>/rebase-apply/`

#### Scenario: Main worktree rebase

- **WHEN** the main worktree (`.git` is a directory) is mid-rebase
- **THEN** the system checks for rebase state files in `.git/rebase-merge/` and `.git/rebase-apply/`

### Requirement: Rebase recovery in batch worktree path

The `build_worktree_info` function SHALL apply rebase branch recovery for any worktree that `git worktree list --porcelain` reports as `detached`. The recovered branch name SHALL be stored in the `WorktreeInfo.branches` HashMap in place of the truncated SHA.

#### Scenario: Batch resolution with one worktree mid-rebase

- **WHEN** `git worktree list --porcelain` reports 5 worktrees, one of which is `detached`
- **THEN** the detached worktree's branch is recovered from rebase state files and stored in the branches HashMap; the other 4 use their porcelain-reported branches

### Requirement: Rebase recovery in BranchCache fallback

`BranchCache::get_branch` SHALL attempt rebase branch recovery when `git symbolic-ref --short HEAD` fails (returns None). The recovered branch SHALL be cached with the same mtime-based invalidation as normal branches.

#### Scenario: BranchCache miss during rebase

- **WHEN** `git symbolic-ref --short HEAD` fails for a worktree path and the worktree is mid-rebase
- **THEN** the system reads the rebase state file, recovers the branch name, and caches the result
