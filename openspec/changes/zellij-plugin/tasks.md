## 1. Core enablers (Rust)

- [x] 1.1 Add `EZ_BIN` (from `std::env::current_exe()`) to the plugin command environment in `src/plugin/runner.rs`, next to the existing `EZ_CONFIG_DIR`
- [x] 1.2 Add `--all` to `SessionCommand::List` in `src/cli.rs` (mutually exclusive with `--repo`, documented in help text)
- [x] 1.3 Implement `--all` in the session list handler: JSON mode emits `[{id, name, path, sessions: [...]}]` per registered repo using the existing session JSON shape; non-JSON mode prints each repo's tree under a header
- [x] 1.4 Return a clear error when `--all` and `--repo` are both passed
- [x] 1.5 Add `encode_mux_name(repo_basename, session_name) -> String` (non-`[A-Za-z0-9_-]` → `_`, joined with `__`) to `src/session/current.rs` with unit tests covering dots, slashes, colons, spaces, and non-ASCII

## 2. Multiplexer-agnostic current session

- [x] 2.1 Add a `Zellij(PathBuf)` variant to `CurrentSessionSource` with label `"zellij session name"`
- [x] 2.2 Add a zellij resolution branch to `resolve_current_session`: read `$ZELLIJ_SESSION_NAME`, match it against `encode_mux_name(basename(repo.path), session.name)` across registered repos/sessions, placed after the tmux branches and before the cwd fallback
- [x] 2.3 Update the `SessionNotFound` error text to mention tmux, zellij, and the working directory
- [x] 2.4 Rewrite `cd_to_session` in `src/main.rs` to call `resolve_current_session(None)` and write the session path (falling back to the repo path when `session.path` is `None`); drop the direct `tmux show-options` call and the `$TMUX` requirement
- [x] 2.5 Update the `CdToSession` doc comment in `src/cli.rs` to describe multiplexer-agnostic resolution
- [x] 2.6 Unit-test the name-match resolution helper (match, no-match, and collision-picks-first cases)

## 3. Delete reaper isolation

- [x] 3.1 Clear `ZELLIJ`, `ZELLIJ_SESSION_NAME`, `ZELLIJ_PANE_ID`, and `TMUX_PANE` alongside `TMUX` when spawning the detached reaper in `src/session/mod.rs`
- [x] 3.2 Resolve `reap_delay_ms` from `[plugin_settings.tmux]` then `[plugin_settings.zellij]`, defaulting to 200 ms

## 4. Zellij plugin manifest

- [x] 4.1 Create `plugins/zellij/manifest.toml`: name `zellij`, version `0.1.0`, hooks `on_session_create`, `on_session_delete`, `on_session_enter`, `on_session_exit`, `on_session_rename`, `on_bind`, `on_view`, `on_view_select`, executable `zellij-plugin`
- [x] 4.2 Declare the bind: key `alt-z`, name `zellij_attach`, label `zellij`, contexts `["session"]`
- [x] 4.3 Declare the view: name `zellij`, key `ctrl-z`, label `zellij`, contexts `["repo", "owner", "workspace", "tree", "label"]`
- [x] 4.4 Declare config schema: `auto_attach` (bool, false), `force_delete` (bool, true), `reap_delay_ms` (int, 200)

## 5. Zellij plugin executable

- [x] 5.1 Create `plugins/zellij/zellij-plugin` (bash, `set -euo pipefail`, `chmod +x`) with the request parsing, `dbg`/`dbg_zellij` debug helpers gated on `EZ_PLUGIN_DEBUG_LOG`, and an `encode()` mirroring `encode_mux_name`
- [x] 5.2 Add shared helpers: `zellij_available`, `session_running <name>` (via `zellij list-sessions -sn`), `ensure_session <name> <path>` (background-create with session env exported), and `attach_cmd <name> <path>` emitting the `$ZELLIJ`-branching attach/switch command
- [x] 5.3 `on_session_create`: ensure the background session exists at the session path with env applied; return `session_mutations.env.EZ_ZELLIJ_SESSION` and `plugin_state.zellij_session`
- [x] 5.4 `on_session_delete`: `delete-session --force` when `force_delete` is true, otherwise `kill-session`; always return success
- [x] 5.5 `on_session_enter`: return the attach/switch command when `auto_attach` is true, otherwise plain success; `on_session_exit`: plain success
- [x] 5.6 `on_session_rename`: rename via `zellij --session <old> action rename-session <new>` when the old session is running; succeed silently otherwise
- [x] 5.7 `on_bind`: ensure the session exists, then return the attach/switch command as `post_shell_commands`; error with "zellij not installed" when the binary is missing
- [x] 5.8 `on_view`: enumerate sessions via `"$EZ_BIN" session list --all --json`, mark each `●`/`○` against the running list, emit `view_items` with value `<encoded-name>|<session-path>` and prompt `zellij`; return an empty list with a "not installed" prompt when zellij is missing
- [x] 5.9 `on_view_select`: parse the value, ensure the session exists at its path, return the attach/switch command
- [x] 5.10 Handle the unknown-hook default case with `{"success": true}`

