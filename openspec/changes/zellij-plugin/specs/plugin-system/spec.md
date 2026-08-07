## MODIFIED Requirements

### Requirement: Bundled plugins
The system SHALL embed bundled plugins (git-worktree, tmux, zellij, cursor-mcp-auth, cursor-trusted-workspace, cursor-mcp-approvals, kv) in the binary. They are auto-extracted to the plugin directory on first use and auto-updated when the bundled version changes.

#### Scenario: First-run extraction
- **WHEN** user enables a bundled plugin and it does not exist in the plugin directory
- **THEN** system extracts the plugin files from the binary to `~/.config/ez/plugins/<name>/`

#### Scenario: Auto-update on version change
- **WHEN** the bundled plugin version differs from the installed version
- **THEN** system overwrites the installed plugin files with the new version

#### Scenario: Zellij plugin available out of the box
- **WHEN** user runs `ez plugin list`
- **THEN** `zellij` appears as an available bundled plugin alongside `tmux`

## ADDED Requirements

### Requirement: Plugin process environment
When invoking a plugin executable, the system SHALL set `EZ_CONFIG_DIR` to the resolved config directory and `EZ_BIN` to the absolute path of the running `ez` binary, so plugins can shell out to ez without depending on `PATH` or on the shell wrapper function.

#### Scenario: EZ_BIN points at the running binary
- **WHEN** any hook is invoked
- **THEN** the plugin process environment contains `EZ_BIN` set to the path returned by the current executable lookup
- **AND** invoking `"$EZ_BIN" repo list --json` from the plugin succeeds

#### Scenario: Config dir still provided
- **WHEN** any hook is invoked
- **THEN** the plugin process environment still contains `EZ_CONFIG_DIR`
