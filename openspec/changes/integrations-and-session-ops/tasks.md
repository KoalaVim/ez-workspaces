## 1. JSON Output for CLI Commands

- [x] 1.1 Add `--json` flag to `ez repo list` in `src/cli.rs` and `src/repo/mod.rs` — output JSON array of repo objects
- [x] 1.2 Add `--json` flag to `ez session list` in `src/cli.rs` and `src/session/mod.rs` — output JSON array of session objects
- [x] 1.3 Add `ez remove` top-level alias in `src/cli.rs` that delegates to `ez repo remove`

## 2. Remove Repo by Path

- [x] 2.1 Modify `repo::remove_repo` in `src/repo/mod.rs` to accept path argument — resolve to absolute path and match against registered repos
- [x] 2.2 Update `src/cli.rs` to pass the argument to `remove_repo` which handles both name and path
- [x] 2.3 Add path resolution: if arg contains `/` or `.`, resolve to canonical path and find matching `RepoEntry.path`

## 3. Full Session Rename (Branch + Worktree)

- [x] 3.1 Extend `rename_session` in `src/session/mod.rs` to run `git branch -m <old> <new>` in the worktree
- [x] 3.2 Add `git worktree move <old-path> <new-path>` after branch rename, update `session.path`
- [x] 3.3 Skip branch/worktree operations for bare sessions and non-git repos
- [x] 3.4 Add `rename_context` (`old_name`, `new_name`, `old_path`, `new_path`) to `OnSessionRename` hook request in `src/plugin/protocol.rs` and `src/plugin/mod.rs`
- [x] 3.5 Update tmux plugin `on_session_rename` handler to rename the tmux session using `rename_context`

## 4. Cursor Conversation Copy on Rename

- [x] 4.1 Add `copy_cursor_conversations` config option (default `false`) to `EzConfig` in `src/config/model.rs`
- [x] 4.2 Implement Cursor workspace slug computation in `src/session/mod.rs` (or a new `src/session/cursor.rs`)
- [x] 4.3 On rename (when enabled), compute old/new slugs, copy `~/.cursor/projects/<slug>/agent-transcripts/<conv>/` and `~/.cursor/chats/<hash>/<conv>/` dirs
- [x] 4.4 Compute chat hash: `md5(realpath(workspace_path))` using the same algorithm as Cursor

## 5. Cursor MCP Auth Plugin

- [x] 5.1 Create `plugins/cursor-mcp-auth/manifest.toml` with `OnSessionCreate`, `OnSessionDelete`, `OnSessionRename` hooks
- [x] 5.2 Create `plugins/cursor-mcp-auth/cursor-mcp-auth-plugin` bash script — `on_session_create` handler: compute main repo slug and worktree slug, symlink `mcp-auth.json`
- [x] 5.3 Add `on_session_delete` handler: remove symlink at worktree slug's project dir
- [x] 5.4 Add `on_session_rename` handler: remove old symlink, create new one using `rename_context` paths
- [x] 5.5 Register as bundled plugin in `src/plugin/bundled.rs`

## 6. PR Checkout Workflow

- [x] 6.1 Enhance `prompt_github_pr` in `src/session/name_builder.rs` — use `gh pr view <url> --json headRefName,baseRefName,number` to resolve branch
- [x] 6.2 Set session name to `headRefName` (branch name) instead of `pr<number>`, store `ez_pr_number` and `ez_pr_url` in session env
- [x] 6.3 Pass `start_point = origin/<headRefName>` to session creation so git-worktree plugin uses the remote branch
- [x] 6.4 After session creation, run `git reset --mixed $(git merge-base HEAD origin/<baseRefName>)` in the new worktree to present PR changes as dirty
- [x] 6.5 Handle fallbacks: `gh` not installed, auth failure, merge-base not found — fall back to existing `pr<number>` behavior with warnings

## 7. PR Status Tracking

- [x] 7.1 On session enter in `src/session/mod.rs`, check if `ez_pr_number` is in session env — if so and status is stale (>5 min), run `gh pr view` to refresh `ez_pr_status`
- [x] 7.2 Add PR status display in session picker in `src/browser/mod.rs` — colored `[PR #N status]` indicator
- [x] 7.3 Add PR status to preview pane in `src/browser/preview.rs`

## 8. Raycast Adapter

- [x] 8.1 Create `raycast/` directory with README explaining setup
- [x] 8.2 Create `raycast/ez-repos.sh` Raycast script command — lists repos via `ez repo list --json`, opens selected repo
- [x] 8.3 Create `raycast/ez-sessions.sh` Raycast script command — lists sessions for a repo, enters selected session

## 9. Documentation & Verification

- [ ] 9.1 Update `README.md` with new commands (`ez remove`, `--json` flags), PR checkout workflow, cursor-mcp-auth plugin, Raycast adapter
- [ ] 9.2 Update `docs/user-guide.md` with full rename behavior, PR checkout, PR status, Raycast setup
- [ ] 9.3 Update `AGENTS.md` with new modules and plugin
- [ ] 9.4 Run `make check` — zero warnings, all tests pass
- [ ] 9.5 Manual smoke test all features
