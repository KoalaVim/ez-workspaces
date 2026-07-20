## Why

KoalaVim (managed by the `kv` CLI) shares a single environment across all workspaces. When switching between sessions, editor state (plugins, config, cache) bleeds across projects. Each ez-workspaces session should get its own isolated `kv` environment so editor state stays per-session.

## What Changes

- Add a new **bundled `kv` plugin** that hooks into the session lifecycle to manage `kv env` environments:
  - `on_session_create`: forks the `main` kv env into a session-specific env (`kv env fork main <session-name>`)
  - `on_session_delete`: deletes the session's kv env (`kv env delete <env-name>`)
  - `on_session_enter`: sets `KV_ENV` environment variable so kv uses the session-specific env
  - `on_session_rename`: renames the kv env to match the new session name (`kv env rename <old> <new>`)
- The plugin uses session `env` mutations to inject `KV_ENV=<env-name>` into the session, making it available to shell wrapper / tmux.
- Default sessions (main) map to the `main` kv env (no fork needed).
- The plugin is opt-in (user must `ez plugin enable kv`), not auto-enabled, since not all users use KoalaVim.

## Capabilities

### New Capabilities
- `kv-env-plugin`: Bundled plugin that manages per-session KoalaVim environments via the `kv env` CLI. Handles fork-on-create, delete-on-delete, env-injection-on-enter, and rename-on-rename.

### Modified Capabilities

(none — this is a pure addition via the plugin system, no existing specs change)

## Impact

- **New files**: `plugins/kv/manifest.toml`, `plugins/kv/kv-plugin` (bash script)
- **Modified files**: `src/plugin/bundled.rs` (register new bundled plugin)
- **Dependencies**: Requires `kv` CLI installed on the user's machine. Plugin gracefully no-ops if `kv` is not found.
- **Existing plugins**: No conflicts. Runs after git-worktree (doesn't mutate session path), alongside tmux.
