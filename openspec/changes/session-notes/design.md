## Context

Sessions in ez are metadata records backed by plugins (git worktrees, tmux sessions). They carry env vars, labels, and plugin state, but have no built-in mechanism for free-form user notes. Users context-switch between sessions frequently and lose track of what they were doing.

The system already has per-session data keyed by session ID (stable across renames) and a well-defined lifecycle (create → enter → rename → delete). Notes plug into this lifecycle as external file-based storage.

All ez metadata currently lives under `~/.config/ez/` (config dir). Notes are user data, not configuration, and belong in the OS data directory (`dirs::data_dir()`).

## Goals / Non-Goals

**Goals:**
- Give each session a persistent notes directory with a default `README.md`
- Provide two browser actions (open note, cd to notes) bound to configurable keys
- Provide CLI equivalents (`ez session note open/cd/path`)
- Show note content in the fzf preview pane via `bat`
- Clean up notes on session delete
- Keep the feature self-contained with minimal impact on existing modules

**Non-Goals:**
- Structured/typed notes or metadata extraction from note content
- Note search or indexing across sessions
- Syncing notes to remote or version control
- Note templates beyond the initial empty `README.md`
- Moving existing ez data (repos, sessions) to the data dir (separate concern)

## Decisions

### 1. Storage location: OS data directory keyed by session ID

Notes live at `<data_dir>/ez/repos/<repo-id>/notes/<session-id>/README.md`.

- `dirs::data_dir()` resolves to `~/Library/Application Support` on macOS, `~/.local/share` on Linux.
- Keying by session ID (UUID) means renames are a no-op for storage.
- The directory structure mirrors `repos/<repo-id>/` from the config dir for consistency.
- Users can add arbitrary files alongside `README.md`.

Alternative considered: storing on `~/.config/ez/`. Rejected because notes are user data, not configuration. The XDG spec and macOS conventions distinguish between config and data directories.

Alternative considered: field on the `Session` struct in `sessions.toml`. Rejected because TOML multiline strings are awkward, every session load/save would touch note content, and the file would grow unboundedly.

### 2. Lazy creation

The notes directory and `README.md` are created on first access (open or cd action), not on session creation. This avoids creating empty directories for sessions that never use notes.

`ensure_notes_dir()` handles both mkdir and writing an empty `README.md` if it doesn't exist.

### 3. Open command resolution

`note_open_command` defaults to `"$EDITOR"`. At runtime, the system resolves `$EDITOR` from the environment. If `$EDITOR` is not set and no `note_open_command` is configured, the system errors with a clear message.

The open command is invoked with the README path as its sole argument: `$EDITOR <path>/README.md`. In the fzf browser, this uses fzf's `execute(...)` action to hand the terminal to the editor (blocking). Terminal editors (vim, nvim) work because they take over the TTY; GUI editors (code, cursor) work because they return immediately.

### 4. Preview rendering via `bat`

The preview pane adds a "Note" section between "Git Info" (or "Metadata") and "Keybinds". It shells out to `bat --style=plain --color=always --line-range=:20 <path>` to render the first 20 lines with syntax highlighting. If `bat` is not installed or the README doesn't exist, the section is skipped entirely.

Alternative considered: reading the file in Rust and printing raw. Rejected because `bat` provides markdown syntax highlighting for free and matches the quality users expect from a preview pane.

### 5. Cleanup on session delete

When a session is deleted, `delete_notes_dir()` removes the entire `<data_dir>/ez/repos/<repo-id>/notes/<session-id>/` directory. This is called in the main delete flow (before the detached reap worker), since it's a fast local filesystem operation that doesn't need terminal immunity.

### 6. CLI structure

Notes are a subcommand of session: `ez session note {open,cd,path}`. This keeps the CLI hierarchy flat and discoverable. `open` and `cd` resolve the current session when `--name` is omitted (reusing existing `current::resolve_current_session()`).

### 7. Keybinds: `alt-i` / `alt-I`

`alt-i` opens the README, `alt-I` (Alt-Shift-I) cd's to the notes directory. Both are free (no fzf default or existing ez binding conflicts). They appear in the session preview pane keybind table.

## Risks / Trade-offs

- **`bat` dependency for preview**: `bat` is optional; if absent, the note preview section is silently skipped. Users who want preview rendering need `bat` installed. This is acceptable since `bat` is widely available and ez already has external tool dependencies (git, fzf, gh).
- **Data dir divergence**: Notes live in a different base directory than sessions metadata. This means `~/.config/ez/` and `<data_dir>/ez/` both contain per-repo data, which could confuse users exploring the filesystem. Acceptable for now; a future change could migrate all data to the data dir.
- **No migration**: Session delete is the only lifecycle event that touches notes. If a user manually deletes the data dir, notes are lost silently. No backup or sync mechanism is provided.
