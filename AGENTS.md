# AGENTS.md - ez-workspaces

## Project Overview

ez-workspaces (`ez`) is a Rust-based workspace and session manager. It manages git repos, tree-based sessions (backed by worktrees), and plugins via a CLI and interactive fzf browser.

## Architecture

- **Language**: Rust (single crate, modular structure)
- **Config**: TOML at `~/.config/ez/`
- **Data**: Notes and session data at `<data_dir>/ez/` (macOS: `~/Library/Application Support/ez/`, Linux: `~/.local/share/ez/`)
- **Dir overrides**: `EZ_CONFIG_DIR` / `EZ_DATA_DIR` (non-empty) replace the config/data dirs (resolved in `src/paths.rs`; plugins receive both)
- **Plugins**: External scripts (JSON-over-stdio protocol)
- **Interactive UI**: native ratatui TUI (default) or fzf (`selector.backend = "fzf"`); both share `browser/domain/`; wizards use the `InteractiveSelector` trait (`FzfSelector`, `NativeSelector`)
- **Env**: `EZ_NO_DAEMON=1` disables daemon auto-start

## Source Layout

```
src/
  main.rs           CLI dispatch
  cli.rs            Clap command definitions
  error.rs          Error types
  paths.rs          Path resolution (config_dir, data_dir, notes paths)
  config/           Global config (TOML)
  repo/             Repo CRUD + git clone
  session/          Session lifecycle + tree hierarchy
    from_dirty.rs   Session creation from dirty changes (stash workflow)
    base.rs         Base-branch prompt for sessions under the default session
    notes.rs        Per-session notes directory management
    env.rs          Session env CLI (set/list/unset)
  plugin/           Plugin execution engine
  browser/          Interactive browser (two front ends over a shared domain layer)
    domain/         Front-end independent logic: rows, preview, action handlers, effects
    tui/            Native ratatui front end (selector.backend = "native")
      app/          App state split: actions, draw, keymap, loader, snapshot_io
    views/          fzf views (pull-style)
plugins/            Bundled plugin scripts
docs/               Documentation
```

## Key Modules

### config/ - Configuration
- `model.rs`: `EzConfig`, `SelectorConfig`, `PluginsConfig`, `NativeConfig` (`preview_ratio`, `snapshot`) structs; `SelectorBackend` enum (`Fzf`, `Native`); `NameBuilderMode` enum (`FullName`, `BuildFromParts`, `GitHubPr`, `JiraUrl`); `SortMode` enum (`Lru`, `Alpha`); `name_builder_modes`, `default_sort`, `note_open_command` fields on `EzConfig`; `view_next`, `view_prev` (tab/btab view cycling), `sort_toggle`, `new_bare_session`, `session_from_dirty`, `clone_repo`, `remove_repo`, `note_open`, `note_cd` keybind fields
- `mod.rs`: load/save/edit config

### repo/ - Repository Management
- `model.rs`: `RepoIndex`, `RepoEntry` (with `is_git` field), `RepoMeta` (with `last_accessed` field)
- `store.rs`: filesystem persistence
- `mod.rs`: clone, add, remove, list, resolve operations

### session/ - Session Management
- `model.rs`: `Session` (with `bare` and `last_accessed` fields), `SessionTree` structs
- `tree.rs`: tree operations (roots, children, ancestors, descendants, render); `TreeNode` struct and `format_session_tree_line` for box-drawing glyph rendering
- `name_builder.rs`: interactive name builder with mode selection (`FullName`, `BuildFromParts`, `GitHubPr`, `JiraUrl`)
- `store.rs`: filesystem persistence
- `from_dirty.rs`: session creation from dirty changes (stash workflow)
- `base.rs`: `offer_default_branch_base` — when a new session's parent is the default session (or absent) and the default checkout is on a branch other than main/master, asks (`confirm_prompt`) whether to start from that branch and sets `ez_start_point`
- `notes.rs`: per-session notes directory (`<data_dir>/ez/repos/<repo-id>/notes/<session-id>/README.md`); `ensure_notes_dir`, `open_note`, `delete_notes_dir`, `notes_readme_exists`
- `current.rs`: current-session detection from tmux `@ez_session_name` + `@ez_repo_id` (preferred) with fallback to `@ez_session_path` and worktree paths
- `env.rs`: `ez session env {set|list|unset}` — set, list, and unset environment variables on a session
- `mod.rs`: new, register existing worktree, delete, enter, exit, rename, ensure_default_session, note dispatch, env dispatch, `UnmanagedWorktree` struct, `list_unmanaged_worktrees()`, `register_worktree_inline()`, `delete_unmanaged_worktree()`, `parse_worktree_list_porcelain()`

