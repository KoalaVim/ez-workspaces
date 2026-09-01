## Context

The interactive browser spawns `git symbolic-ref --short HEAD` once per session (or repo) on every render cycle. With 11 sessions, that's 11 subprocess spawns (~5ms each, ~55-80ms total). The session picker also calls `git worktree list --porcelain` for unmanaged worktree detection — this output already contains branch info for every worktree but it's discarded. The repo picker (19 repos) runs another 19 serial `get_branch()` calls. Every action within the picker (sort toggle, rename, label edit) re-runs the entire cycle.

The browser already has a loop structure where fzf blocks on user input — any I/O savings on the render phase directly reduce perceived latency.

## Goals / Non-Goals

**Goals:**
- Eliminate per-session `git symbolic-ref` calls in the session picker by reusing `git worktree list` output
- Parallelize per-repo `get_branch()` calls in the repo picker, tree view, and owner view
- Cache branch lookups within a single `ez` process so re-renders after non-branch-changing actions are free
- Add millisecond timestamps to debug logs for measuring impact

**Non-Goals:**
- Persisting the branch cache to disk across `ez` invocations (process-scoped is sufficient)
- Optimizing the fzf preview subprocess (`ez preview`) — separate concern
- Reducing plugin subprocess overhead (separate concern)
- Caching across different repos' worktree lists (each repo gets its own `git worktree list` call)

## Decisions

### Decision 1: Unified `WorktreeInfo` struct replaces separate worktree and branch lookups

**Choice:** Introduce a `WorktreeInfo` struct that holds both the branch map (`HashMap<PathBuf, Option<String>>`) and the unmanaged worktree list. A single `build_worktree_info(repo_path, session_tree)` function parses `git worktree list --porcelain` once and returns both.

**Alternative considered:** Keep `list_unmanaged_worktrees` as-is and add a separate `batch_get_branches` function that also calls `git worktree list`. Rejected because it would still run `git worktree list` twice.

**Rationale:** The existing `parse_worktree_list_porcelain` function already extracts paths and branches. Extending it to also produce a branch map is a small change. The unmanaged filtering is then applied to the same parsed data.

**Where it lives:** `src/session/mod.rs` alongside the existing `list_unmanaged_worktrees` and `parse_worktree_list_porcelain`. The new function consumes these rather than duplicating logic.

### Decision 2: `BranchCache` with `Mutex<HashMap>` for thread-safe mtime caching

**Choice:** A `BranchCache` struct wrapping `Mutex<HashMap<PathBuf, (Option<String>, SystemTime)>>`. The public API is `fn get_branch(&self, path: &Path) -> Option<String>` which checks mtime, returns cached value or falls through to `git symbolic-ref`. The cache is created once in `browse()` and passed by reference through the call chain.

**Alternative considered:** `DashMap` for lock-free concurrent reads. Rejected because the contention window is tiny (HashMap lookup + stat) and adding a dependency isn't warranted. `std::sync::Mutex` is sufficient.

**Alternative considered:** `RefCell<HashMap>` for single-threaded use, with a separate parallel branch for repo picker. Rejected because it splits the cache in two and prevents repo picker results from populating the shared cache.

**Rationale:** `Mutex` is simple, has negligible overhead for this access pattern (short critical sections, low contention), and works with `std::thread::scope`.

**How mtime resolution works for worktrees:** For a worktree, `path/.git` is a file containing `gitdir: /path/to/.git/worktrees/<name>`. The HEAD file is at that gitdir path + `/HEAD`. The cache resolves this once per path, caches both the branch and the mtime.

### Decision 3: `std::thread::scope` for parallel repo branch resolution (no rayon)

**Choice:** Use `std::thread::scope` (stable since Rust 1.63) to spawn one thread per repo for branch resolution. Threads share the `BranchCache` by reference.

**Alternative considered:** `rayon::par_iter`. Rejected because adding rayon as a dependency for one parallel loop is heavy. The repo count (10-30) is small enough that raw threads are fine.

**Rationale:** `thread::scope` is in std, borrows work naturally (no `Arc` needed), and the parallelism pattern is simple: spawn N, join all, collect results.

**Where it's used:**
- `src/browser/views/repo.rs` — the per-repo item building loop
- `src/browser/views/tree.rs` — the per-repo branch resolution (session branches use worktree cache)
- `src/browser/views/owner.rs` — the per-repo item building in the selected owner's list

### Decision 4: `BranchCache` threaded through via function parameters, not globals

**Choice:** The `BranchCache` is created in `browse()` and passed as `&BranchCache` to `session_action_loop`, view functions, and any function that calls `get_branch`.

**Alternative considered:** Thread-local / lazy static. Rejected because it hides dependencies and makes testing harder.

**Rationale:** Explicit parameter passing is consistent with the existing code style (e.g. `config` is passed through the call chain). The cache is lightweight (one `Mutex<HashMap>`).

### Decision 5: Microsecond timestamps in debug log

**Choice:** Change `env_logger::Builder` to use `.format_timestamp_micros()`.

**Rationale:** Current second-level precision makes it impossible to measure individual operation timing. Microsecond precision supports future sub-millisecond optimization work. This is a one-line change with no behavioral impact.

## Risks / Trade-offs

**[Risk] Worktree list output may not include all session paths** → Sessions whose paths are outside the git worktree system (manually created directories, broken worktree links) won't appear in the cache. Mitigation: fall back to `None` (branch indicator omitted), matching current behavior when `git symbolic-ref` fails.

**[Risk] Mutex contention in parallel branch resolution** → The critical section is a HashMap lookup + `stat()` call (~0.02ms). With 19 threads, worst case is 18 × 0.02ms = 0.36ms of contention. Negligible compared to the ~5ms git subprocess time saved per thread.

**[Risk] HEAD mtime granularity on some filesystems** → HFS+ (older macOS) has 1-second mtime granularity. If a branch changes and is looked up within the same second, the cache returns stale data. Mitigation: APFS (all modern Macs) has nanosecond precision. On HFS+, the stale branch name is corrected on the next render cycle (the mtime will differ by then). This is a display-only issue with no data corruption risk.

**[Trade-off] Worktree cache rebuilt on every loop iteration** → The session action loop rebuilds the worktree cache on each re-render (after actions). This is intentional: actions like session creation change the worktree state. The cost is one `git worktree list` call (~5ms) per iteration, down from N+1 calls (~55ms+ for 11 sessions). Could be optimized further by only rebuilding after branch-mutating actions, but the win is marginal and adds complexity.

**[Trade-off] BranchCache parameter threading** → Every function in the call chain from `browse()` through the view functions needs a `&BranchCache` parameter. This touches many function signatures. The alternative (global state) is worse for testability and clarity.

## Open Questions

None — the design is straightforward and all decisions are low-risk. Implementation can proceed.
