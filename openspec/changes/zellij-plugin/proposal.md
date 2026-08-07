## Why

ez-workspaces ships a bundled tmux plugin that turns every session into a multiplexer session (create, attach, rename, kill, plus a browser view of running sessions). Zellij users get none of that: the browser has no attach bind, `on_enter = "tmux"` has no counterpart, and `ez cd-to-session` / current-session detection are hard-wired to tmux user options. Zellij 0.44 exposes every primitive needed (`attach --create-background`, `action switch-session --cwd`, remote `action rename-session`, `delete-session --force`), so the gap is purely missing integration.

## What Changes

- Add a bundled **zellij plugin** mirroring the tmux plugin: creates a background zellij session per ez session, attaches/switches on demand, renames on session rename, deletes on session delete, and registers a browser view of all ez sessions with a running/stopped marker.
  - Browser keys: `Alt-z` (attach/switch to selected session), `Ctrl-z` (zellij view). Distinct from the tmux plugin's `Alt-a` / `Ctrl-a` so both can be enabled at once.
  - Config: `auto_attach`, `force_delete`, `reap_delay_ms`.
  - `on_enter = "zellij"` / `on_create = "zellij"` work through the existing bind-label resolution, no core change needed.
- Generalize **current-session detection** (`src/session/current.rs`) from "tmux user options" to a multiplexer-agnostic resolver: tmux user options first, then `$ZELLIJ_SESSION_NAME` matched against registered repo/session names, then cwd. Zellij has no per-session key/value store, so the mapping is derived from the deterministic session-name encoding rather than stamped metadata.
- Generalize **`ez cd-to-session`** to work inside a zellij session (today it errors out unless `$TMUX` is set).
- Add **`ez session list --all --json`** so plugins can enumerate every repo's sessions in one call instead of re-implementing TOML parsing in awk (the tmux plugin does this today).
- Pass **`EZ_BIN`** (path to the running ez binary) to plugin processes so plugins can shell out to ez reliably.
- Clear zellij env vars alongside `TMUX` when spawning the detached delete-reaper, so tearing down the session you are currently attached to does not kill the reaper.
- Read the reaper's `reap_delay_ms` from whichever multiplexer plugin is configured (tmux, then zellij) instead of tmux only.
- Docs: user guide (keybinds, views, `on_enter`, cd-to-session), plugin guide (`EZ_BIN`), README plugin list.

Not in scope: sharing code between the tmux and zellij plugins (bundled plugins are single-file executables), zellij layouts, and pushing env changes into an already-running zellij session (zellij has no `set-environment` equivalent — documented as a limitation).

## Capabilities

### New Capabilities
- `zellij-plugin`: bundled plugin that maps ez sessions onto zellij sessions — create/attach/switch/rename/delete, the zellij browser view, name encoding, and its config schema.

### Modified Capabilities
- `plugin-system`: bundled-plugin roster gains `zellij`; plugin processes receive `EZ_BIN` in their environment.
- `session-management`: current-session resolution becomes multiplexer-agnostic (tmux user options → zellij session name → cwd); `session list` gains `--all`; the detached delete-reaper clears zellij env vars and resolves `reap_delay_ms` from tmux or zellij settings.
- `shell-integration`: `ez cd-to-session` works inside zellij; the return-to-ez-after-detach loop is specified for any multiplexer bind, not just tmux.
- `configuration`: `[plugin_settings.zellij]` keys (`auto_attach`, `force_delete`, `reap_delay_ms`) and `on_enter`/`on_create` accepting `"zellij"`.

## Impact

- New files: `plugins/zellij/manifest.toml`, `plugins/zellij/zellij-plugin`.
- Modified: `src/plugin/bundled.rs` (register bundled plugin), `src/plugin/runner.rs` (`EZ_BIN`), `src/session/current.rs` (resolver), `src/session/mod.rs` (reaper env + `reap_delay_ms` lookup, `list --all`), `src/main.rs` (`cd_to_session`), `src/cli.rs` (`--all` flag, help text).
- External dependency: `zellij` ≥ 0.40 on `PATH` (for `attach --create-background` and `action switch-session`); the plugin degrades to a no-op with a clear error when zellij is missing.
- No breaking changes: tmux behavior, keys, and config are untouched.
