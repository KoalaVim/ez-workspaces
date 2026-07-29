## 1. Update tmux plugin to return post_shell_commands instead of killing directly

- [x] 1.1 Change `on_session_delete` in `plugins/tmux/tmux-plugin` to return `{"success": true, "post_shell_commands": ["tmux kill-session -t \"=<name>\""]}` instead of running `tmux kill-session` directly
- [x] 1.2 Remove the retry/sleep logic from the plugin's delete handler

## 2. Thread post_cmd_file to delete paths

- [x] 2.1 Add `post_cmd_file: Option<&Path>` parameter to `delete_session` in `src/session/mod.rs`
- [x] 2.2 Add `post_cmd_file: Option<&Path>` parameter to `delete_session_by_id` in `src/session/mod.rs`
- [x] 2.3 Update the `SessionCommand::Delete` dispatch to pass `post_cmd_file` through
- [x] 2.4 Update browser `session_action_loop` delete action to pass `post_cmd_file` to `delete_session_by_id`

## 3. Replace reaper with synchronous hooks + post-cmd

- [x] 3.1 In `delete_session`, replace `spawn_detached_reap` with synchronous `plugin::run_hooks(OnSessionDelete, ...)` and collect `post_shell_commands` from responses
- [x] 3.2 Write collected post_shell_commands to `post_cmd_file` (or run inline as fallback)
- [x] 3.3 Apply the same pattern in `delete_session_by_id`
- [x] 3.4 Remove `spawn_detached_reap` function
- [x] 3.5 Remove `reap_delete` function
- [x] 3.6 Remove `ReapPayload` struct
- [x] 3.7 Remove the `ReapDelete` variant from `SessionCommand` in `src/cli.rs`
- [x] 3.8 Remove the `ReapDelete` dispatch arm in `src/session/mod.rs`
- [x] 3.9 Remove the `reap-delete` match in `src/main.rs` (if separate from cli dispatch)

## 4. Clean up

- [x] 4.1 Remove `reap_delay_ms` references from config/plugin settings handling
- [x] 4.2 Remove unused imports (`setsid`, `libc`, serde for `ReapPayload`, etc.)

## 5. Verify

- [x] 5.1 Run `make check` to ensure build compiles cleanly with no warnings and tests pass
