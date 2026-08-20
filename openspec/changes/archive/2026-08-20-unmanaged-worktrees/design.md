## Context

ez sessions are metadata records in `sessions.toml` that point to git worktrees. The git-worktree plugin creates worktrees on session creation. But worktrees can also exist outside ez's control — created manually (`git worktree add`), by other tools, or orphaned from failed session cleanups. These are invisible in the browser.

Currently, `ez session register` can adopt an existing worktree, but the user must know it exists and run the command manually. There's no discovery mechanism.

## Goals / Non-Goals

**Goals:**
- Surface non-managed worktrees in the per-repo session picker so users can discover and adopt them
- Make registration a one-step action (select → register → enter)
- Keep the UI clean — non-managed worktrees are secondary to managed sessions

**Non-Goals:**
- Auto-registering worktrees without user action
- Showing non-managed worktrees in the global Tree view (only the per-repo session picker)
- Periodic background scanning or notifications about orphaned worktrees
- Deleting non-managed worktrees from the browser (register first, then delete as a normal session)

## Decisions

### Detection: `git worktree list --porcelain`

Run `git worktree list --porcelain` against the repo root. Parse the output to extract worktree paths and branch names. Subtract:
1. The main repo path itself (already the `main ★` session)
2. Any path matching a managed session's `session.path`

The remainder are non-managed worktrees.

**Why porcelain:** Stable, machine-parseable output. Each worktree is a block of lines: `worktree <path>`, `HEAD <sha>`, `branch refs/heads/<name>` (or `detached`). Easy to parse in Rust.

**Alternative considered:** Scanning the `.git/worktrees/` directory directly. Rejected — `git worktree list` handles edge cases (locked worktrees, prunable entries) and is the canonical API.

### Placement: Below managed sessions with a "Not Registered" header

Non-managed worktrees appear in the same fzf picker as managed sessions, below a separator line with a "Not Registered" header. This is a single fzf instance — the non-managed items are appended to the session items list.

The header row itself is non-interactive (selecting it does nothing). Non-managed items are dimmed and show the branch name and path.

**Why not a separate view or keybind:** Discovery is the point. If users have to opt in to see them, they won't discover orphaned worktrees. Showing them inline creates awareness.

### Interaction: Register + Enter on select

When the user selects a non-managed worktree:
1. Register it as a session under the default (main) session using existing `register_existing_worktree` logic
2. Run `on_enter` to enter the newly registered session

This is a single action — no confirmation prompt needed since registration is non-destructive.

### Implementation location

Add a `list_unmanaged_worktrees` helper in `src/session/mod.rs` that takes a repo entry and session tree, runs `git worktree list --porcelain`, and returns a list of `UnmanagedWorktree { path, branch }` structs.

The `session_action_loop` in `src/browser/mod.rs` calls this helper after building the managed session items, appends the "Not Registered" header and worktree items to the fzf item list, and handles selection by registering and entering.

### Preview pane

Non-managed worktrees show basic git info in the preview pane: branch, recent commits, dirty status. This reuses the existing directory preview logic since the worktree path is a valid git directory.

## Risks / Trade-offs

- **[Performance] Subprocess on every render** → `git worktree list` typically completes in <50ms. Acceptable for an interactive picker that already shells out to fzf. If it becomes a problem, we can cache per browser session.
- **[UX clutter] Repos with many stale worktrees** → The "Not Registered" section could dominate the picker. Mitigated by placing it below managed sessions and dimming the items. Users can clean up by registering and deleting.
- **[Edge case] Locked or prunable worktrees** → `git worktree list` includes these. We should filter out prunable entries (worktree path doesn't exist on disk) but show locked ones since they're valid worktrees.
- **[Edge case] Detached HEAD worktrees** → Some worktrees may have no branch (detached HEAD). Show the short SHA instead of a branch name.
