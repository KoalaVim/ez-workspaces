## Why

Users need more flexibility in how they manage repos and sessions. Currently, only git repos can be tracked (requiring `.git`), sessions always create worktrees, sorting is alphabetical only, and there's no way to snapshot dirty changes into a new session. These limitations slow down common workflows: managing non-git directories, quickly finding recently used repos/sessions, creating lightweight sessions without worktree overhead, and safely moving uncommitted work to a new branch.

## What Changes

- **Non-git directory tracking**: Allow `ez add` to register directories without `.git` as tracked repos. The git-worktree plugin must skip worktree operations for these entries. Sessions under non-git repos are simple directory bookmarks.
- **Sort by last recently used**: Add LRU sorting for repos and sessions in all browser views, with a toggle keybind to switch between alphabetical and LRU. Track last-accessed timestamps on sessions and repos.
- **Bare sessions (no worktree)**: Add a new keybind (`Alt-Shift-N`) and CLI flag (`--bare`) to create sessions without triggering the git-worktree plugin's `OnSessionCreate` hook. Useful for bookmarking or plugin-only sessions.
- **Session from dirty changes**: Add a keybind and CLI command to create a new session from the current repo's uncommitted (unstaged) changes. The new session's worktree is based on the same commit as the current session, with the dirty files moved via `git stash` + `git stash pop` in the new worktree.

## Capabilities

### New Capabilities
- `non-git-repos`: Support for tracking directories without `.git` as repos, with worktree plugin skip logic
- `lru-sorting`: Last-recently-used sorting for repos and sessions across browser views
- `bare-sessions`: Session creation without worktree via explicit keybind/flag
- `session-from-dirty`: Create a new session by moving current uncommitted changes to a new worktree

### Modified Capabilities
- `repo-management`: Add support for non-git directory registration
- `session-management`: Add bare session flag and session-from-dirty workflow
- `interactive-browser`: Add LRU sort toggle and new keybinds (Alt-Shift-N, session-from-dirty)
- `configuration`: Add LRU sort config fields and bare session keybind

## Impact

- `src/repo/` — add non-git repo support, skip `.git` checks when flagged
- `src/session/` — bare session creation, session-from-dirty logic, LRU timestamps
- `src/browser/` — LRU sorting, new keybinds, sort toggle
- `src/config/model.rs` — new config fields for sorting and keybinds
- `plugins/git-worktree/` — skip logic for non-git repos and bare sessions
- `src/repo/model.rs` — `RepoMeta` timestamp field
- `src/session/model.rs` — `Session` timestamp and bare flag fields
