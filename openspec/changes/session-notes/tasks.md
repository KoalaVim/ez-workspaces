## 1. Paths and Storage

- [x] 1.1 Add `data_dir()` helper to `src/paths.rs` using `dirs::data_dir()` returning `<data_dir>/ez/`
- [x] 1.2 Add `notes_dir(repo_id, session_id)` helper to `src/paths.rs` returning `<data_dir>/ez/repos/<repo-id>/notes/<session-id>/`
- [x] 1.3 Add `notes_readme(repo_id, session_id)` helper to `src/paths.rs` returning the README.md path

## 2. Notes Module

- [x] 2.1 Create `src/session/notes.rs` with `ensure_notes_dir(repo_id, session_id)` that creates the directory and empty README.md if they don't exist
- [x] 2.2 Add `open_note(repo_id, session_id, command)` that resolves the open command, ensures the dir, and spawns the editor with the README path
- [x] 2.3 Add `resolve_note_open_command(config)` that reads `note_open_command` config, resolves `$EDITOR` from env, and returns the command or an error if unset
- [x] 2.4 Add `delete_notes_dir(repo_id, session_id)` that removes the notes directory if it exists
- [x] 2.5 Add `notes_dir_exists(repo_id, session_id)` predicate for preview pane checks
- [x] 2.6 Register `notes` module in `src/session/mod.rs`

## 3. Configuration

- [x] 3.1 Add `note_open` (default `"alt-i"`) and `note_cd` (default `"alt-I"`) fields to `KeybindsConfig` in `src/config/model.rs`
- [x] 3.2 Add `note_open_command` (default `"$EDITOR"`) field to `EzConfig` in `src/config/model.rs`
- [x] 3.3 Add default functions `default_bind_note_open()`, `default_bind_note_cd()`, `default_note_open_command()` and wire into `Default` impls

## 4. CLI

- [x] 4.1 Add `SessionNoteCommand` enum with `Open`, `Cd`, `Path` variants to `src/cli.rs`, each with optional `--name` and `--repo` args
- [x] 4.2 Add `Note { command: SessionNoteCommand }` variant to `SessionCommand` enum in `src/cli.rs`
- [x] 4.3 Implement `dispatch_note(cmd, cd_file)` in `src/session/mod.rs` that resolves the session (by name or current detection) and calls the appropriate notes function
- [x] 4.4 Wire `SessionCommand::Note` into the `dispatch()` match in `src/session/mod.rs`

## 5. Session Delete Cleanup

- [x] 5.1 Call `notes::delete_notes_dir()` for each session in the delete cascade in `delete_session()` and `delete_session_by_id()` in `src/session/mod.rs`

## 6. Browser Integration

- [x] 6.1 Wire `note_open` keybind in the session action loop (`src/browser/mod.rs`): on press, ensure notes dir, run `execute($EDITOR <readme>)` via fzf, re-render
- [x] 6.2 Wire `note_cd` keybind in the session action loop (`src/browser/mod.rs`): on press, ensure notes dir, write path to cd-file, exit browser
- [x] 6.3 Add note keybinds to `preview_keybind_help()` in `src/browser/preview.rs` so they appear in the session preview pane keybind table

## 7. Preview Pane

- [x] 7.1 Add a "Note" section to `preview_session()` in `src/browser/preview.rs` between Git Info and Keybinds
- [x] 7.2 Implement `bat` rendering: check if README exists and `bat` is available, shell out to `bat --style=plain --color=always --line-range=:20 <path>`, print output
- [x] 7.3 Skip the Note section silently if README doesn't exist or `bat` is not installed

## 8. Documentation

- [x] 8.1 Update `AGENTS.md` with notes module description, new config fields, and CLI commands
- [x] 8.2 Update `docs/user-guide.md` with session notes usage
- [x] 8.3 Update `README.md` with session notes feature
