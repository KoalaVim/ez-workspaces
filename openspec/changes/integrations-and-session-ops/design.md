## Context

ez-workspaces has a working session lifecycle (create, enter, delete, rename) and plugin system. Several gaps remain: rename only updates metadata (not the git branch or worktree directory), the "From GitHub PR" name builder mode only extracts the PR number without checking out the actual branch, Cursor IDE's MCP auth tokens don't carry over to worktree directories, and there's no integration with system-level launchers like Raycast. Repo removal only works by name, not path.

The `gh` CLI is widely available and provides structured JSON output for PR metadata. Cursor stores per-workspace state under `~/.cursor/projects/<slug>/` where slug is derived from the workspace path. Raycast supports script commands (bash/python/swift) that output JSON for list rendering.

## Goals / Non-Goals

**Goals:**
- Full session rename that updates branch, worktree dir, and optionally Cursor conversation references
- PR-aware session creation that checks out the actual PR branch and presents changes as dirty files
- Shared MCP auth across worktree sessions via symlinks
- System-level quick-launch via Raycast
- PR merge status tracking on sessions
- Repo removal by path (not just name)

**Non-Goals:**
- GUI/Electron app — Raycast adapter is the only GUI integration
- Automatic PR creation from sessions
- Cursor extension/plugin API integration (only filesystem-level symlinks)
- Supporting launchers other than Raycast in this change

## Decisions

### 1. Cursor MCP auth: bundled plugin using OnSessionCreate hook

**Decision**: Implement as a new bundled plugin `cursor-mcp-auth` that runs on `OnSessionCreate`. It computes the Cursor project slug for both the main repo path and the worktree path, then creates a symlink from the worktree's `mcp-auth.json` to the main repo's.

**Rationale**: The slug algorithm (`sed -E 's/[^a-zA-Z0-9]/-/g; s/-+/-/g; s/^-+|-+$//g'`) is already known from the user's dotfiles. A plugin is cleaner than baking it into core — users who don't use Cursor can skip it.

**Slug formula**: `echo "$path" | sed -E 's/[^a-zA-Z0-9]/-/g; s/-+/-/g; s/^-+|-+$//g'`

The plugin also needs an `OnSessionRename` handler to update the symlink when the worktree path changes.

### 2. PR checkout: extend name builder + post-create workflow

**Decision**: Enhance the existing "From GitHub PR" name builder mode to:
1. Use `gh pr view <url> --json headRefName,baseRefName,number` to get the PR branch and base
2. Create the session with the PR branch name (not `pr<number>`)
3. After worktree creation, run `git reset $(git merge-base HEAD origin/<base>)` in the worktree to unstage all PR commits as dirty changes

**Alternative considered**: Creating a separate `ez session from-pr` command. Rejected because it duplicates session creation logic and the name builder already has the PR URL input flow.

**Rationale**: `git reset --mixed` to the merge-base makes the worktree show exactly the PR's diff as uncommitted files, which is ideal for code review. The session name uses the actual branch name so plugins (tmux, etc.) see the real branch.

The `--hard` reset is too aggressive — it would lose any local changes. `--mixed` keeps the working tree intact with changes shown as unstaged.

### 3. Full session rename: branch + worktree + Cursor conversation

**Decision**: Extend `rename_session` in `src/session/mod.rs` to:
1. Rename the git branch: `git branch -m <old> <new>` in the worktree
2. Move the worktree directory: `git worktree move <old-path> <new-path>`
3. Update session metadata (name, path)
4. Run `OnSessionRename` hooks with both old and new names/paths
5. Optionally copy Cursor conversations: compute old and new workspace slugs, copy transcript and chat directories

**Hook protocol change**: `OnSessionRename` hook request gains `rename_context` with `old_name`, `new_name`, `old_path`, `new_path`.

**Cursor conversation copy**: This is opt-in via config flag `copy_cursor_conversations = true` in `[plugin_settings.cursor-mcp-auth]` or a separate config key. Implementation mirrors the logic in `copy-conv.sh` but scoped to the specific workspace slugs.

### 4. Remove repo by path

**Decision**: Modify `repo::remove_repo` to accept either a name/id OR a path. If the argument looks like a path (contains `/` or `.`), resolve it to an absolute path and find the matching `RepoEntry` by path.

**Rationale**: Simple to implement. Also add `ez remove` as a top-level alias for `ez repo remove` for ergonomics.

### 5. Raycast adapter: script commands

**Decision**: Ship Raycast script commands as standalone bash/zsh scripts in a `raycast/` directory. They use `ez repo list --json` and `ez session list --json --repo <id>` (need to add `--json` output to these commands) to populate Raycast's list, then execute `open -a Terminal` with `ez session enter` or the tmux attach command.

**Two scripts**:
- `ez-repos.sh` — lists repos, selecting one shows its sessions
- `ez-sessions.sh` — lists sessions for a selected repo, selecting one enters/attaches

**Prerequisite**: Add `--json` flag to `ez repo list` and `ez session list` to output structured JSON.

### 6. PR status: store in session env, display in browser

**Decision**: Store PR metadata in `session.env` as `ez_pr_number`, `ez_pr_url`, and `ez_pr_status` (open/merged/closed). The status is fetched via `gh pr view <number> --repo <remote> --json state` and cached. Refresh happens:
- On session enter (if stale, > 5 minutes)
- On explicit refresh keybind (future)

Display: show a colored indicator in the session picker line (e.g. `[PR #42 open]` green, `[PR #42 merged]` purple, `[PR #42 closed]` red).

**Alternative considered**: Storing in `plugin_state`. Rejected because `env` is already shown in hook requests and accessible to all plugins.

### 7. JSON output for CLI commands

**Decision**: Add `--json` flag to `ez repo list` and `ez session list` that outputs JSON arrays. This is needed by the Raycast adapter and is generally useful for scripting.

## Risks / Trade-offs

- **`gh` dependency**: PR checkout and PR status require `gh` to be installed and authenticated. Risk: users without `gh` see errors. Mitigation: check for `gh` availability before attempting, fall back gracefully with informative messages.
- **`git worktree move` limitations**: Not all git versions support `git worktree move`. Mitigation: check git version or catch the error and fall back to manual directory move + worktree re-add.
- **Cursor path convention**: The workspace slug algorithm may change in future Cursor versions. Mitigation: the plugin is self-contained and easy to update.
- **Raycast macOS-only**: Raycast only works on macOS. Mitigation: the adapter is a separate directory with no impact on core functionality.
- **PR status staleness**: Cached PR status may be outdated. Mitigation: refresh on enter with a TTL, and the status is purely informational (no actions depend on it).
