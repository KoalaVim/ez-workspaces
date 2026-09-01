# Git Branch Cache

## Purpose

Avoid subprocess calls for branch resolution when the branch hasn't changed since the last lookup, using filesystem mtime checks (~0.01ms) instead of `git symbolic-ref` subprocess spawns (~5ms).

## ADDED Requirements

### Requirement: Mtime-based branch caching

The system SHALL maintain an in-memory cache of `(branch_name, HEAD_mtime)` per worktree path within a single `ez` process lifetime. On branch lookup, the system SHALL `stat()` the HEAD reference file and compare its mtime against the cached value. If the mtime matches, the cached branch name SHALL be returned without spawning a subprocess.

#### Scenario: Cache hit on unchanged branch

- **WHEN** `get_branch()` is called for a path whose HEAD file mtime matches the cached mtime
- **THEN** the system returns the cached branch name without spawning any subprocess
- **AND** the total operation time is dominated by `stat()` (~0.01ms), not subprocess overhead (~5ms)

#### Scenario: Cache miss on changed branch

- **WHEN** `get_branch()` is called for a path whose HEAD file mtime differs from the cached value
- **THEN** the system spawns `git symbolic-ref --short HEAD` to resolve the current branch
- **AND** updates the cache with the new branch name and mtime

#### Scenario: Cache miss on first call

- **WHEN** `get_branch()` is called for a path not yet in the cache
- **THEN** the system spawns `git symbolic-ref --short HEAD` to resolve the branch
- **AND** stores the result and HEAD file mtime in the cache

#### Scenario: HEAD file resolution for worktrees

- **WHEN** the path is a git worktree (not the main repo checkout)
- **THEN** the system resolves the HEAD file by reading `path/.git` (a file containing `gitdir: <path>`) and stat-ing the actual HEAD file in the resolved git directory

#### Scenario: HEAD file missing or unreadable

- **WHEN** the HEAD file cannot be stat-ed (path deleted, permissions error)
- **THEN** the system falls back to spawning `git symbolic-ref` (same as a cache miss)

### Requirement: Cache scoped to process lifetime

The branch cache SHALL be scoped to a single `ez` process invocation. It SHALL NOT be persisted to disk. Within one `ez` session (e.g. the picker redraw loop), the cache survives across redraws, eliminating redundant git calls on sort toggles and other actions that re-render the list.

#### Scenario: Cache survives picker redraw

- **WHEN** the user toggles sort order in the session picker (triggering a re-render)
- **AND** no branches have changed since the last render
- **THEN** all branch lookups are cache hits (zero subprocess calls for branches)

#### Scenario: Cache does not persist across invocations

- **WHEN** the user exits `ez` and starts a new `ez` session
- **THEN** the branch cache starts empty and all lookups are cache misses on first render

### Requirement: Cache used by repo picker and tree view

The mtime-based branch cache SHALL also be used by the repo picker and tree view for their `get_branch()` calls on repo root paths. The same cache instance SHALL be shared across views within a single `ez` process.

#### Scenario: Repo picker uses cache across redraws

- **WHEN** the repo picker re-renders after a label edit (no branch changes)
- **THEN** all 19 repo branch lookups are cache hits

#### Scenario: Tree view shares cache with other views

- **WHEN** user switches from repo view to tree view
- **THEN** branches already cached from the repo view are cache hits in the tree view
