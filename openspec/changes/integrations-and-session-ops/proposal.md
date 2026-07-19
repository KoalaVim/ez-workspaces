## Why

ez sessions currently lack integration with external developer tools (Cursor IDE, GitHub CLI, Raycast) and several session operations are incomplete or missing. Session rename only updates metadata without touching the underlying branch/worktree, the "from PR" workflow doesn't actually check out the PR's branch, MCP auth tokens don't carry over to worktrees, there's no way to see PR merge status at a glance, repo removal requires knowing the repo name rather than accepting a path, and there's no quick-launcher integration.

## What Changes

- **Cursor MCP auth plugin**: New bundled plugin that symlinks `mcp-auth.json` from the main repo's Cursor project directory to each worktree's project directory, so MCP server auth tokens are shared across all sessions of a repo.
- **PR checkout workflow**: Enhance the "From GitHub PR" name builder mode to use `gh pr view` to resolve the PR's branch, check it out in the new worktree, and `git reset` to the merge-base with main — presenting the PR's changes as dirty/unstaged files for review.
- **Full session rename**: Extend `ez session rename` and `Alt-r` to also rename the git branch and move the worktree directory. Optionally copy Cursor conversations from the old workspace to the new one (using the Cursor workspace slug convention).
- **Remove repo by path**: `ez repo remove` and `ez remove` should accept a directory path (resolving it to the registered repo) in addition to the repo name, including non-git repos.
- **Raycast adapter**: A Raycast script command or extension that lists repos/sessions and launches them via `ez`, providing a system-wide quick-launcher.
- **PR status on session info**: Store and display the associated PR's merge status (open/merged/closed) in session metadata, using `gh pr view` to fetch status. Show in `ez session list`, the browser preview, and session picker display.

## Capabilities

### New Capabilities
- `cursor-mcp-auth`: Bundled plugin that shares Cursor MCP auth tokens across worktree sessions by symlinking `mcp-auth.json`
- `pr-checkout`: Enhanced PR-based session creation that checks out the PR branch and presents changes as dirty files
- `pr-status`: Track and display GitHub PR merge status in session metadata
- `raycast-adapter`: Raycast script commands for launching ez repos and sessions

### Modified Capabilities
- `session-management`: Add full rename (branch + worktree + optional Cursor conversation copy), enhance session-from-dirty to support PR checkout workflow
- `repo-management`: Accept path argument for repo removal (resolve path to registered repo)
- `interactive-browser`: Display PR status indicator in session picker and preview pane

## Impact

- **New files**: `plugins/cursor-mcp-auth/`, `src/session/rename.rs` (extracted rename logic), raycast scripts
- **Modified modules**: `src/session/mod.rs` (rename), `src/session/name_builder.rs` (PR checkout), `src/repo/mod.rs` (remove by path), `src/browser/mod.rs` (PR status display), `src/browser/preview.rs` (PR status in preview)
- **New dependencies**: `gh` CLI (optional, for PR features), Raycast (optional, for adapter)
- **Plugin protocol**: New `OnSessionRename` hook fields for old/new branch and path; new `is_git` context for cursor-mcp-auth
