## 1. Remove path display from session list

- [x] 1.1 Remove `→ <path>` suffix from interactive browser session items in `src/browser/mod.rs` (lines ~381-386 `path_info` block and its usage in the format string)
- [x] 1.2 Remove `(path)` suffix from CLI `ez session list` tree mode in `src/session/mod.rs` (lines ~729-734)
- [x] 1.3 Remove `(path)` suffix from CLI `ez session list` flat mode in `src/session/mod.rs` (lines ~707-711)
- [x] 1.4 Remove path suffix from `src/browser/preview.rs` session tree rendering (lines ~273, ~374) if present

## 2. OnAttachedSessions plugin hook

- [x] 2.1 Add `OnAttachedSessions` variant to `HookType` enum in `src/plugin/model.rs`
- [x] 2.2 Add `attached_sessions: Option<Vec<String>>` field to `HookResponse` in `src/plugin/protocol.rs`
- [x] 2.3 Add a helper function in `src/plugin/mod.rs` that runs `OnAttachedSessions` hooks across enabled plugins and unions the `attached_sessions` responses into a `HashSet<SessionId>`
- [x] 2.4 Include the full session list in the hook request so plugins can match their state against known session IDs and paths

## 3. Plugin implementations

- [x] 3.1 Implement `on_attached_sessions` in `plugins/tmux/tmux-plugin`: run `tmux list-sessions -F '#{session_name}|#{session_attached}|#{@ez_managed}'`, filter managed+attached, match against session list from request, return session IDs
- [x] 3.2 Add `on_attached_sessions` to tmux `manifest.toml` hooks list
- [x] 3.3 Implement `on_attached_sessions` in `plugins/zellij/zellij-plugin`: run `zellij list-sessions -n`, filter non-EXITED, match encoded mux names against sessions from request, return session IDs
- [x] 3.4 Add `on_attached_sessions` to zellij `manifest.toml` hooks list
- [x] 3.5 Implement `on_attached_sessions` in `plugins/herdr/herdr-plugin`: run `herdr worktree list --cwd <repo>`, match `open_workspace_id` against session paths from request, return session IDs
- [x] 3.6 Add `on_attached_sessions` to herdr `manifest.toml` hooks list

## 4. Aqua color rendering

- [x] 4.1 Update interactive browser session item rendering in `src/browser/mod.rs` to use cyan for attached session names instead of yellow
- [x] 4.2 Update CLI `ez session list` tree mode rendering in `src/session/mod.rs` to use cyan for attached session names
- [x] 4.3 Update CLI `ez session list` flat mode rendering in `src/session/mod.rs` to use cyan for attached session names

## 5. Integration

- [x] 5.1 Call the attached-sessions helper once per render cycle in `session_action_loop` before building SelectItem list
- [x] 5.2 Call the attached-sessions helper once in `list_sessions` CLI handler before print loop
- [x] 5.3 Verify existing tests pass (`cargo test`)
- [ ] 5.4 Manual test: open a tmux/zellij/herdr session, run `ez` and `ez session list`, confirm attached session appears in aqua and no path suffix is shown
