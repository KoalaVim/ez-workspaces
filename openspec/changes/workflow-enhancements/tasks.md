## 1. Non-Git Directory Tracking

- [ ] 1.1 Add `is_git: bool` field to `RepoEntry` in `src/repo/model.rs` with `#[serde(default = "default_true")]`
- [ ] 1.2 Update `ez add` in `src/repo/mod.rs` to detect `.git` presence and set `is_git` accordingly, skip remote-URL and default-branch detection when `false`
- [ ] 1.3 Add `is_git: bool` to `RepoInfo` in plugin protocol (`src/plugin/protocol.rs`) and populate it in hook requests (`src/plugin/mod.rs`)
- [ ] 1.4 Update git-worktree plugin to check `is_git` and return early on `OnSessionCreate`/`OnSessionDelete`/`OnSessionRename` when `false`
- [ ] 1.5 Update session creation in `src/session/mod.rs` to set session `path` to repo root for non-git repos (skip name-builder git-dependent modes)
- [ ] 1.6 Update browser preview pane in `src/browser/preview.rs` to show directory listing instead of git status for non-git repos
- [ ] 1.7 Update docs: `README.md`, `docs/user-guide.md`, `AGENTS.md`

## 2. LRU Sorting

- [ ] 2.1 Add `last_accessed: Option<String>` (ISO 8601) to `Session` in `src/session/model.rs` and `RepoMeta` in `src/repo/model.rs`, with `#[serde(default)]`
- [ ] 2.2 Update session enter logic in `src/session/mod.rs` to set `last_accessed` on the session and its parent repo's `RepoMeta`
- [ ] 2.3 Update repo browse-into logic in `src/browser/mod.rs` to set `last_accessed` on `RepoMeta`
- [ ] 2.4 Add `SortMode` enum (`Alpha`, `Lru`) and sorting utilities in a shared location (e.g. `src/browser/mod.rs` or a new `src/browser/sort.rs`)
- [ ] 2.5 Add `default_sort` field to `EzConfig` in `src/config/model.rs` (default `"alpha"`)
- [ ] 2.6 Add `sort_toggle` keybind to `KeybindsConfig` (default `ctrl-s`)
- [ ] 2.7 Wire sort toggle into fzf actions in `src/browser/selector.rs` — re-render with toggled sort on keybind press, update header to show current sort mode
- [ ] 2.8 Apply sorting to all browser views: Workspace, Repo, Owner, Label, Tree, and session picker
- [ ] 2.9 Update docs: `README.md`, `docs/user-guide.md`, `AGENTS.md`

## 3. Bare Sessions

- [ ] 3.1 Add `bare: bool` field to `Session` in `src/session/model.rs` with `#[serde(default)]`
- [ ] 3.2 Add `--bare` flag to `SessionCommand::New` in `src/cli.rs`
- [ ] 3.3 Update `new_session` in `src/session/mod.rs` to skip `OnSessionCreate` and `OnSessionDelete` hooks when `bare = true`, leave session `path` as `None`
- [ ] 3.4 Update session enter in `src/session/mod.rs` — for bare sessions with `on_enter = "cd"`, display message and skip cd; allow plugin-bind enter actions
- [ ] 3.5 Add `new_bare_session` keybind to `KeybindsConfig` (default `alt-shift-n`)
- [ ] 3.6 Wire `Alt-Shift-N` keybind in the session action loop (`src/browser/mod.rs`) — create session with `bare = true`
- [ ] 3.7 Display `[bare]` indicator in session tree rendering (`src/session/tree.rs`)
- [ ] 3.8 Update docs: `README.md`, `docs/user-guide.md`, `AGENTS.md`

## 4. Session From Dirty Changes

- [ ] 4.1 Add `start_point: Option<String>` to `SessionInfo` in plugin protocol (`src/plugin/protocol.rs`)
- [ ] 4.2 Update git-worktree plugin to use `start_point` when provided instead of `resolve_start_point()`
- [ ] 4.3 Add `ez session from-dirty <name>` subcommand in `src/cli.rs`
- [ ] 4.4 Implement `session_from_dirty` in `src/session/mod.rs`: detect current session, check dirty state, stash push, create session with start_point=HEAD, stash pop in new worktree, rollback on failure
- [ ] 4.5 Add `session_from_dirty` keybind to `KeybindsConfig` (default `alt-s`)
- [ ] 4.6 Wire `alt-s` keybind in session action loop (`src/browser/mod.rs`) — invoke session-from-dirty flow
- [ ] 4.7 Update docs: `README.md`, `docs/user-guide.md`, `AGENTS.md`

## 5. Config & Keybinds Integration

- [ ] 5.1 Add all new config fields to `EzConfig` and `KeybindsConfig` in `src/config/model.rs`: `default_sort`, `sort_toggle`, `new_bare_session`, `session_from_dirty`
- [ ] 5.2 Update browser header rendering to show new keybind hints for all new actions
- [ ] 5.3 Update `docs/design.md` diagrams if data flow changes

## 6. Final Verification

- [ ] 6.1 Run `make check` (fmt + clippy + tests) — zero warnings, all tests pass
- [ ] 6.2 Run `make install-debug` and manual smoke test all 4 features
