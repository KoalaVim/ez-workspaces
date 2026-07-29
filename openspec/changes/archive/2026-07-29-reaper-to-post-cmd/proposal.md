## Why

Session deletion currently spawns a detached "reaper" process (`ez session reap-delete`) via `setsid()` to run `OnSessionDelete` plugin hooks (tmux kill-session, git worktree remove). This exists because the tmux kill could destroy the terminal before cleanup finished. The indirection adds complexity (temp file serialization, 200ms delay, retry logic) and makes debugging harder. Since the `ez` shell wrapper already has a `--post-cmd-file` mechanism for running commands after ez exits, we can use it to kill the tmux session — the kill happens deterministically after all cleanup is done.

## What Changes

- **Remove the detached reaper process** (`spawn_detached_reap`, `reap_delete`, `ReapPayload`, the hidden `reap-delete` subcommand).
- **Run worktree removal synchronously** during the delete operation (before ez exits).
- **Write `tmux kill-session` to `post_cmd_file`** so the shell wrapper executes it after ez returns. This eliminates the race condition entirely.
- **Thread `post_cmd_file` to `delete_session`** (CLI path) and `delete_session_by_id` (browser path).
- **Update the tmux plugin** `on_session_delete` hook to return the kill command in `post_shell_commands` instead of executing it directly.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `shell-integration`: The post-cmd-file mechanism gains a new consumer (session delete writes tmux kill commands to it).

## Impact

- `src/session/mod.rs`: Remove `spawn_detached_reap`, `reap_delete`, `ReapPayload`; add `post_cmd_file` param to delete functions; run plugin hooks synchronously.
- `src/cli.rs`: Remove the `ReapDelete` subcommand.
- `src/main.rs`: Remove the `ReapDelete` dispatch arm.
- `src/browser/mod.rs`: Thread `post_cmd_file` to delete actions in the session action loop.
- `plugins/tmux/tmux-plugin`: Change `on_session_delete` to return `post_shell_commands` instead of running `tmux kill-session` directly.
