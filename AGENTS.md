# AGENTS.md - ez-workspaces

## Project Overview

ez-workspaces (`ez`) is a Rust-based workspace and session manager. It manages git repos, tree-based sessions (backed by worktrees), and plugins via a CLI and interactive fzf browser.

## Architecture

- **Language**: Rust (single crate, modular structure)
- **Config**: TOML at `~/.config/ez/`
- **Plugins**: External scripts (JSON-over-stdio protocol)
- **Interactive UI**: fzf via `InteractiveSelector` trait

## Source Layout

```
src/
  main.rs           CLI dispatch
  cli.rs            Clap command definitions
  error.rs          Error types
  paths.rs          Path resolution
  config/           Global config (TOML)
  repo/             Repo CRUD + git clone
  session/          Session lifecycle + tree hierarchy
    from_dirty.rs   Session creation from dirty changes (stash workflow)
  plugin/           Plugin execution engine
  browser/          Interactive fzf browser
plugins/            Bundled plugin scripts
docs/               Documentation
```

## Key Modules

### config/ - Configuration
- `model.rs`: `EzConfig`, `SelectorConfig`, `PluginsConfig` structs; `NameBuilderMode` enum (`FullName`, `BuildFromParts`, `GitHubPr`, `JiraUrl`); `SortMode` enum (`Lru`, `Alpha`); `name_builder_modes`, `default_sort` fields on `EzConfig`; `sort_toggle`, `new_bare_session`, `session_from_dirty`, `clone_repo` keybind fields
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
- `current.rs`: current-session detection from tmux `@ez_session_name` + `@ez_repo_id` (preferred) with fallback to `@ez_session_path` and worktree paths
- `mod.rs`: new, register existing worktree, delete, enter, exit, rename, ensure_default_session

### plugin/ - Plugin System
- `model.rs`: `PluginManifest`, `HookType` enum (14 hook types including `OnBind`, `OnView`, `OnViewSelect`, `OnNameResolve`), `PluginBind`, `PluginView`, `ConfigField`
- `protocol.rs`: `HookRequest` (with `BindContext`, `ViewContext`, `NameResolveContext`), `HookResponse` (with `post_shell_commands`, `cd_target`, `view_items`, `resolved_name`), `PluginConfig` (with `user_config`), `ViewItem`
- `runner.rs`: process execution with timeout
- `bundled.rs`: embedded plugins, auto-extracted and auto-updated on version change
- `mod.rs`: hook dispatch, enable/disable, `collect_plugin_views()`, `run_view_hook()`, `run_view_select_hook()`

### browser/ - Interactive Browser
- `selector.rs`: `InteractiveSelector` trait + `FzfSelector` impl
- `mod.rs`: drill-down browse flow, session action loop, label input parser, shared git helpers
- `preview.rs`: fzf preview pane renderer (repo, directory, keybind help)
- `views/mod.rs`: top-level view dispatcher (`ViewMode`: Tree/Workspace/Repo/Owner/Label/Plugin) with view-switch keybind handling including plugin views
- `views/plugin_view.rs`: renderer for plugin-provided views (OnView → fzf → OnViewSelect)

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
```

## Plugin Development

Plugins are shell scripts or executables in `~/.config/ez/plugins/<name>/`. See `docs/plugin-guide.md` for the JSON protocol. Bundled plugins (git-worktree, tmux, cursor-mcp-auth, cursor-trusted-workspace, cursor-mcp-approvals, kv) in `plugins/` directory.

## Conventions

- Files under 500 lines
- `thiserror` for error types
- No async — synchronous CLI
- Shell out to `git` (no libgit2)
- TOML for all config/metadata files
- When adding a new feature, always update `README.md`, `docs/user-guide.md`, and `AGENTS.md`
- When changing architecture, modules, data flow, or adding new modules, update `docs/design.md` diagrams
- Escape/Ctrl+C in interactive menus always goes back to the previous level (e.g., parent directory, previous menu). Only cancel/abort when at the top-most level.
- All CLI output must be colored using the `colored` crate. Use `--no-color` global flag to disable. Convention: green for success, yellow for warnings, cyan for info/labels, bold for emphasis, dimmed for secondary info.
- Use `log` crate (`log::debug!`, `log::trace!`, etc.) for debug logging. Activated via `ez --debug` (writes to `/tmp/ez-debug-<pid>.log`, prints path on exit) or `RUST_LOG=debug`. Add debug logs to any non-trivial logic (selector interactions, plugin execution, fzf I/O).
- The build must produce zero warnings. Fix all warnings before finishing a task. Remove dead code, don't suppress with `#[allow]` unless the code is a public API intended for future use.
