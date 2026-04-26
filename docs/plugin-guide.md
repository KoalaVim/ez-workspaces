# Plugin Guide

## Overview

Plugins are external executables that hook into ez-workspaces lifecycle events. They receive a JSON request on stdin and return a JSON response on stdout. Any language works — shell, Python, Go, Rust, etc.

## Plugin Structure

```
~/.config/ez/plugins/my-plugin/
  manifest.toml       # Plugin metadata
  my-plugin-exec      # Executable (any language)
```

### manifest.toml

```toml
name = "my-plugin"
version = "0.1.0"
description = "What this plugin does"
hooks = ["on_session_create", "on_session_delete", "on_session_enter", "on_view", "on_view_select"]
executable = "my-plugin-exec"

# Register a custom view (appears as a keybind in the browser)
[[views]]
name = "my-view"
key = "ctrl-a"
label = "my view"
contexts = ["session", "repo", "owner", "workspace", "tree", "label"]

# Register action keybinds on selected items
[[binds]]
key = "alt-x"
name = "my_action"
label = "do something"
contexts = ["session"]

# Declare user-facing configuration options
[[config_schema]]
name = "auto_run"
type = "bool"
default = false
description = "Run automatically on session enter"
```

## Available Hooks

| Hook | When | Session in payload? |
|------|------|---------------------|
| `on_session_create` | After session metadata created | Yes |
| `on_session_delete` | Before session metadata removed | Yes |
| `on_session_enter` | User enters a session | Yes |
| `on_session_exit` | User exits a session | Yes |
| `on_session_rename` | Session renamed | Yes |
| `on_session_sync` | User requests sync | Yes |
| `on_repo_clone` | After repo cloned + registered | No |
| `on_repo_remove` | Before repo unregistered | No |
| `on_plugin_init` | Plugin enabled | No |
| `on_plugin_deinit` | Plugin disabled | No |
| `on_view` | User switches to plugin view | No |
| `on_view_select` | User selects item in plugin view | No |

## JSON Protocol

### Request (stdin)

```json
{
  "hook": "on_session_create",
  "repo": {
    "id": "personal-my-repo",
    "path": "/home/user/workspace/personal/my-repo",
    "remote_url": "git@github.com:user/my-repo.git",
    "default_branch": "main"
  },
  "session": {
    "id": "uuid-here",
    "name": "feature-auth",
    "parent_id": null,
    "path": null,
    "env": {},
    "plugin_state": {},
    "is_default": false
  },
  "config": {
    "plugin_state": {},
    "user_config": {}
  },
  "view_context": null,
  "bind_context": null
}
```

- `session` is `null` for repo-level hooks and view hooks
- `config.plugin_state` carries this plugin's per-repo state from previous invocations
- `config.user_config` carries user-facing settings from `[plugin_settings.<name>]` in config.toml
- `view_context` is present for `on_view` and `on_view_select` hooks (contains `view_name`, `selected_value`, `selected_display`)
- `bind_context` is present for `on_bind` hooks (contains `name`, `key`, `view`, `selected_value`, `selected_display`)

### Response (stdout)

```json
{
  "success": true,
  "error": null,
  "session_mutations": {
    "path": "/path/to/worktree",
    "env": { "MY_VAR": "value" },
    "plugin_state": { "my_key": "my_value" }
  },
  "repo_mutations": {
    "plugin_state": { "repo_key": "repo_value" }
  },
  "shell_commands": ["echo 'hello'"],
  "post_shell_commands": ["tmux switch-client -t mysession"],
  "cd_target": "/path/to/directory",
  "view_items": [
    { "display": "  Item 1", "value": "item-1" },
    { "display": "  Item 2", "value": "item-2" }
  ],
  "view_prompt": "my items",
  "view_preview_cmd": "cat {1}/README.md"
}
```

- All fields except `success` are optional
- `session_mutations` and `repo_mutations` use patch semantics (only included fields are merged)
- `shell_commands` are executed inside ez (before exit)
- `post_shell_commands` are executed in the user's shell *after* ez exits — use for commands that need the terminal (e.g., `tmux switch-client`)
- `cd_target` overrides the directory the shell wrapper will `cd` into
- `view_items`, `view_prompt`, `view_preview_cmd` are returned by `on_view` hooks to provide items for plugin views
- `error` is a string message if `success` is false
- Plugins have a 30-second timeout by default (configurable via `plugin_timeout` in config)

## Plugin Views

Plugins can register custom views that appear as keybinds alongside the built-in views (Tree, Workspace, Repo, Owner, Label). When the user presses the view key, ez calls the plugin's `on_view` hook, renders the returned items in fzf, and calls `on_view_select` when the user picks one.

### View flow

1. User presses the plugin's view key (e.g., `Ctrl-a`)
2. ez calls the plugin with `on_view` hook
3. Plugin returns `view_items` (display + value pairs), optional `view_prompt` and `view_preview_cmd`
4. ez renders items in fzf with all view-switch keys active
5. User selects an item (or switches to another view)
6. ez calls the plugin with `on_view_select` hook, passing the selected item in `view_context`
7. Plugin returns `post_shell_commands` and/or `cd_target`
8. ez writes post commands and exits; the shell wrapper executes them

### Conflict resolution

If a plugin view key conflicts with a core keybind (e.g., `ctrl-t` is already used by Tree view), the core keybind wins and the plugin view is skipped. A warning is logged in debug mode. Choose a non-conflicting key or let users remap core keybinds in `[keybinds]`.

## Plugin Configuration

Plugins can declare user-facing configuration options via `[[config_schema]]` in the manifest. Users set these in `config.toml`:

```toml
[plugin_settings.my-plugin]
auto_run = true
custom_path = "/usr/local/bin/my-tool"
```

The plugin receives these values in `config.user_config` on every hook invocation.

### Error Handling

- For `on_session_enter`/`on_session_exit`: plugin errors are warnings (non-fatal)
- For `on_session_create`/`on_session_delete`: plugin errors abort the operation
- stderr output is logged as diagnostics

## Example: Minimal Plugin (bash)

```bash
#!/usr/bin/env bash
set -euo pipefail

REQUEST=$(cat)
HOOK=$(echo "$REQUEST" | jq -r '.hook')
SESSION_NAME=$(echo "$REQUEST" | jq -r '.session.name // empty')

case "$HOOK" in
    on_session_create)
        echo "Creating session: $SESSION_NAME" >&2
        echo '{"success": true}'
        ;;
    *)
        echo '{"success": true}'
        ;;
esac
```

## Example: Python Plugin

```python
#!/usr/bin/env python3
import json, sys

request = json.load(sys.stdin)
hook = request["hook"]
session = request.get("session", {})

if hook == "on_session_enter":
    response = {
        "success": True,
        "session_mutations": {
            "env": {"MY_PROJECT_SESSION": session.get("name", "")}
        }
    }
else:
    response = {"success": True}

json.dump(response, sys.stdout)
```

## Installing a Plugin

1. Place the plugin directory under `~/.config/ez/plugins/`
2. Ensure the executable has execute permissions
3. Enable it: `ez plugin enable my-plugin`
