## Why

Sessions often accumulate TODO items in their notes (e.g. `- [ ] push final review`, `- [ ] update docs`). Deleting a session with outstanding unchecked todos risks losing track of unfinished work. A pre-delete guard catches this and forces explicit acknowledgment.

## What Changes

- Add a pre-delete guard that reads the session's notes README.md and checks for unchecked markdown todos (`- [ ] ...`)
- If unchecked todos are found, block deletion unless `--force` is passed or the user confirms interactively
- The guard applies to both CLI `ez session delete` and the browser delete action
- For cascade deletes, check all descendants' notes as well

## Capabilities

### New Capabilities
- `delete-guard-unchecked-todos`: Pre-delete check that prevents session deletion when notes contain unchecked markdown todo items (`- [ ] ...`)

### Modified Capabilities
- `session-management`: The delete session requirement gains an additional pre-flight check (unchecked todos guard) that runs alongside the existing dirty-worktree check

## Impact

- `src/session/notes.rs`: New function to scan for unchecked todos
- `src/session/mod.rs`: Wire the guard into `delete_session` and `delete_session_by_id` flows
- `src/browser/mod.rs`: Wire the guard into the browser delete action (same as dirty worktree warning)
- `src/error.rs`: New error variant for unchecked todos
