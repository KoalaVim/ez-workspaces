## Context

Session deletion currently uses a two-phase approach:
1. Synchronous: Remove session from store, print confirmation
2. Async (detached reaper): Run `OnSessionDelete` plugin hooks (worktree removal + tmux kill)

The reaper exists because `tmux kill-session` can destroy the controlling terminal (if the user is inside the session being deleted), which would SIGHUP the ez process before cleanup finishes. The reaper runs in a new process session via `setsid()` to survive this.

However, the `ez` shell wrapper already solves this problem: commands written to `post_cmd_file` execute AFTER ez exits. If we move `tmux kill-session` to post-cmd, the kill happens after all Rust code has completed and returned.

## Goals / Non-Goals

**Goals:**
- Eliminate the reaper process, temp file serialization, and `reap-delete` subcommand
- Run worktree removal synchronously (it's safe — doesn't kill terminals)
- Write `tmux kill-session` to `post_cmd_file` for the shell wrapper to execute
- Maintain identical user-facing behavior (session deleted, tmux session killed)

**Non-Goals:**
- Changing how `on_session_enter` or `on_session_create` hooks work
- Modifying the shell wrapper function itself (it already supports post-cmd)
- Adding new CLI flags

## Decisions

1. **Synchronous worktree removal + post-cmd tmux kill**: The git-worktree plugin's `on_session_delete` hook (which removes the worktree directory) runs synchronously during the delete command. Only the tmux kill moves to post-cmd. Rationale: worktree removal doesn't kill terminals so there's no reason to defer it.

2. **Plugin hook still runs, response routes differently**: The `OnSessionDelete` hook still fires against the tmux plugin. But instead of the plugin executing `tmux kill-session` directly, it returns the command in `post_shell_commands`. The caller writes those to `post_cmd_file`.

3. **Fallback when no post_cmd_file**: When `post_cmd_file` is `None` (e.g., user calls `ez session delete` without the shell wrapper, or from a script), run the post-shell commands inline via `plugin::runner::run_shell_commands`. This preserves backward compatibility.

4. **Thread post_cmd_file through delete paths**: Both `delete_session` (CLI) and `delete_session_by_id` (browser) receive `post_cmd_file`. The session dispatch already has it; we just need to pass it down.

5. **Remove reap_delay_ms config**: The `reap_delay_ms` tmux plugin setting becomes unnecessary since there's no timing race.

## Risks / Trade-offs

- [User deletes session outside shell wrapper (e.g., `command ez session delete foo`)] → Fallback runs tmux kill inline. If the killed session is the current terminal, the terminal dies mid-command. This is the same behavior as the old reaper without setsid. Acceptable because: (a) the shell wrapper is the documented way to use ez, and (b) deleting your own active session is an edge case.
- [Cascade delete of multiple sessions] → Multiple `tmux kill-session` commands written to post-cmd. They execute sequentially after ez exits. No issue.
- [Browser delete path] → The browser already has `post_cmd_file` in scope. After delete, the browser loop continues (user sees the session list without the deleted session). The tmux kill fires after the user eventually exits the browser.
