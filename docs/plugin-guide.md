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
hooks = ["on_session_create", "on_session_delete", "on_session_enter"]
executable = "my-plugin-exec"
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
    "plugin_state": {}
  }
}
```

- `session` is `null` for repo-level hooks (`on_repo_clone`, `on_repo_remove`)
- `config.plugin_state` carries this plugin's per-repo state from previous invocations

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
  "shell_commands": ["echo 'hello'"]
}
```

- All fields except `success` are optional
- `session_mutations` and `repo_mutations` use patch semantics (only included fields are merged)
- `shell_commands` are executed after the hook completes (useful for `on_session_enter`)
- `error` is a string message if `success` is false
- Plugins have a 30-second timeout by default (configurable via `plugin_timeout` in config)

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
