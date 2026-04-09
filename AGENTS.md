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
  plugin/           Plugin execution engine
  browser/          Interactive fzf browser
plugins/            Bundled plugin scripts
docs/               Documentation
```

## Key Modules

### config/ - Configuration
- `model.rs`: `EzConfig`, `SelectorConfig`, `PluginsConfig` structs
- `mod.rs`: load/save/edit config

### repo/ - Repository Management
- `model.rs`: `RepoIndex`, `RepoEntry`, `RepoMeta`
- `store.rs`: filesystem persistence
- `mod.rs`: clone, add, remove, list, resolve operations

### session/ - Session Management
- `model.rs`: `Session`, `SessionTree` structs
- `tree.rs`: tree operations (roots, children, ancestors, descendants, render)
- `store.rs`: filesystem persistence
- `mod.rs`: new, delete, enter, exit, rename, ensure_default_session

### plugin/ - Plugin System
- `model.rs`: `PluginManifest`, `HookType` enum (10 hook types)
- `protocol.rs`: `HookRequest`, `HookResponse` JSON types
- `runner.rs`: process execution with timeout
- `mod.rs`: hook dispatch, enable/disable

### browser/ - Interactive Browser
- `selector.rs`: `InteractiveSelector` trait + `FzfSelector` impl
- `mod.rs`: drill-down browse flow, preview handler

## Key Traits

- `InteractiveSelector`: abstracting UI backends (fzf, skim, etc.)
- Repo/session stores use plain functions (not traits yet) — extract if mocking is needed

## Build & Test

```bash
cargo build
cargo test
./target/debug/ez --help
```

## Plugin Development

Plugins are shell scripts or executables in `~/.config/ez/plugins/<name>/`. See `docs/plugin-guide.md` for the JSON protocol. Bundled plugins in `plugins/` directory.

## Conventions

- Files under 500 lines
- `thiserror` for error types
- No async — synchronous CLI
- Shell out to `git` (no libgit2)
- TOML for all config/metadata files
