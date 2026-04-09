# Claude Code Skills

ez-workspaces ships with three proprietary Claude Code skills that provide contextual guidance when working on the codebase.

## Available Skills

### ez-session

Session lifecycle management — creating, listing, deleting, entering, and renaming sessions with tree hierarchy support.

**Use when:** working with workspace sessions, worktrees, or multi-branch workflows.

**Key files:** `src/session/mod.rs`, `src/session/model.rs`, `src/session/tree.rs`, `src/session/store.rs`

### ez-plugin

Plugin development and management — creating custom plugins, debugging plugin issues, or extending ez-workspaces.

**Use when:** writing custom plugins, debugging the JSON protocol, or modifying the plugin runner.

**Key files:** `src/plugin/mod.rs`, `src/plugin/model.rs`, `src/plugin/protocol.rs`, `src/plugin/runner.rs`, `plugins/`

### ez-browse

Interactive fzf browser — the drill-down workspace browser and the `InteractiveSelector` trait.

**Use when:** working on the browser UI, selector trait, preview rendering, or fzf integration.

**Key files:** `src/browser/mod.rs`, `src/browser/selector.rs`

## Location

Skills are defined in `.claude/skills/ez-*/SKILL.md` and are automatically loaded by Claude Code when relevant to the task at hand.
