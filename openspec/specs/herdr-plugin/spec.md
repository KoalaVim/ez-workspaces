# Herdr Plugin

## Purpose

Bundled plugin that integrates herdr workspace management with the ez session lifecycle — open/focus on enter, close on delete, rename on rename, keybind to open from picker. The plugin is thin: ez owns worktrees and herdr discovers them as plain git worktrees; this plugin only bridges lifecycle events.

## Requirements

### Requirement: Herdr plugin manifest
The herdr plugin SHALL declare a `manifest.toml` with name `herdr`, hooks `on_session_create`, `on_session_delete`, `on_session_enter`, `on_session_exit`, `on_session_rename`, `on_bind`, and `on_attached_sessions`, executable `herdr-plugin`, one keybind (`alt-h` for `herdr_open` in the `session` context), and one config schema entry (`close_workspace_on_delete` bool defaulting to true).

#### Scenario: Manifest is valid
- **WHEN** the plugin system loads `plugins/herdr/manifest.toml`
- **THEN** it registers the herdr plugin with all seven hooks, the `alt-h` keybind, and the config option

### Requirement: Graceful degradation when herdr is unavailable
The plugin SHALL check that the `herdr` CLI is on PATH and that a herdr server is running before executing any herdr command. If either check fails, the plugin SHALL return `{success: true}` without error.

#### Scenario: herdr not installed
- **WHEN** any hook fires and `herdr` is not on PATH
- **THEN** the plugin returns `{success: true}` and takes no action

#### Scenario: herdr server not running
- **WHEN** any hook fires and `herdr status` does not report `status: running`
- **THEN** the plugin returns `{success: true}` and takes no action

### Requirement: Session create stores herdr path
On `on_session_create`, the plugin SHALL return `session_mutations.plugin_state.herdr_path` set to the session path, without opening a herdr workspace.

#### Scenario: New session records path
- **WHEN** `on_session_create` fires for a session at path `/repo/feat-branch`
- **THEN** the plugin returns `{success: true, session_mutations: {plugin_state: {herdr_path: "/repo/feat-branch"}}}`
- **AND** no herdr workspace is opened

### Requirement: Session enter opens herdr workspace
On `on_session_enter`, the plugin SHALL always open and focus a herdr workspace for the session path when herdr is available, and record `herdr_path` in plugin state. The open command SHALL be returned as a `post_shell_commands` entry.

#### Scenario: herdr available
- **WHEN** `on_session_enter` fires and herdr is available
- **THEN** the plugin returns a `post_shell_commands` entry that runs `herdr worktree open --cwd <repo_path> --path <session_path> --focus`
- **AND** records `herdr_path` in `session_mutations.plugin_state`

#### Scenario: herdr unavailable
- **WHEN** `on_session_enter` fires and herdr is not available
- **THEN** the plugin returns `{success: true}` with `herdr_path` in plugin state but no `post_shell_commands`

### Requirement: Keybind opens herdr workspace
On `on_bind`, the plugin SHALL open and focus a herdr workspace for the session path via `post_shell_commands`. When running inside herdr (`HERDR_PANE_ID` is set), the response SHALL include `accept: true` so the browser exits without relaunching.

#### Scenario: alt-h pressed inside herdr
- **WHEN** `on_bind` fires for the `herdr_open` bind and herdr is available and `HERDR_PANE_ID` is set
- **THEN** the plugin returns a `post_shell_commands` entry that opens the herdr workspace with `accept: true`

#### Scenario: alt-h pressed outside herdr
- **WHEN** `on_bind` fires for the `herdr_open` bind and herdr is available and `HERDR_PANE_ID` is not set
- **THEN** the plugin returns a `post_shell_commands` entry that opens the herdr workspace without `accept`

#### Scenario: alt-h when herdr unavailable
- **WHEN** `on_bind` fires but herdr is not available
- **THEN** the plugin returns `{success: true}` and ez falls back to its default behavior

### Requirement: Session delete closes herdr workspace
On `on_session_delete`, the plugin SHALL close the herdr workspace associated with the session path if `close_workspace_on_delete` is `true`. It SHALL first look up the workspace via `herdr worktree list --cwd <repo_path>`, and fall back to `herdr workspace list` (matching by `checkout_path`) if the worktree directory is already gone.

#### Scenario: Workspace closed on delete
- **WHEN** `on_session_delete` fires with `close_workspace_on_delete = true` and a herdr workspace is open for the session path
- **THEN** the plugin calls `herdr workspace close <workspace_id>` and returns `{success: true}`

#### Scenario: Worktree already removed
- **WHEN** `on_session_delete` fires but the worktree directory was already removed by git-worktree plugin
- **THEN** the plugin falls back to `herdr workspace list` to find the workspace by checkout_path and closes it

#### Scenario: close_workspace_on_delete disabled
- **WHEN** `on_session_delete` fires with `close_workspace_on_delete = false`
- **THEN** the plugin returns `{success: true}` without closing any workspace

### Requirement: Session rename renames herdr workspace
On `on_session_rename`, the plugin SHALL rename the herdr workspace associated with the session to the new name. It SHALL look up the workspace by `old_path` first (since the git-worktree plugin may have already moved the checkout), falling back to the current session path.

#### Scenario: Workspace renamed
- **WHEN** `on_session_rename` fires with `new_name = "new-feat"` and a herdr workspace exists for the session
- **THEN** the plugin calls `herdr workspace rename <workspace_id> "new-feat"` and returns `{success: true}`

### Requirement: Session exit is a no-op
On `on_session_exit`, the plugin SHALL return `{success: true}` without closing or modifying any herdr workspace.

#### Scenario: Exiting preserves workspace
- **WHEN** `on_session_exit` fires
- **THEN** the plugin returns `{success: true}` and takes no action on the herdr workspace
