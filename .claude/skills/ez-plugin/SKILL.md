---
name: "ez-plugin"
description: "Create and manage ez-workspaces plugins. Use when writing custom plugins, debugging plugin issues, or extending ez-workspaces with new functionality."
---

# ez-workspaces Plugin Development

## What This Skill Does

Guides the creation of external script plugins for ez-workspaces. Plugins are language-agnostic executables that communicate via JSON-over-stdio.

## Key Files

- `src/plugin/mod.rs` — Plugin dispatch, hook execution
- `src/plugin/model.rs` — PluginManifest, HookType enum
- `src/plugin/protocol.rs` — HookRequest, HookResponse types
- `src/plugin/runner.rs` — Process execution with timeout
- `plugins/git-worktree/` — Reference implementation (bash)
- `plugins/tmux/` — Reference implementation (bash)
- `docs/plugin-guide.md` — Full documentation

## Plugin Structure

```
~/.config/ez/plugins/my-plugin/
  manifest.toml       # Required: name, version, hooks, executable
  my-executable       # Any language, must be executable
```

### manifest.toml

```toml
name = "my-plugin"
version = "0.1.0"
description = "What it does"
hooks = ["on_session_create", "on_session_delete"]
executable = "my-executable"
```

## JSON Protocol

**Request (stdin)**:
```json
{
  "hook": "on_session_create",
  "repo": { "id": "...", "path": "...", "remote_url": "...", "default_branch": "..." },
  "session": { "id": "...", "name": "...", "parent_id": null, "path": null, "env": {}, "plugin_state": {}, "is_default": false },
  "config": { "plugin_state": {} }
}
```

**Response (stdout)**:
```json
{
  "success": true,
  "session_mutations": { "path": "/new/path", "env": {}, "plugin_state": {} },
  "repo_mutations": { "plugin_state": {} },
  "shell_commands": ["echo hello"]
}
```

## 10 Hook Types

`on_session_create`, `on_session_delete`, `on_session_enter`, `on_session_exit`, `on_session_rename`, `on_session_sync`, `on_repo_clone`, `on_repo_remove`, `on_plugin_init`, `on_plugin_deinit`

## Error Behavior

- Create/delete hooks: errors abort the operation
- Enter/exit hooks: errors are warnings (non-fatal)
- Timeout: 30s default (configurable via `plugin_timeout`)
- stderr: logged as diagnostics

## Quick Plugin Template (bash)

```bash
#!/usr/bin/env bash
set -euo pipefail
REQUEST=$(cat)
HOOK=$(echo "$REQUEST" | jq -r '.hook')

case "$HOOK" in
    on_session_create)
        echo '{"success": true, "session_mutations": {"env": {"MY_VAR": "hello"}}}'
        ;;
    *)
        echo '{"success": true}'
        ;;
esac
```