### plugin/ - Plugin System
- `model.rs`: `PluginManifest`, `HookType` enum (14 hook types including `OnBind`, `OnView`, `OnViewSelect`, `OnNameResolve`), `PluginBind`, `PluginView`, `ConfigField`
- `protocol.rs`: `HookRequest` (with `BindContext`, `ViewContext`, `NameResolveContext`), `HookResponse` (with `post_shell_commands`, `cd_target`, `view_items`, `resolved_name`), `PluginConfig` (with `user_config`), `ViewItem`
- `runner.rs`: process execution with timeout
- `bundled.rs`: embedded plugins, auto-extracted and auto-updated on version change
- `mod.rs`: hook dispatch, enable/disable, `collect_plugin_views()`, `run_view_hook()`, `run_view_select_hook()`

### browser/ - Interactive Browser
- `selector.rs`: `InteractiveSelector` trait + `FzfSelector` impl
- `mod.rs`: `browse` entry, `browse_repo`, `SortMode`, re-exports of the helpers below
- `session_loop.rs`: fzf session-list loop; drives the selector, builds rows via `domain`, dispatches keys to `domain::actions` and applies the returned effects
- `drill.rs`: fzf workspace directory drill-down (rows from `domain::rows::directory_rows`)
- `effects.rs`: fzf-side effect executor (`apply_effects`) writing `cd_file` / `post_cmd_file` / the `#EZ_RELAUNCH` marker; `write_cd_target`, `write_post_commands`, `write_relaunch_marker`, `accept_session` wrapper
- `git.rs`: `BranchCache`, `git_cmd`, `get_branch`, `is_dirty`, gitdir/rebase helpers
- `format.rs`: display formatters (`format_last_accessed`, `format_pr_indicator`, `format_branch_indicator`, `format_repo_display`) and `parse_label_input`
- `domain/mod.rs`: `Row { display (ANSI), value, kind }`, `RowKind`, and the `Effect` enum (`Cd`, `PostCmds { cmds, relaunch }`, `Exec`, `OpenNote`, `Refresh`, `Notify`, `Exit`); `outln!` macro for writing into a `String`
- `domain/rows.rs`: row builders for Tree, Workspace roots, directory drill-down, Repo, Owner, Label and plugin views (`Vec<Row>`, no terminal involved)
- `domain/session_rows.rs`: row builder for the per-repo session list (managed sessions + unmanaged worktrees)
- `domain/preview.rs`, `preview_session.rs`, `keybind_help.rs`: preview renderers writing into a `String` (`fmt::Write`); `ez preview` prints the returned string
- `domain/actions.rs`: session-list handlers returning `Vec<Effect>` (`ActionCtx`, `select_session`, `select_unmanaged`, `delete_unmanaged`, `handle_session_action`); wizards still pull from `&dyn InteractiveSelector`
- `domain/accept.rs`: `accept_session` / `bind_response_effects` as effects (on_enter / on_create navigation)
- `domain/fresh.rs`: re-validates a row's target against fresh data before acting (stale rows are refused with a notification); `domain/repo_actions.rs`: repo-level handlers (labels, remove, clone); `domain/plugin_view.rs`: plugin view rows and select handling; `domain/preview_slow.rs`: slow preview parts (git log, notes) split out so a front end can run them off-thread; `domain/tests.rs`: domain tests
- `tui/mod.rs`: native entry (`run`); `terminal.rs` RAII terminal guard + panic hook + suspend/resume; `event.rs` sync poll loop with `mpsc` results (dirty-flag redraw); `keys.rs` fzf key-string parser; `list.rs` nucleo fuzzy list on ANSI-stripped text; `nav.rs` + `level.rs` navigation stack (Esc pops, Ctrl-C = `Interrupted`); `preview_pane.rs` in-process ANSI preview (`ansi-to-tui`, cached); `effects.rs` native effect executor (cd/post-cmd files, suspend for `Exec`); `modal.rs` + `modal_state.rs` `NativeSelector` (modal prompts on the shared terminal); `capture.rs` + `stdio.rs` keep handler/stray stdout+stderr off the alternate screen; `workers.rs` fixed thread pool with generation ids; `snapshot.rs` versioned atomic TOML of last rows (`<data_dir>/ez/browser-snapshot.toml`, `native.snapshot`); `theme.rs` palette (named ANSI colors, follows the terminal theme), rounded panels, key chips, spinner, and the preview section-header restyle; `app/draw.rs` layout (search bar on top, view tabs, list + preview panels, key-hint footer); `app/handler.rs` event-loop callbacks
- `tui/app.rs` + `app/`: `App` state, key map, drawing, row loading on workers, snapshot swap-in, action dispatch
- `views/mod.rs`: top-level view dispatcher (`ViewMode`: Tree/Workspace/Repo/Owner/Label/Plugin) with view-switch keybind handling including plugin views
- `views/*.rs`: fzf views (tree, workspace, repo, owner, label, plugin_view) built on `domain::rows`

