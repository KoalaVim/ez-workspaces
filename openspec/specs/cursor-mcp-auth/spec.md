# Cursor MCP Auth

## Purpose

Bundled plugin that ensures MCP authentication is shared across worktrees by symlinking `mcp-auth.json` from the main repo's Cursor project directory to each worktree's Cursor project directory.

## Requirements

### Requirement: MCP auth symlink on session create
The cursor-mcp-auth plugin SHALL create a symlink from the worktree's Cursor project directory `mcp-auth.json` to the main repo's `mcp-auth.json` on `OnSessionCreate`. The plugin SHALL compute workspace slugs using the formula: replace all non-alphanumeric characters with `-`, collapse consecutive dashes, and strip leading/trailing dashes. The source is `~/.cursor/projects/<main-repo-slug>/mcp-auth.json` and the destination is `~/.cursor/projects/<worktree-slug>/mcp-auth.json`.

#### Scenario: Session created with MCP auth in main repo
- **WHEN** a session is created under a git repo that has `~/.cursor/projects/<main-slug>/mcp-auth.json`
- **THEN** the plugin creates the destination project directory if needed and symlinks `mcp-auth.json` from the main repo's Cursor project dir

#### Scenario: No MCP auth in main repo
- **WHEN** a session is created but the main repo has no `mcp-auth.json` in its Cursor project dir
- **THEN** the plugin logs a debug message and does nothing (no error)

#### Scenario: Non-git repo skipped
- **WHEN** a session is created under a non-git repo (`is_git = false`)
- **THEN** the plugin skips symlink creation

#### Scenario: Bare session skipped
- **WHEN** a bare session is created
- **THEN** the plugin skips symlink creation (bare sessions have no worktree path)

### Requirement: MCP auth symlink update on rename
The cursor-mcp-auth plugin SHALL update the `mcp-auth.json` symlink on `OnSessionRename` when the worktree path changes. It SHALL remove the old symlink and create a new one at the new worktree's Cursor project slug.

#### Scenario: Rename updates symlink
- **WHEN** a session is renamed and the worktree path changes
- **THEN** the plugin removes the symlink at the old slug's project dir and creates one at the new slug's project dir

### Requirement: MCP auth cleanup on session delete
The cursor-mcp-auth plugin SHALL remove the `mcp-auth.json` symlink on `OnSessionDelete`. It SHALL NOT delete the destination project directory (other Cursor state may live there).

#### Scenario: Session deleted removes symlink
- **WHEN** a session is deleted
- **THEN** the plugin removes the `mcp-auth.json` symlink from the worktree's Cursor project dir
