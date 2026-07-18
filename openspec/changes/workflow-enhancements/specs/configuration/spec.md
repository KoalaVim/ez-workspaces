# Configuration (Delta)

## MODIFIED Requirements

### Requirement: Config file format
The system SHALL use TOML format for the configuration file at `~/.config/ez/config.toml`. The config SHALL be deserialized into an `EzConfig` struct with default values for all fields.

#### Scenario: Load config with defaults
- **WHEN** config file does not exist or is empty
- **THEN** system uses default values for all fields (workspace_roots: empty, selector.backend: "fzf", default_select_by: "workspace", on_enter: "cd", on_create: "none", plugin_timeout: 30, default_sort: "alpha")

#### Scenario: Partial config
- **WHEN** config file contains only `workspace_roots = ["~/workspace"]`
- **THEN** all other fields use their defaults (including `default_sort = "alpha"`)

### Requirement: Keybinds configuration
The config SHALL support a `[keybinds]` section with configurable keys for: `new_session` (default alt-n), `delete_session` (alt-d), `rename_session` (alt-r), `cd_session` (alt-c), `view_tree` (ctrl-t), `view_workspace` (ctrl-w), `view_repo` (ctrl-e), `view_owner` (ctrl-o), `view_label` (ctrl-g), `edit_labels` (alt-l), `bare_session` (default alt-shift-n), `session_from_dirty` (default alt-s), `sort_toggle` (default ctrl-s).

#### Scenario: Custom keybind
- **WHEN** config has `[keybinds] new_session = "alt-c"`
- **THEN** the browser uses Alt-c for creating new sessions instead of Alt-n

#### Scenario: Cd session keybind
- **WHEN** config has `[keybinds] cd_session = "alt-g"`
- **THEN** the browser uses Alt-g for the cd action instead of the default Alt-c

#### Scenario: Default cd keybind
- **WHEN** no `cd_session` keybind is configured
- **THEN** the browser uses Alt-c as the default keybind for cd-ing into a session

#### Scenario: Bare session keybind override
- **WHEN** config has `[keybinds] bare_session = "alt-b"`
- **THEN** the browser uses Alt-b for bare session creation instead of Alt-Shift-N

#### Scenario: Sort toggle keybind override
- **WHEN** config has `[keybinds] sort_toggle = "ctrl-r"`
- **THEN** the browser uses Ctrl-r for sort toggle instead of ctrl-s

#### Scenario: Session from dirty keybind override
- **WHEN** config has `[keybinds] session_from_dirty = "alt-shift-s"`
- **THEN** the browser uses Alt-Shift-S for session-from-dirty instead of alt-s

## ADDED Requirements

### Requirement: Default sort configuration
The config SHALL support a `default_sort` field at the top level with values `"alpha"` or `"lru"` (default `"alpha"`). This determines the initial sort order when the browser is launched.

#### Scenario: Default alpha sort
- **WHEN** config has `default_sort = "alpha"` or the field is absent
- **THEN** browser starts with alphabetical sorting

#### Scenario: Default LRU sort
- **WHEN** config has `default_sort = "lru"`
- **THEN** browser starts with items sorted by last-accessed time descending

#### Scenario: Invalid sort value
- **WHEN** config has `default_sort = "invalid"`
- **THEN** system falls back to `"alpha"` and logs a warning
