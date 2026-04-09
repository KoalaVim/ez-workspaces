---
name: "ez-session"
description: "Manage ez-workspaces sessions: create, list, delete, enter, and rename sessions with tree hierarchy support. Use when working with workspace sessions, worktrees, or multi-branch workflows."
---

# ez-workspaces Session Management

## What This Skill Does

Manages the full lifecycle of workspace sessions in ez-workspaces. Sessions are tree-based (parent/child) and backed by plugins (git worktree, tmux). This skill handles creation, deletion, navigation, and hierarchy operations.

## Key Files

- `src/session/mod.rs` — Session operations (new, delete, enter, exit, rename)
- `src/session/model.rs` — Session and SessionTree data structures
- `src/session/tree.rs` — Tree operations (roots, children, descendants, render)
- `src/session/store.rs` — TOML persistence

## Commands

```bash
# Create a session
ez session new feature-auth

# Create a child session
ez session new api-tests --parent feature-auth

# List as tree
ez session list

# Delete with cascade
ez session delete feature-auth --force

# Enter (cd + plugin hooks)
ez session enter feature-auth

# Rename
ez session rename old-name new-name
```

## Session Tree Structure

Sessions are stored as a flat list with `parent_id` pointers in `~/.config/ez/repos/<id>/sessions.toml`. The `SessionTree` struct provides tree operations:

- `roots()` — root-level sessions
- `children(id)` — direct children
- `descendants(id)` — all descendants (for cascade delete)
- `render_tree()` — depth-first (depth, session) pairs for display
- `find_by_name(name)` — lookup by name

## Default Session

When a repo is first accessed, a "main" session is auto-created pointing to the repo's working directory (`is_default: true`).

## Plugin Integration

Session operations trigger plugin hooks:
- `on_session_create` — after metadata created
- `on_session_delete` — before metadata removed (children deleted first, bottom-up)
- `on_session_enter` — on enter (plugins can return `shell_commands`)
- `on_session_exit` — on exit

Plugins can set `session.path`, `session.env`, and `session.plugin_state` via `session_mutations` in their response.

## Testing

```bash
cargo test session
```

Unit tests cover tree operations (add, remove, roots, children, descendants, render, duplicate detection, invalid parent).