## Key Traits

- `InteractiveSelector`: abstracting UI backends (fzf, skim, etc.)
- Repo/session stores use plain functions (not traits yet) — extract if mocking is needed

## Build & Test

```bash
make build         # debug build
make test          # run tests
make release       # optimized build
make install       # cargo install --locked --path .
make install-debug # install debug build (unoptimized, faster compile)
make lint          # clippy
make fmt           # format code
make check         # fmt check + clippy + tests
make dev           # debug build against .dev/ (EZ_CONFIG_DIR/EZ_DATA_DIR); ARGS="...", DEV_DIR=...
make dev-sync      # mirror real repos/sessions/notes into .dev/
```

## Plugin Development

Plugins are shell scripts or executables in `~/.config/ez/plugins/<name>/`. See `docs/plugin-guide.md` for the JSON protocol. Bundled plugins (git-worktree, tmux, zellij, herdr, rex, cursor-mcp-auth, cursor-trusted-workspace, cursor-mcp-approvals, kv) in `plugins/` directory.

## Conventions

- Files under 500 lines
- `thiserror` for error types
- No async — synchronous CLI
- Shell out to `git` (no libgit2)
- TOML for all config/metadata files
- When adding a new feature, always update `README.md`, `docs/user-guide.md`, and `AGENTS.md`
- When changing architecture, modules, data flow, or adding new modules, update `docs/design.md` diagrams
- Escape in interactive menus always goes back to the previous level (e.g., parent directory, previous menu). Only cancel/abort when at the top-most level.
- Ctrl-C quits ez from any level. The fzf selector binds `ctrl-c:become(exit 131)` and returns `Err(EzError::Interrupted)`; let it propagate with `?` and never swallow it in `Err(e) => eprintln!` arms.
- All CLI output must be colored using the `colored` crate. Use `--no-color` global flag to disable. Convention: green for success, yellow for warnings, cyan for info/labels, bold for emphasis, dimmed for secondary info.
- Use `log` crate (`log::debug!`, `log::trace!`, etc.) for debug logging. Activated via `ez --debug` (writes to `/tmp/ez-debug-<pid>.log`, prints path on exit) or `RUST_LOG=debug`. Add debug logs to any non-trivial logic (selector interactions, plugin execution, fzf I/O).
- The build must produce zero warnings. Fix all warnings before finishing a task. Remove dead code, don't suppress with `#[allow]` unless the code is a public API intended for future use.
