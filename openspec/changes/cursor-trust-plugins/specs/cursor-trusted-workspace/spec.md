# Cursor Trusted Workspace (Delta)

## ADDED Requirements

### Requirement: Auto-trust worktree workspace on session create
The cursor-trusted-workspace plugin SHALL create a `.workspace-trusted` file in the Cursor project directory (`~/.cursor/projects/<slug>/`) on `OnSessionCreate`. The file SHALL contain `{"trustedAt": "<ISO-8601 timestamp>", "workspacePath": "<absolute worktree path>"}`. The plugin SHALL compute workspace slugs using the formula: replace all non-alphanumeric characters with `-`, collapse consecutive dashes, and strip leading/trailing dashes.

#### Scenario: Session created under git repo
- **WHEN** a session is created under a git repo
- **THEN** the plugin creates `~/.cursor/projects/<worktree-slug>/.workspace-trusted` with the worktree's absolute path and current timestamp

#### Scenario: Existing trust file preserved
- **WHEN** a session is created and the worktree's Cursor project directory already has `.workspace-trusted`
- **THEN** the plugin overwrites it with the current timestamp (idempotent)

#### Scenario: Non-git repo skipped
- **WHEN** a session is created under a non-git repo (`is_git = false`)
- **THEN** the plugin skips trust file creation

#### Scenario: Bare session skipped
- **WHEN** a bare session is created (no worktree path)
- **THEN** the plugin skips trust file creation

### Requirement: Trust file cleanup on session delete
The cursor-trusted-workspace plugin SHALL remove the `.workspace-trusted` file from the Cursor project directory on `OnSessionDelete`. It SHALL NOT delete the project directory itself.

#### Scenario: Session deleted removes trust file
- **WHEN** a session is deleted
- **THEN** the plugin removes `.workspace-trusted` from the worktree's Cursor project directory

#### Scenario: No trust file exists
- **WHEN** a session is deleted but no `.workspace-trusted` exists
- **THEN** the plugin succeeds silently (no error)

### Requirement: Trust file update on session rename
The cursor-trusted-workspace plugin SHALL update the `.workspace-trusted` file on `OnSessionRename` when the worktree path changes. It SHALL remove the old trust file and create a new one at the new worktree's Cursor project slug with the new path.

#### Scenario: Rename updates trust file
- **WHEN** a session is renamed and the worktree path changes
- **THEN** the plugin removes `.workspace-trusted` at the old slug's project dir and creates one at the new slug's project dir with the new worktree path

#### Scenario: Missing rename paths
- **WHEN** a session is renamed but `rename_context` has no `old_path` or `new_path`
- **THEN** the plugin skips (no error)
