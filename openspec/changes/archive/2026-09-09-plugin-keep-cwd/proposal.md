## Why

With `on_create = "herdr"` (or `on_enter = "herdr"`), ez opens the new session in a herdr workspace *and* cds the shell it was invoked from into the new worktree. The user asked for herdr to handle navigation; the shell they created from should stay where it is. The cause is in `accept_session`: when a plugin bind reports a navigation effect but no explicit `cd_target`, ez writes the session path to the cd-file anyway (`src/browser/mod.rs:410-412`), so any plugin that navigates elsewhere (a separate multiplexer workspace) still drags the invoking shell along.

## What Changes

- Add `keep_cwd` (bool, default false) to the plugin `HookResponse`: a plugin that navigated on its own tells ez not to cd the invoking shell.
- `accept_session` honors `keep_cwd`: no implicit `cd_target` write, no fallback cd. Session env exports are still written as post-commands.
- The herdr plugin sets `keep_cwd: true` on its `on_bind` response when running inside herdr (`HERDR_PANE_ID` set) — that branch only *focuses* another herdr workspace, so the invoking shell stays put. Outside herdr the response attaches herdr in the current terminal, where the cd is still wanted after detach.
- No change for tmux/zellij: they omit `keep_cwd` and keep today's behavior.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `plugin-system`: `HookResponse` gains `keep_cwd`.
- `session-management`: the enter/create action's implicit cd and cd-fallback are suppressed when the bind response sets `keep_cwd`.
- `herdr-plugin`: `on_bind` inside herdr returns `keep_cwd: true`.

## Impact

- `src/plugin/protocol.rs` — new `HookResponse` field.
- `src/browser/mod.rs` — `accept_session` cd decisions.
- `plugins/herdr/herdr-plugin` — `on_bind` response.
- `docs/plugin-guide.md` — document `keep_cwd`.
- No shell-wrapper change: an empty cd-file already means "don't cd".
