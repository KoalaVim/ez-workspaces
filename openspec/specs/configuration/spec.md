# Configuration

## Purpose

Manage the global configuration for ez-workspaces via a TOML file at `~/.config/ez/config.toml`. The configuration system supports interactive guided setup, individual key get/set, workspace root management, and direct file editing. All config values have sensible defaults so ez works out of the box.

## Requirements

### Requirement: Config file format
The system SHALL use TOML format for the configuration file at `~/.config/ez/config.toml`. The config SHALL be deserialized into an `EzConfig` struct with default values for all fields.

#### Scenario: Load config with defaults
- **WHEN** config file does not exist or is empty
- **THEN** system uses default values for all fields (workspace_roots: empty, selector.backend: "fzf", default_select_by: "workspace", on_enter: "cd", on_create: "none", plugin_timeout: 30)

#### Scenario: Partial config
- **WHEN** config file contains only `workspace_roots = ["~/workspace"]`
- **THEN** all other fields use their defaults

### Requirement: Workspace roots
The config SHALL maintain a list of workspace root directories that the browser uses for the Workspace view and Tree view. Roots support `~` for home directory expansion.

#### Scenario: Add root
- **WHEN** user runs `ez config add-root ~/workspace/personal`
- **THEN** the path is added to `workspace_roots` in config

#### Scenario: Remove root
- **WHEN** user runs `ez config remove-root ~/workspace/personal`
- **THEN** the path is removed from `workspace_roots`

### Requirement: Interactive guided setup
The system SHALL provide an interactive `ez config` (or `ez config init`) command that walks the user through: workspace roots, default shell, selector backend, plugin enablement, and timeout configuration.

#### Scenario: First-time setup
- **WHEN** user runs `ez config` with no existing config
- **THEN** system prompts through each configuration area interactively and writes the result

### Requirement: Get and set config values
The system SHALL support getting and setting individual config values via CLI using dot-notation keys (e.g. `selector.backend`, `plugin_timeout`).

#### Scenario: Set a value
- **WHEN** user runs `ez config set default_select_by tree`
- **THEN** system updates the config file with `default_select_by = "tree"`

#### Scenario: Get a value
- **WHEN** user runs `ez config get default_select_by`
- **THEN** system prints the current value

### Requirement: Show and edit config
The system SHALL provide `ez config show` to display the current configuration and `ez config edit` to open the config file in the user's editor (from config `editor` field, or `$EDITOR`, or fallback).

#### Scenario: Show config
- **WHEN** user runs `ez config show`
- **THEN** system prints the current config as TOML

#### Scenario: Edit config
- **WHEN** user runs `ez config edit`
- **THEN** system opens `~/.config/ez/config.toml` in the configured editor

### Requirement: Keybinds configuration
The config SHALL support a `[keybinds]` section with configurable keys for: `new_session` (default alt-n), `delete_session` (alt-d), `rename_session` (alt-r), `view_tree` (ctrl-t), `view_workspace` (ctrl-w), `view_repo` (ctrl-e), `view_owner` (ctrl-o), `view_label` (ctrl-g), `edit_labels` (alt-l).

#### Scenario: Custom keybind
- **WHEN** config has `[keybinds] new_session = "alt-c"`
- **THEN** the browser uses Alt-c for creating new sessions instead of Alt-n

### Requirement: Session name stages configuration
The config SHALL support a `session_name_stages` array defining the interactive name builder stages. Each stage has a `name`, `kind` (choice or text), and optional `choices` list.

#### Scenario: Custom stages
- **WHEN** config defines stages with custom choices like team prefixes
- **THEN** the name builder prompts through those custom stages

#### Scenario: Default stages
- **WHEN** no stages are configured
- **THEN** system uses defaults: prefix (feat/fix/chore), ticket-prefix, ticket-number

### Requirement: On-enter and on-create actions
The config SHALL support `on_enter` (default "cd") and `on_create` (default "none") fields that control what happens when a session is entered or created. Values can be "cd", "none", or a plugin-bind label/name. These are overridable per-invocation via `--on-enter` and `--on-create` CLI flags.

#### Scenario: Override on CLI
- **WHEN** user runs `ez --on-enter tmux`
- **THEN** session enter action uses the tmux bind regardless of config

### Requirement: Plugin settings
The config SHALL support a `[plugin_settings.<name>]` section for per-plugin user-facing settings. These are passed to plugins as `config.user_config` in every hook request.

#### Scenario: Tmux settings
- **WHEN** config has `[plugin_settings.tmux] auto_attach = true`
- **THEN** the tmux plugin receives `{"auto_attach": true}` in its hook requests

### Requirement: Fzf configuration
The config SHALL support a `[fzf]` section with `height` (default "90%") and `extra_opts` (additional fzf flags). A deprecated `[selector] fzf_opts` field is also supported for backward compatibility.

#### Scenario: Fzf height
- **WHEN** config has `[fzf] height = "100%"`
- **THEN** all fzf invocations use `--height 100%`