## 5b. Socket path length

- [x] 5b.1 Propagate `zellij attach --create-background` failure out of `ensure_session` via `ENSURE_ERROR` instead of always returning 0
- [x] 5b.2 `on_bind` / `on_view_select`: return `success: false` with the zellij error and no attach command
- [x] 5b.3 `on_session_create` / `on_session_enter`: warn on stderr, still succeed, and omit the attach command
- [x] 5b.4 Append socket-directory guidance to zellij's own "IPC socket path is too long" message
- [x] 5b.5 ~~Auto-shorten the socket path: symlink `/tmp/zj-<uid>` to zellij's own socket directory, export `ZELLIJ_SOCKET_DIR`, and prefix the emitted attach/switch commands with it~~ — shipped, then reverted: a name reachable only through the symlink is invisible to every zellij process ez did not spawn (see design decision 8)
- [x] 5b.6 ~~Only trust a symlink owned by the current user pointing at the expected target; fall back to zellij's default otherwise~~ — dropped with 5b.5
- [x] 5b.7 Add the `socket_dir` setting as an override; respect an existing `ZELLIJ_SOCKET_DIR`
- [x] 5b.8 Document the limit and the automatic fix in `docs/user-guide.md` and `examples/config.toml`
- [x] 5b.9 Compute the name budget in the plugin (`103 - socket dir - contract dir - 2`), reading the contract-version directory from disk and assuming the widest two-digit form when it is absent
- [x] 5b.10 Shorten over-budget names to `<truncated session>_<4 hex md5 of the full name>` in `fit_name`; use it for create, delete, rename, bind, view and view-select
- [x] 5b.11 Accept both encodings in current-session detection via `mux_name_matches`, without re-deriving the budget; golden-value tests lock the bash and Rust digests together
- [x] 5b.12 Fail the create (no session, clear error) when no digest tool exists or the socket directory leaves no room for a name
- [x] 5b.13 Verify a shortened session is listed by a plain `zellij list-sessions`, controllable from a fresh process with no `ZELLIJ_SOCKET_DIR`, renameable across both encodings, and deletable

## 6. Bundling

- [x] 6.1 Register the zellij plugin in `BUNDLED_PLUGINS` in `src/plugin/bundled.rs`
- [x] 6.2 Verify extraction writes `manifest.toml` and a `0o755` executable to `~/.config/ez/plugins/zellij/` on first use

## 7. Verification

- [x] 7.1 `cargo build`, `cargo test`, `cargo clippy` clean
- [x] 7.2 Enable the plugin (`ez plugin enable zellij`), create a session, and confirm a detached zellij session appears in `zellij list-sessions` rooted at the worktree path
- [ ] 7.3 Press `Alt-z` from a plain shell (attaches) and from inside zellij (switches); confirm detaching returns to the ez browser
- [x] 7.4 Confirm `ez cd-to-session` resolves the worktree path from inside a plugin-created zellij session, and that `ez session delete` (no name) auto-detects the current session there
- [x] 7.5 Rename a session and confirm the zellij session name follows; delete it and confirm the name disappears from `zellij list-sessions` with `force_delete = true`
- [ ] 7.6 Open the `Ctrl-z` view; confirm running/stopped markers are correct and selecting a stopped session creates and attaches it
- [x] 7.7 Confirm `on_enter = "zellij"` and `auto_attach = true` paths work, and that deleting the ez session you are attached to completes via the detached reaper
- [x] 7.8 Confirm graceful degradation with zellij removed from `PATH` (lifecycle hooks succeed, bind errors, view shows "not installed")
- [x] 7.9 Enable tmux and zellij together and confirm no keybind collisions

## 8. Documentation

- [x] 8.1 `docs/user-guide.md`: add `Ctrl-z` and `Alt-z` to the keybind/view lists, a zellij plugin section (config, `on_enter = "zellij"`, zellij ≥ 0.40 requirement, env-at-creation limitation), and generalize the cd-to-session and return-after-detach sections
- [x] 8.2 `docs/plugin-guide.md`: document `EZ_BIN` in the plugin environment
- [x] 8.3 `docs/architecture.md` and `README.md`: list zellij among the bundled plugins
- [x] 8.4 `examples/config.toml`: add a commented `[plugin_settings.zellij]` block
