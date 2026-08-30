## Context

Sessions in `ez` are displayed in two picker contexts: `session_action_loop` (the main session picker when drilling into a repo) and `browser/views/tree.rs` (the all-repos tree view). Both build display strings from session metadata (name, labels, PR status, last-accessed) but currently omit the git branch.

Each session optionally carries a `path` field (the worktree directory). The existing `get_branch()` helper already resolves the current branch for a given path via `git symbolic-ref --short HEAD`.

## Goals / Non-Goals

**Goals:**
- Show the branch name for every session in the session picker and tree view
- Use a consistent, non-intrusive style (dimmed, in parentheses) that doesn't compete with the session name or labels

**Non-Goals:**
- Caching branch names across browser refreshes (session counts are small enough for per-render resolution)
- Showing branch info in non-interactive CLI output (`ez session list`)
- Making the branch display configurable or toggleable

## Decisions

### 1. Branch resolution strategy

Resolve the branch at render time by calling `get_branch()` on each session's effective path (session `path` or repo root fallback). This reuses the existing helper and avoids storing stale branch data in session metadata.

**Alternative considered**: Store branch name in session model at creation time. Rejected because branches can be renamed/changed outside of `ez`, leading to stale data.

### 2. Display position and format

Place the branch after the session name and before labels/PR metadata: `session-name (branch-name) [labels] [PR #n status] (2h ago)`.

The parenthesized branch uses `.dimmed()` styling to visually separate it from the bold session name. This mirrors how branch is shown in the repo lines (`[branch]` in cyan) but uses a different delimiter to avoid confusion with labels.

**Alternative considered**: Show after labels like the user's example `main [labels + pr status] (branch-name)`. The chosen position (right after name) keeps the most-identifying info together and avoids visual clutter between labels and last-accessed timestamp.

### 3. Shared helper function

Extract a `format_branch_indicator()` function in `browser/mod.rs` (alongside the existing `format_pr_indicator`) to avoid duplicating the formatting logic between `session_action_loop` and `tree.rs`.

### 4. Handling missing branches

When `get_branch()` returns `None` (detached HEAD, path doesn't exist, non-git dir), show nothing — no placeholder or error indicator. This keeps the display clean for edge cases like bare sessions that were never checked out.

## Risks / Trade-offs

- **Performance**: One `git symbolic-ref` subprocess per session per render. For typical counts (<30 sessions) this adds ~50-100ms total. Acceptable for an interactive picker. → If it becomes an issue, batch via `git worktree list --porcelain` in a future change.
- **Detached HEAD**: Sessions on detached HEADs show no branch indicator. This is correct behavior — there is no branch name to show. Users see `(no branch)` only if we add explicit handling, which we skip for simplicity.
