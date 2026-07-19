# Cursor MCP Approvals (Delta)

## ADDED Requirements

### Requirement: Auto-approve MCP servers on session create
The cursor-mcp-approvals plugin SHALL compute and write `mcp-approvals.json` in the Cursor project directory (`~/.cursor/projects/<slug>/`) on `OnSessionCreate`. The plugin SHALL read MCP server configs from `<repo-root>/.cursor/mcp.json`, iterate over each server entry in `mcpServers`, and compute an approval ID for each server using the formula: `<serverName>-<sha256(JSON.stringify({path: worktreePath, server: transportConfig})).hex().substring(0, 16)>`. The resulting array of approval IDs SHALL be written as JSON.

#### Scenario: Session created with MCP configs in repo
- **WHEN** a session is created under a repo that has `.cursor/mcp.json` with `mcpServers` entries
- **THEN** the plugin reads each server's transport config, computes approval hashes for the worktree path, and writes `mcp-approvals.json` to `~/.cursor/projects/<worktree-slug>/`

#### Scenario: No `.cursor/mcp.json` in repo
- **WHEN** a session is created under a repo without `.cursor/mcp.json`
- **THEN** the plugin logs a debug message and does nothing (no error)

#### Scenario: Empty mcpServers
- **WHEN** `.cursor/mcp.json` exists but `mcpServers` is empty
- **THEN** the plugin writes an empty array `[]` to `mcp-approvals.json`

#### Scenario: Non-git repo skipped
- **WHEN** a session is created under a non-git repo (`is_git = false`)
- **THEN** the plugin skips approval computation

#### Scenario: Bare session skipped
- **WHEN** a bare session is created (no worktree path)
- **THEN** the plugin skips approval computation

#### Scenario: Hash computation correctness
- **WHEN** the plugin computes an approval hash
- **THEN** the JSON input to SHA256 is `{"path":"<worktree-abs-path>","server":<transport-config-json>}` with keys in the exact order: `path` first, `server` second
- **AND** the transport config JSON preserves the key order from `.cursor/mcp.json`
- **AND** the hash is the first 16 hex characters of the SHA256 digest

### Requirement: MCP approvals cleanup on session delete
The cursor-mcp-approvals plugin SHALL remove the `mcp-approvals.json` file from the Cursor project directory on `OnSessionDelete`. It SHALL NOT delete the project directory itself.

#### Scenario: Session deleted removes approvals file
- **WHEN** a session is deleted
- **THEN** the plugin removes `mcp-approvals.json` from the worktree's Cursor project directory

#### Scenario: No approvals file exists
- **WHEN** a session is deleted but no `mcp-approvals.json` exists at the worktree slug
- **THEN** the plugin succeeds silently (no error)

### Requirement: MCP approvals update on session rename
The cursor-mcp-approvals plugin SHALL recompute `mcp-approvals.json` on `OnSessionRename` when the worktree path changes. Since the hash depends on the workspace path, all approval IDs must be recomputed for the new path.

#### Scenario: Rename recomputes approvals
- **WHEN** a session is renamed and the worktree path changes
- **THEN** the plugin removes `mcp-approvals.json` at the old slug's project dir
- **AND** recomputes approval hashes using the new worktree path
- **AND** writes the new `mcp-approvals.json` at the new slug's project dir

#### Scenario: Missing rename paths
- **WHEN** a session is renamed but `rename_context` has no `old_path` or `new_path`
- **THEN** the plugin skips (no error)
