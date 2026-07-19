## Why

When ez creates a new worktree session and the user opens it in Cursor IDE, they are prompted to (1) trust the workspace and (2) re-approve each MCP server. This is friction that breaks flow, especially for repos with many MCP integrations (Jira, Notion, Figma, Slack, etc.). Since worktree sessions are created under the same repo, they should inherit trust and MCP approvals automatically.

## What Changes

Two new bundled plugins that run on session lifecycle hooks:

1. **cursor-trusted-workspace** — Creates a `.workspace-trusted` file in Cursor's per-project directory (`~/.cursor/projects/<slug>/`) so Cursor skips the trust prompt when opening a worktree.

2. **cursor-mcp-approvals** — Computes and writes `mcp-approvals.json` for the new worktree's Cursor project directory. Since Cursor's approval IDs include a SHA256 hash of `{path: workspacePath, server: transportConfig}`, the plugin reads MCP server configs from the repo's `.cursor/mcp.json`, recomputes hashes for the new worktree path, and writes them to the worktree's approval file.

Both plugins follow the same pattern as the existing `cursor-mcp-auth` plugin: bash scripts using the JSON-over-stdio protocol, triggered on `on_session_create`, `on_session_delete`, and `on_session_rename`.

## Capabilities

### New Capabilities
- `cursor-trusted-workspace`: Bundled plugin that auto-creates `.workspace-trusted` in Cursor project directories for worktree sessions
- `cursor-mcp-approvals`: Bundled plugin that recomputes and writes MCP server approval IDs for worktree sessions

### Modified Capabilities

## Impact

- New bundled plugins: `plugins/cursor-trusted-workspace/` and `plugins/cursor-mcp-approvals/`
- `src/plugin/bundled.rs`: Register both new plugins in `BUNDLED_PLUGINS`
- Requires `node` in PATH for the MCP approvals plugin (SHA256 hash computation)
- Reads `.cursor/mcp.json` from the repo root for server transport configs
