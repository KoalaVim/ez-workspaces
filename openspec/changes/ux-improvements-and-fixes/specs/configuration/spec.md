# Configuration (Delta)

## MODIFIED Requirements

### Requirement: Keybinds configuration

The config SHALL support a `[keybinds]` section with configurable keys for: `new_session` (default alt-n), `delete_session` (alt-d), `rename_session` (alt-r), `cd_session` (alt-c), `view_tree` (ctrl-t), `view_workspace` (ctrl-w), `view_repo` (ctrl-e), `view_owner` (ctrl-o), `view_label` (ctrl-g), `edit_labels` (alt-l).

#### Scenario: Custom keybind

- **WHEN** config has `[keybinds] new_session = "alt-c"`
- **THEN** the browser uses Alt-c for creating new sessions instead of Alt-n

#### Scenario: Cd session keybind

- **WHEN** config has `[keybinds] cd_session = "alt-g"`
- **THEN** the browser uses Alt-g for the cd action instead of the default Alt-c

#### Scenario: Default cd keybind

- **WHEN** no `cd_session` keybind is configured
- **THEN** the browser uses Alt-c as the default keybind for cd-ing into a session

## ADDED Requirements

### Requirement: Name builder modes configuration

The config SHALL support a `name_builder_modes` field that specifies which modes are available in the interactive name builder mode picker. The value is an array of mode identifiers. The default includes all modes: `["full_name", "build_from_parts", "github_pr", "jira_url"]`.

#### Scenario: Configure subset of modes

- **WHEN** config has `name_builder_modes = ["full_name", "build_from_parts"]`
- **THEN** only "Full name" and "Build from parts" appear in the mode picker

#### Scenario: Default modes

- **WHEN** no `name_builder_modes` field is present in config
- **THEN** all four modes are available: full_name, build_from_parts, github_pr, jira_url

#### Scenario: Single mode skips picker

- **WHEN** config has `name_builder_modes = ["build_from_parts"]`
- **THEN** the mode picker is skipped and the system enters "Build from parts" directly

### Requirement: Browser loop configuration

The config SHALL support a `browser_loop` boolean field (default `true`) that controls whether the return-to-ez loop is active after tmux detach. This can also be overridden per-invocation via the `--no-loop` CLI flag.

#### Scenario: Disable loop via config

- **WHEN** config has `browser_loop = false`
- **THEN** the shell wrapper does not re-enter the browser after tmux detach

#### Scenario: Override via CLI flag

- **WHEN** user runs `ez --no-loop`
- **THEN** the loop is disabled for this invocation regardless of config
