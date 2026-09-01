# Parallel Git Operations

## Purpose

Run independent `get_branch()` calls concurrently across different repos in the repo picker, tree view, and owner view, collapsing serial subprocess latency into overlapped wall time.

## ADDED Requirements

### Requirement: Parallel branch resolution in repo picker

The repo picker SHALL resolve branches for all repos concurrently using `std::thread::scope` instead of serially mapping over each repo. The results SHALL be collected in the same order as the repo list so display indexing is preserved.

#### Scenario: 19 repos resolved in parallel

- **WHEN** the repo picker renders with 19 registered repos
- **THEN** the system spawns branch resolution for all 19 repos concurrently
- **AND** total wall time is approximately the time of the single slowest git call, not the sum of all calls

#### Scenario: One repo branch fails

- **WHEN** one repo's `git symbolic-ref` fails (e.g. bare repo, missing .git)
- **THEN** that repo shows `?` as its branch (same as current behavior)
- **AND** other repos are unaffected — their branches resolve normally

### Requirement: Parallel branch resolution in tree view

The tree view SHALL resolve repo-level branches concurrently across repos within each workspace root. Per-session branches within a repo SHALL use the worktree cache (git-call-batching) instead of parallelization.

#### Scenario: Tree view with 2 workspace roots and 10 repos each

- **WHEN** the tree view renders
- **THEN** repo-level branches are resolved concurrently across all repos
- **AND** per-session branches within each repo come from the worktree list cache

### Requirement: Parallel branch resolution in owner view

The owner view SHALL resolve branches for all repos within the selected owner concurrently.

#### Scenario: Owner with 5 repos

- **WHEN** the owner view renders repos for a selected owner
- **THEN** all 5 repo branches are resolved concurrently

### Requirement: Thread safety of branch cache

The mtime-based branch cache SHALL be safe to use from multiple threads within `std::thread::scope`. The cache SHALL use interior mutability (e.g. `Mutex<HashMap>` or `DashMap`) to allow concurrent reads and writes from scoped threads.

#### Scenario: Parallel repo lookups populate cache

- **WHEN** 19 parallel threads call the cached `get_branch()` for different paths
- **THEN** all 19 results are stored in the cache without data races
- **AND** subsequent serial lookups for the same paths are cache hits
