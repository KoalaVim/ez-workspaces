## Why

The session picker and repo picker spawn a subprocess for every `git symbolic-ref --short HEAD` call — one per session or repo — plus additional subprocesses for worktree listing and plugin execution. For a repo with 11 sessions and 19 registered repos, this adds up to **30+ fork+exec calls** on the hot path before fzf even appears, costing ~200-500ms of startup latency. Every action within the picker (sort toggle, rename, delete) re-runs the full cycle. The session picker already calls `git worktree list --porcelain` for unmanaged worktree detection, which returns branch info for every worktree — but discards it and re-fetches each branch individually.

## What Changes

- **Batch session branches via worktree list**: Parse `git worktree list --porcelain` once per repo into a `path → branch` map. Use it for both session branch display and unmanaged worktree detection. Eliminates N per-session `git symbolic-ref` calls, replacing them with one git call that's already happening.
- **Parallel repo branches**: Run `get_branch()` across different repos concurrently using `std::thread::scope`. Collapses ~100ms of serial git calls into ~5-10ms wall time in the repo picker, tree view, and owner view.
- **Mtime-based branch caching**: Cache `(branch_name, HEAD_mtime)` per path. On render, `stat()` the HEAD file (~0.01ms) instead of spawning git (~5ms). Only shell out when the mtime has changed. Persisted across picker redraws within a session; optionally across ez invocations.
- **Microsecond debug timestamps**: Switch `env_logger` to `.format_timestamp_micros()` so debug logs can actually measure per-operation timing.

## Capabilities

### New Capabilities
- `git-call-batching`: Batching session branch lookups via `git worktree list --porcelain` output, replacing per-session subprocess calls with a single parsed result.
- `git-branch-cache`: Mtime-based branch cache that avoids subprocess calls when the branch hasn't changed since the last lookup.
- `parallel-git-ops`: Concurrent `get_branch()` execution across independent repos using thread scoping.

### Modified Capabilities
- `interactive-browser`: The session picker loop and repo picker loop change how they obtain branch info — from per-item subprocess calls to cached/batched lookups. No user-visible behavior change.
- `session-branch-display`: Branch display now sources from the batched worktree list or mtime cache instead of individual `git symbolic-ref` calls. Same display output, different data path.
- `unmanaged-worktree-discovery`: The worktree list parse is unified with the branch-batching parse — one git call serves both purposes instead of separate calls.

## Impact

- **`src/browser/mod.rs`**: `session_action_loop` refactored to build a worktree info cache before the item loop. `get_branch()` gains a cache-aware variant. The per-session branch lookup in the item-building loop becomes a HashMap lookup.
- **`src/browser/views/repo.rs`**: The per-repo `get_branch()` loop replaced with parallel execution via `std::thread::scope`.
- **`src/browser/views/tree.rs`**: Same parallel treatment for repo branches; session branches use worktree batching when inside a repo context.
- **`src/browser/views/owner.rs`**: Same parallel treatment for the selected-owner repo list.
- **`src/session/mod.rs`**: `list_unmanaged_worktrees` refactored to share its parsed output with the branch cache instead of being a standalone function.
- **`src/main.rs`**: One-line change to `env_logger::Builder` for microsecond timestamps.
- **No new dependencies**: `std::thread::scope` (stable since Rust 1.63) replaces the need for rayon. No new crates required.
- **No user-visible behavior change**: Same branch names, same display, same picker behavior — just faster.
