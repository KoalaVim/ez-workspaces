## MODIFIED Requirements

### Requirement: Enable and disable plugins

The system SHALL support enabling and disabling plugins via CLI commands. Enabled plugins are listed in `config.toml` under `[plugins] enabled`. Enabling runs `OnPluginInit`; disabling runs `OnPluginDeinit`.

#### Scenario: Enable plugin
- **WHEN** user runs `ez plugin enable git-worktree`
- **THEN** system adds `git-worktree` to the enabled list and runs `OnPluginInit`

#### Scenario: Disable plugin
- **WHEN** user runs `ez plugin disable tmux`
- **THEN** system removes `tmux` from the enabled list and runs `OnPluginDeinit`

#### Scenario: List plugins as table
- **WHEN** user runs `ez plugin list`
- **THEN** system SHALL display all available plugins in a table with columns: Name, Status, Description
- **AND** the table SHALL include a header row with column names
- **AND** the table SHALL include a separator line between the header and data rows
- **AND** column widths SHALL adapt to the longest value in each column
- **AND** the Name column SHALL be colored cyan
- **AND** the Status column SHALL display "enabled" in green or "disabled" in dimmed text

#### Scenario: No plugins found
- **WHEN** user runs `ez plugin list` and no plugins exist in the plugins directory
- **THEN** system SHALL display a warning message with the plugins directory path
