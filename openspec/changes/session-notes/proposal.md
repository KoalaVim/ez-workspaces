## Why

Sessions in ez represent focused work contexts (branches, worktrees, tasks), but there's no built-in way to attach notes or context to them. When context-switching between sessions, users lose track of what they were doing, what's left to do, or what they discovered. A per-session notes directory solves this by giving each session a persistent scratchpad.

## What Changes

- Each session gets a notes directory (under the OS data dir, keyed by session ID) with a default `README.md` created lazily on first access.
- Two new browser keybinds: `alt-i` to open the session's README in `$EDITOR`, `alt-I` to cd into the notes directory.
- Two new CLI subcommands: `ez session note open` and `ez session note cd` (plus `path` for scripting).
- The fzf preview pane gains a "Note" section that renders the README via `bat` when it exists.
- New config fields: `note_open` and `note_cd` keybinds, `note_open_command` (defaults to `$EDITOR`, errors if unset).
- Session deletion cleans up the associated notes directory.

## Capabilities

### New Capabilities
- `session-notes`: Per-session notes directory with README, open/cd actions, CLI commands, browser keybinds, and preview rendering.

### Modified Capabilities
- `session-management`: Session delete flow must clean up the notes directory.
- `interactive-browser`: Session view gains two new keybinds and a preview pane section.
- `configuration`: New keybind fields (`note_open`, `note_cd`) and `note_open_command` setting.

## Impact

- **Code**: `paths.rs` (new `data_dir`), new `session/notes.rs` module, `config/model.rs` (keybinds + setting), `cli.rs` (subcommand), `session/mod.rs` (dispatch + delete cleanup), `browser/mod.rs` (keybind wiring), `browser/preview.rs` (note section).
- **Dependencies**: None new. Uses `bat` for preview rendering (optional external tool, graceful fallback).
- **Storage**: New directory tree under `<data_dir>/ez/repos/<repo-id>/notes/<session-id>/`.
- **Config**: Additive — new optional fields with defaults, fully backward-compatible.
