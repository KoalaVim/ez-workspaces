## Why

When a plugin (e.g. kv) mutates a session's worktree path to an external directory, that path is a valid `.git` directory. If the user later browses into it via the workspace drill-down, `browse_repo` silently auto-registers it as a new repo — creating duplicate entries in the repo view for what are really session worktrees. This pollutes the repo list with phantom repos that share the same name.

## What Changes

- Before auto-registering a repo in `browse_repo`, check if the path is already tracked as a session worktree under any registered repo.
- If it is, skip registration and jump directly to the owning repo's session picker instead.

## Capabilities

### New Capabilities
_(none)_

### Modified Capabilities
- `repo-management`: Auto-register on browse now checks for existing session worktree ownership before registering.

## Impact

- `src/browser/mod.rs`: `browse_repo` gains a session-worktree check before calling `repo::add_repo`.
- `src/session/store.rs` or `src/session/mod.rs`: new helper to find which repo+session owns a given path.
