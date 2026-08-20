## Why

Git worktrees created outside of ez (manually via `git worktree add`, by other tools, or orphaned from failed session deletes) are invisible in the session picker. Users must run `git worktree list` manually to discover them. This creates a blind spot — stale worktrees accumulate, and useful ones aren't accessible from the browser.

## What Changes

- Detect non-managed git worktrees when rendering the per-repo session picker
- Show them below managed sessions under a "Not Registered" title, visually distinct (dimmed)
- Selecting a non-managed worktree registers it as an ez session (reusing `register_existing_worktree` logic) and enters it
- Detection uses `git worktree list --porcelain`, subtracting paths already tracked as session paths and the main repo path

## Capabilities

### New Capabilities
- `unmanaged-worktree-discovery`: Detect and display git worktrees that exist on disk but aren't tracked as ez sessions, with one-click registration from the session picker

### Modified Capabilities
- `interactive-browser`: Add "Not Registered" section to the per-repo session picker showing non-managed worktrees
- `session-management`: Extend registration flow to support inline registration from the browser (register + enter in one action)

## Impact

- `src/browser/mod.rs`: session_action_loop gains worktree detection and rendering of the "Not Registered" section
- `src/session/mod.rs`: may need a helper to list git worktrees and diff against session paths
- Preview pane may need updates if non-managed worktrees should show preview info
- No breaking changes, no new dependencies — `git worktree list --porcelain` is standard git
