## Why

ez has bundled integrations for tmux, zellij and herdr, but none for Rex. A Rex plugin (one Rex session per repo, one tab per ez session) exists as an external plugin that has only been exercised on the happy path. Bundling it ships it with ez, and porting is a good moment to harden it against the edge cases it hasn't met yet.

## What Changes

- Add `plugins/rex/` (manifest + executable) as a bundled plugin, registered in `src/plugin/bundled.rs`, ported from the existing external plugin.
- Keep the existing behavior: `alt-x` bind (`rex_open`) opens/focuses the session's Rex tab via a post-shell `focus` sub-command with `accept` + `keep_cwd`; `on_session_delete` closes the tab (never the one ez runs in) and kills the Rex session when it has no tabs left.
- Rewrite the executable in bash, like the other bundled plugins, so it doesn't need zsh.
- Hardening found through manual testing, including:
  - never auto-start a Rex server from a hook that only cleans up (delete/rename)
  - always print valid JSON, even when `rex`/`jq` calls fail mid-hook
  - quote paths correctly in post-shell commands (ez's plugin dir on macOS contains a space)
  - add `on_session_rename` so a renamed session keeps its tab instead of getting a duplicate on next focus
- Docs: README, `docs/user-guide.md`, `docs/plugin-guide.md` (bundled list), `AGENTS.md`.

## Capabilities

### New Capabilities
- `rex-plugin`: Bundled plugin mapping ez repos/sessions onto Rex sessions/tabs — open/focus on bind, relabel on rename, close on delete, graceful no-op when Rex is unavailable.

### Modified Capabilities
<!-- none -->

## Impact

- New: `plugins/rex/manifest.toml`, `plugins/rex/rex-plugin`.
- Modified: `src/plugin/bundled.rs` (one entry), docs listed above.
- Runtime deps: `rex` CLI and `jq` (both optional — plugin no-ops without `rex`).
