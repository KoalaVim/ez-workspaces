# Architecture

## Overview

ez-workspaces is a Rust CLI tool organized as a single crate with modular components.

```
src/
  main.rs           Entry point + CLI dispatch
  lib.rs            Re-exports
  cli.rs            Clap command definitions
  error.rs          EzError enum (thiserror)
  paths.rs          Config directory resolution
  config/           Global configuration (TOML)
  repo/             Repo registration and management
  session/          Session lifecycle and tree hierarchy
  plugin/           Plugin execution (JSON protocol)
  browser/          Interactive fzf browser
```

## Key Design Decisions

### Central Storage

All metadata lives in `~/.config/ez/`:

```
~/.config/ez/
  config.toml           Global config
  repos/
    index.toml          All registered repos
    <repo-id>/
      repo.toml         Repo metadata + plugin state
      sessions.toml     Session tree
  plugins/
    <plugin-name>/
      manifest.toml
      <executable>
```

### Virtual Sessions

Sessions are metadata-only by default. Plugins give them physical meaning:
- git-worktree plugin sets `session.path` to a worktree directory
- tmux plugin creates a tmux session

This keeps the core simple and makes ez usable without git.

### Session Tree

Sessions use a flat list with `parent_id` pointers (adjacency list). This is simple to serialize in TOML and supports efficient tree operations via iteration.

### Plugin Protocol

Plugins are external executables using JSON-over-stdio:
- Request on stdin (single JSON line) — includes `bind_context` / `view_context` for interactive hooks
- Response on stdout (single JSON line) — can return `view_items` for plugin views, `post_shell_commands` for post-exit execution
- stderr for diagnostics
- Patch semantics for mutations
- 30s timeout (configurable)
- User-facing config via `[plugin_settings.<name>]` in config.toml, delivered in `config.user_config`

### Interactive Selector Trait

The `InteractiveSelector` trait abstracts the UI backend. The default `FzfSelector` shells out to fzf. This can be replaced with skim, dialoguer, or a TUI framework without changing browser logic.

### Browser View Dispatcher

The browser has a `ViewMode` enum (`Tree`, `Workspace`, `Repo`, `Owner`, `Label`, `Plugin`) implemented in `src/browser/views/mod.rs`. `browse()` enters a dispatch loop that renders the current view and listens for view-switch keybinds (`ctrl-t/w/e/o/g` by default, plus plugin view keys like `ctrl-a`). Each view uses `select_with_actions` with the view-switch keys registered in `--expect`; pressing one exits the current fzf instance and the loop continues in the next mode. Plugin views are rendered by `plugin_view.rs` which calls the plugin's `OnView` hook to get items, then `OnViewSelect` on selection. Plugin view keys are collected from enabled plugin manifests and merged into the expect list alongside core keys. Nested selectors (drill-down, session picker) also include plugin view keys.

### Post-Exit Shell Commands

The shell wrapper (`ez init-shell`) sources a post-cmd-file after `cd`. This allows plugins to run commands in the user's shell after ez exits — critical for `tmux switch-client` which needs the user's terminal. Plugins return `post_shell_commands` in their response.

### Shell Integration

The `cd`-on-enter pattern uses a temp file (same approach as zoxide, nnn). The `ez init-shell` command generates a shell wrapper function that also sources a post-cmd-file for plugin post-exit commands.

## Data Flow

```
User runs `ez session new feature-x`
  -> cli.rs parses args
  -> session::new_session()
     -> repo::resolve_repo() finds the current repo
     -> session::store::load_sessions() loads session tree
     -> Creates Session struct, adds to tree
     -> plugin::run_hooks(OnSessionCreate, ...)
        -> For each enabled plugin:
           -> plugin::runner::execute() spawns plugin process
           -> Sends HookRequest JSON on stdin
           -> Reads HookResponse JSON from stdout
           -> Applies session_mutations (e.g., sets path)
     -> session::store::save_sessions() persists tree
```

## Error Handling

- Custom `EzError` enum with `thiserror`
- All modules return `Result<T, EzError>`
- Plugin errors: fatal for create/delete hooks, warnings for enter/exit
- `main.rs` catches and prints errors, sets exit code 1

## Testing

- Unit tests: session tree operations, JSON protocol serialization, path utilities
- Integration tests: CLI commands via `assert_cmd`, plugin protocol
- Mock support: `InteractiveSelector` trait enables `MockSelector` for browser tests
