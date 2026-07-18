# Plugin System

## Purpose

Provide an extensible plugin architecture that lets external scripts or executables hook into the session and repo lifecycle, register custom keybinds and views, and declare configuration options. Plugins communicate via a JSON-over-stdio protocol. Bundled plugins (git-worktree, tmux) are embedded in the binary and auto-extracted on first use.

## Requirements

### Requirement: Plugin manifest
Each plugin SHALL have a `manifest.toml` in its plugin directory declaring: name, version, description, hooks (list of `HookType`), executable filename, optional binds, optional views, optional config schema, and a `mutates_session_path` flag.

#### Scenario: Manifest with hooks and binds
- **WHEN** a plugin directory contains a `manifest.toml`
- **THEN** system reads the manifest to determine which hooks the plugin handles, what keybinds it registers, and what views it provides

### Requirement: Hook types
The system SHALL support 13 hook types: `OnSessionCreate`, `OnSessionDelete`, `OnSessionEnter`, `OnSessionExit`, `OnSessionRename`, `OnSessionSync`, `OnRepoClone`, `OnRepoRemove`, `OnPluginInit`, `OnPluginDeinit`, `OnBind`, `OnView`, `OnViewSelect`.

#### Scenario: Session lifecycle hooks
- **WHEN** a session is created, deleted, entered, exited, or renamed
- **THEN** system invokes the corresponding hook on all enabled plugins that declare it

#### Scenario: Repo lifecycle hooks
- **WHEN** a repo is cloned or removed
- **THEN** system invokes `OnRepoClone` or `OnRepoRemove` on applicable plugins

### Requirement: JSON-over-stdio protocol
The system SHALL communicate with plugins by spawning the executable, writing a JSON `HookRequest` to stdin, closing stdin, and reading a JSON `HookResponse` from stdout. Diagnostics go to stderr.

#### Scenario: Plugin invocation
- **WHEN** a hook is triggered
- **THEN** system spawns the plugin executable, writes a JSON request to stdin with hook type, session/repo context, and user config, then reads the JSON response from stdout

#### Scenario: Plugin timeout
- **WHEN** a plugin does not respond within the configured `plugin_timeout` seconds
- **THEN** system returns `PluginTimeout` error

### Requirement: Hook request context
The `HookRequest` SHALL include: `hook` (hook type), `session` (session data if applicable), `repo` (repo entry), `repo_meta` (repo metadata), `config` (plugin-specific user config from `plugin_settings`), and hook-specific context (`BindContext` for `OnBind`, `ViewContext` for `OnView`/`OnViewSelect`).

#### Scenario: Bind context
- **WHEN** `OnBind` hook is invoked
- **THEN** request includes `bind_context` with `bind_name`, `key`, `view_context`, `selected_id`, and `selected_name`

#### Scenario: View select context
- **WHEN** `OnViewSelect` hook is invoked
- **THEN** request includes `view_context` with `view_name` and `selected_value` (the item the user picked)

### Requirement: Hook response mutations
The `HookResponse` SHALL support: `session_mutations` (modify session fields like path, env, plugin_state), `repo_mutations` (modify repo metadata fields), `shell_commands` (run inline during ez execution), `post_shell_commands` (written to post-cmd-file, sourced by shell wrapper after ez exits), `cd_target` (path to cd into), and `view_items` (list of items for `OnView`).

#### Scenario: Session path mutation
- **WHEN** git-worktree plugin responds to `OnSessionCreate`
- **THEN** response includes `session_mutations` with `path` set to the new worktree path

#### Scenario: Post-shell commands
- **WHEN** tmux plugin responds to `OnBind`
- **THEN** response includes `post_shell_commands` like `tmux switch-client -t session-name`

### Requirement: Plugin execution ordering
Plugins with `mutates_session_path = true` SHALL run before plugins without it for each hook. This ensures that downstream plugins see the resolved session path.

#### Scenario: Worktree before tmux
- **WHEN** `OnSessionCreate` fires with both git-worktree and tmux plugins enabled
- **THEN** git-worktree runs first (sets `session.path`), then tmux runs (sees the resolved path)

### Requirement: Plugin binds
Plugins SHALL register keybinds via manifest `[[binds]]` entries. Each bind has a `key`, `name`, `label`, optional `description`, and a list of `contexts` (e.g. `["session"]`). When the user presses the keybind in the browser, the system invokes `OnBind` with the bind context.

#### Scenario: Tmux attach bind
- **WHEN** user presses the tmux plugin's keybind on a session
- **THEN** system invokes `OnBind` with the bind name and session context
- **THEN** plugin returns `post_shell_commands` to attach/switch tmux session

### Requirement: Plugin views
Plugins SHALL register views via manifest `[[views]]` entries. Each view has a `name`, `key`, `label`, and `contexts`. When the user switches to the view, the system invokes `OnView` to get items, then `OnViewSelect` when an item is selected.

#### Scenario: Tmux session view
- **WHEN** user presses Ctrl-a (tmux view keybind)
- **THEN** system calls `OnView`, plugin returns a list of tmux sessions as `view_items`
- **THEN** fzf displays the items; on selection, system calls `OnViewSelect`

### Requirement: Plugin config schema
Plugins SHALL declare user-facing configuration fields via manifest `[[config_schema]]` entries. Each field has a `name`, `type` (bool/string/int), optional `default`, and optional `description`. These values are read from `[plugin_settings.<plugin_name>]` in the user's config.

#### Scenario: Tmux auto_attach setting
- **WHEN** user sets `[plugin_settings.tmux] auto_attach = true`
- **THEN** the tmux plugin receives `auto_attach = true` in its `config.user_config` field in every hook request

### Requirement: Bundled plugins
The system SHALL embed bundled plugins (git-worktree, tmux) in the binary. They are auto-extracted to the plugin directory on first use and auto-updated when the bundled version changes.

#### Scenario: First-run extraction
- **WHEN** user enables a bundled plugin and it does not exist in the plugin directory
- **THEN** system extracts the plugin files from the binary to `~/.config/ez/plugins/<name>/`

#### Scenario: Auto-update on version change
- **WHEN** the bundled plugin version differs from the installed version
- **THEN** system overwrites the installed plugin files with the new version

### Requirement: Enable and disable plugins
The system SHALL support enabling and disabling plugins via CLI commands. Enabled plugins are listed in `config.toml` under `[plugins] enabled`. Enabling runs `OnPluginInit`; disabling runs `OnPluginDeinit`.

#### Scenario: Enable plugin
- **WHEN** user runs `ez plugin enable git-worktree`
- **THEN** system adds `git-worktree` to the enabled list and runs `OnPluginInit`

#### Scenario: Disable plugin
- **WHEN** user runs `ez plugin disable tmux`
- **THEN** system removes `tmux` from the enabled list and runs `OnPluginDeinit`

#### Scenario: List plugins
- **WHEN** user runs `ez plugin list`
- **THEN** system shows all available plugins with their enabled/disabled status
