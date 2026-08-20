## Why

ez-workspaces already bundles multiplexer plugins for tmux and zellij, but has no integration for herdr — a workspace manager that maps worktrees to persistent workspaces with panes and agents. Users who run herdr alongside ez must manually open/close herdr workspaces when switching ez sessions, losing the seamless lifecycle management the other multiplexer plugins provide.

## What Changes

- Add a new bundled `herdr` plugin under `plugins/herdr/` with:
  - `manifest.toml` declaring hooks (`on_session_create`, `on_session_delete`, `on_session_enter`, `on_session_exit`, `on_session_rename`, `on_bind`), a keybind (`alt-h`), and config schema (`auto_open`, `close_workspace_on_delete`).
  - `herdr-plugin` shell executable that maps ez session lifecycle events to herdr workspace open/close/rename/focus commands.
- The plugin follows the same thin-integration pattern as the tmux and zellij plugins: ez owns worktrees, herdr discovers them as plain git worktrees, and the plugin only bridges lifecycle events.

## Capabilities

### New Capabilities
- `herdr-plugin`: Bundled plugin that integrates herdr workspace management with ez session lifecycle — open/focus on enter, close on delete, rename on rename, keybind to open from picker.

### Modified Capabilities

_(none — no existing spec requirements change)_

## Impact

- New files: `plugins/herdr/manifest.toml`, `plugins/herdr/herdr-plugin`
- No changes to existing plugins or core code
- No new dependencies — the plugin shells out to `herdr` CLI and gracefully skips when herdr is not installed or not running
