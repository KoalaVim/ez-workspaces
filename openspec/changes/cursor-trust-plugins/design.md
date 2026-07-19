## Context

Cursor IDE stores per-workspace state under `~/.cursor/projects/<slug>/` where slug is derived from the workspace path (replace non-alphanumeric chars with `-`, collapse, strip). When a new worktree session is opened in Cursor, two prompts block the user:

1. **Workspace trust** — Cursor asks whether to trust the workspace. Approval writes `.workspace-trusted` with format: `{"trustedAt": "<ISO>", "workspacePath": "<abs-path>"}`.

2. **MCP server approvals** — Each MCP server must be individually approved. Approvals are stored in `mcp-approvals.json` as an array of `<serverName>-<hash>` strings. The hash is `sha256(JSON.stringify({path: workspacePath, server: transportConfig})).hex().substring(0, 16)`.

The existing `cursor-mcp-auth` plugin already handles symlinking `mcp-auth.json` (OAuth tokens). These two new plugins complete the Cursor integration story.

## Goals / Non-Goals

**Goals:**
- Auto-create `.workspace-trusted` for worktree sessions on create/rename
- Auto-compute `mcp-approvals.json` for worktree sessions using the correct hash formula
- Clean up both files on session delete
- Follow existing bundled plugin conventions (bash, JSON-over-stdio, debug logging)

**Non-Goals:**
- Handling global/user-level MCP configs (only repo-level `.cursor/mcp.json`)
- Syncing `mcp-disabled.json` (users may want different disable lists per worktree)
- Supporting non-macOS platforms for hash computation (macOS focus)

## Decisions

### 1. Hash computation: `shasum -a 256` over `node -e`
`shasum` is a macOS built-in (ships with perl). Avoids requiring Node.js in PATH. The hash formula requires computing `sha256(JSON.stringify({path, server}))` — the plugin constructs the JSON string in bash and pipes to `shasum`. This matches Cursor's algorithm exactly (verified against real approvals).

### 2. Separate plugins (not extending cursor-mcp-auth)
Each plugin has a single responsibility and can be independently enabled/disabled. Users who don't need MCP approvals (no `.cursor/mcp.json`) won't run unnecessary hooks.

### 3. Read `.cursor/mcp.json` from repo root only
The MCP server configs live at `<repo-root>/.cursor/mcp.json`. The plugin iterates `mcpServers` entries, using each server's name and transport config object for hash computation.

### 4. JSON key ordering for hash
`JSON.stringify` in JavaScript produces keys in insertion order. The hash input is `{"path":"...","server":{...}}` — the `path` key must come before `server`. Within the server object, key order must match `.cursor/mcp.json` as parsed by JavaScript (insertion order). Bash must reproduce this exactly using `jq` to read and reconstruct the JSON.

## Risks / Trade-offs

- **Cursor format changes**: The `.workspace-trusted` and `mcp-approvals.json` formats are undocumented internals. Cursor updates could change them. Mitigation: plugins are trivial to update.
- **JSON key ordering**: The SHA256 hash is sensitive to JSON serialization order. If `jq` reorders keys differently than JavaScript's `JSON.stringify`, hashes won't match. Mitigation: use `jq -S` is wrong (sorts keys); instead reconstruct JSON manually or use `jq -c` which preserves input order.
- **Missing `.cursor/mcp.json`**: Repos without MCP configs will silently skip (no error). This is the expected behavior.
- **Race condition with Cursor**: If Cursor reads the files while the plugin writes them, partial data could be read. Mitigation: write to a temp file and `mv` atomically.
