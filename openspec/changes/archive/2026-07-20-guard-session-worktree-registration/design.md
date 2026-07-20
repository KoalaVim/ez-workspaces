## Context

`browse_repo` auto-registers any `.git` directory the user selects in the drill-down. Plugins like kv mutate session paths to external git directories (e.g. `~/.local/share/kvim-envs/*/lazy/KoalaVim`). These get silently registered as new repos.

## Goals / Non-Goals

**Goals:**
- Prevent auto-registration of paths already tracked as session worktrees.
- Redirect the user to the owning repo's session picker when they browse into a session worktree.

**Non-Goals:**
- Preventing manual `ez add` on worktree paths (user intent is explicit).
- Cleaning up already-stale repo entries (user can `ez repo remove`).

## Decisions

### Check all repos' sessions for path match

Before calling `repo::add_repo` in `browse_repo`, iterate over all registered repos and their sessions to see if any session's `path` matches the candidate path. If found, use the owning repo entry instead of registering a new one. This is O(repos × sessions) but both are small lists (tens of items), so it's negligible.

## Risks / Trade-offs

- **[False positive on shared paths]** → Unlikely in practice. Session paths are unique per session. If two repos somehow share a worktree path, the first match wins — acceptable since this is a dedup guard, not a critical lookup.
