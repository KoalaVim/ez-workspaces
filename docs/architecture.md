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
- Request on stdin (single JSON line)
- Response on stdout (single JSON line)
- stderr for diagnostics
- Patch semantics for mutations
- 30s timeout (configurable)

### Interactive Selector Trait

The `InteractiveSelector` trait abstracts the UI backend. The default `FzfSelector` shells out to fzf. This can be replaced with skim, dialoguer, or a TUI framework without changing browser logic.

### Shell Integration

The `cd`-on-enter pattern uses a temp file (same approach as zoxide, nnn). The `ez init-shell` command generates a shell wrapper function.

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
