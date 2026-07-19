## 1. cursor-trusted-workspace plugin

- [x] 1.1 Create `plugins/cursor-trusted-workspace/manifest.toml` with hooks `on_session_create`, `on_session_delete`, `on_session_rename`
- [x] 1.2 Create `plugins/cursor-trusted-workspace/cursor-trusted-workspace-plugin` bash script implementing: `on_session_create` (write `.workspace-trusted` JSON with `trustedAt` and `workspacePath`), `on_session_delete` (remove `.workspace-trusted`), `on_session_rename` (remove old, create new)
- [x] 1.3 Register `cursor-trusted-workspace` in `src/plugin/bundled.rs` `BUNDLED_PLUGINS` array

## 2. cursor-mcp-approvals plugin

- [x] 2.1 Create `plugins/cursor-mcp-approvals/manifest.toml` with hooks `on_session_create`, `on_session_delete`, `on_session_rename`
- [x] 2.2 Create `plugins/cursor-mcp-approvals/cursor-mcp-approvals-plugin` bash script implementing: `on_session_create` (read `.cursor/mcp.json` from repo root, iterate `mcpServers`, compute `sha256(JSON.stringify({path, server}))` approval hashes using `shasum -a 256`, write `mcp-approvals.json`), `on_session_delete` (remove `mcp-approvals.json`), `on_session_rename` (remove old, recompute and write new)
- [x] 2.3 Register `cursor-mcp-approvals` in `src/plugin/bundled.rs` `BUNDLED_PLUGINS` array

## 3. Build and verify

- [x] 3.1 Run `make build` — ensure zero warnings
- [x] 3.2 Run `make test` — ensure all tests pass

## 4. Documentation

- [x] 4.1 Update `README.md` with cursor-trusted-workspace and cursor-mcp-approvals plugin descriptions
- [x] 4.2 Update `docs/user-guide.md` with plugin configuration details
- [x] 4.3 Update `AGENTS.md` if any new modules or architectural changes (no changes needed — AGENTS.md references `plugins/` directory generically)
