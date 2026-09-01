# Git Call Batching

## Purpose

Eliminate per-session `git symbolic-ref` subprocess calls in the session picker by extracting branch information from the `git worktree list --porcelain` output that is already being fetched for unmanaged worktree detection.

## Requirements

### Requirement: Worktree info cache provides branches and unmanaged list from a single git call

The system SHALL parse `git worktree list --porcelain` once per session picker render and produce both a `HashMap<PathBuf, Option<String>>` mapping worktree paths to branch names AND the list of unmanaged worktrees. This single git call SHALL replace all per-session `get_branch()` subprocess calls within `session_action_loop`.

#### Scenario: Session branch resolved from worktree cache

- **WHEN** the session picker renders items for a repo with 11 sessions (each with a worktree path)
- **THEN** the system calls `git worktree list --porcelain` exactly once
- **AND** resolves each session's branch via HashMap lookup instead of spawning `git symbolic-ref`

#### Scenario: Default session uses repo root branch from cache

- **WHEN** a default session has no dedicated worktree path and falls back to the repo root
- **THEN** the branch is resolved from the worktree cache using the repo root path (which appears as the first entry in `git worktree list` output)

#### Scenario: Worktree path not in cache

- **WHEN** a session's path does not appear in the `git worktree list` output (e.g. path was moved or deleted)
- **THEN** the branch indicator is omitted (same behavior as current `get_branch()` returning `None`)

#### Scenario: Detached HEAD worktree in cache

- **WHEN** a worktree has a detached HEAD
- **THEN** the cache stores `None` as the branch for that path
- **AND** the session picker omits the branch indicator (matching current behavior)

### Requirement: Worktree cache reused for unmanaged worktree detection

The unmanaged worktree detection logic SHALL consume the same parsed worktree list used for branch caching instead of running `git worktree list --porcelain` a second time.

#### Scenario: Single git call serves both purposes

- **WHEN** the session picker renders for a repo with worktrees
- **THEN** the system runs `git worktree list --porcelain` exactly once
- **AND** uses the parsed result for both branch display and unmanaged worktree filtering

### Requirement: Non-git repos skip worktree cache

The worktree cache SHALL NOT be constructed for non-git repos. Branch indicators and unmanaged worktree detection SHALL be skipped entirely, matching current behavior.

#### Scenario: Non-git repo renders without git calls

- **WHEN** a non-git repo is browsed in the session picker
- **THEN** the system does not call `git worktree list` or `git symbolic-ref`
- **AND** sessions render without branch indicators
