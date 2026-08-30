## Why

When browsing sessions, there's no way to see which git branch each session points to. The session name is often a descriptive label (e.g. "fix-auth-bug") that may differ from the actual branch name, and bare sessions share the repo's current branch. Showing the branch name removes the need to mentally map sessions to branches.

## What Changes

- Display the git branch name next to each session in the session picker (both `session_action_loop` and tree view)
- Format: `session-name [labels] [PR #n status] (branch-name)` — branch shown in parentheses, dimmed, after all existing metadata
- Branch is resolved by running `git symbolic-ref --short HEAD` on the session's worktree path (or the repo root for bare/pathless sessions)
- For bare sessions pointing to the repo root, the branch is still shown (it's the repo's current HEAD branch)

## Capabilities

### New Capabilities

- `session-branch-display`: Show the git branch name alongside session entries in all picker views

### Modified Capabilities

_(none — this is purely additive display logic)_

## Impact

- `src/browser/mod.rs` — `session_action_loop` display formatting and `format_session_display` or equivalent helper
- `src/browser/views/tree.rs` — tree view session line formatting
- Adds per-session `get_branch()` call during picker rendering (one git subprocess per session); acceptable since session counts are typically small (<30)
