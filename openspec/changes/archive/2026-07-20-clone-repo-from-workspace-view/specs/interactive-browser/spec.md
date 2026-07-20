## MODIFIED Requirements

### Requirement: Workspace view with drill-down
The Workspace view SHALL first present a list of configured workspace roots. On selection, it SHALL drill into directories level by level until a git repo is found, then transition to the session picker. Directories with a `.git` folder are shown with branch info; hidden directories (starting with `.`) are excluded. The drill-down SHALL support action keybinds including a clone keybind that allows cloning a new repo into the currently browsed directory.

#### Scenario: Drill into directories
- **WHEN** user selects a workspace root
- **THEN** system lists subdirectories; git repos show branch and labels; non-repos show as blue directories
- **THEN** selecting a non-repo directory drills deeper; selecting a repo transitions to the session picker

#### Scenario: Back navigation during drill-down
- **WHEN** user presses Escape during directory drill-down (not at the top level)
- **THEN** system returns to the parent directory

#### Scenario: Jump to workspace
- **WHEN** user runs `ez --workspace personal`
- **THEN** system skips the root picker and starts drill-down in the matching workspace root

#### Scenario: Clone repo during drill-down
- **WHEN** user presses the clone keybind (default `alt-a`) during directory drill-down
- **THEN** system prompts for a git URL, clones into the current directory, and enters the session picker for the cloned repo
