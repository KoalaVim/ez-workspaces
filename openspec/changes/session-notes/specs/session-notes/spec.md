## ADDED Requirements

### Requirement: Notes directory per session
The system SHALL maintain a notes directory for each session at `<data_dir>/ez/repos/<repo-id>/notes/<session-id>/` containing a default `README.md` file. The directory and README SHALL be created lazily on first access (open or cd action). The data directory SHALL be resolved via `dirs::data_dir()` (macOS: `~/Library/Application Support`, Linux: `~/.local/share`).

#### Scenario: First access creates directory and README
- **WHEN** user triggers the note open or note cd action for a session that has no notes directory
- **THEN** system creates `<data_dir>/ez/repos/<repo-id>/notes/<session-id>/README.md` with empty content
- **AND** the parent directories are created as needed

#### Scenario: Subsequent access reuses existing directory
- **WHEN** user triggers the note open action for a session that already has a notes directory with content
- **THEN** system opens the existing README.md without modifying it

#### Scenario: User can add arbitrary files
- **WHEN** user creates additional files in the session's notes directory
- **THEN** those files persist alongside README.md and are accessible via the cd action

### Requirement: Open note action
The system SHALL provide an action to open a session's README.md in the configured editor. The editor command SHALL be resolved from `note_open_command` config (default `"$EDITOR"`). If `$EDITOR` is not set and no explicit command is configured, the system SHALL return an error with a clear message.

#### Scenario: Open note with $EDITOR set
- **WHEN** user triggers the open note action and `$EDITOR` is set to `nvim`
- **THEN** system opens `<notes-dir>/README.md` in `nvim`

#### Scenario: Open note with custom command
- **WHEN** config has `note_open_command = "code"`
- **THEN** system opens the README in VS Code

#### Scenario: Open note without $EDITOR
- **WHEN** user triggers the open note action, `note_open_command` is `"$EDITOR"`, and `$EDITOR` is not set
- **THEN** system returns an error: "$EDITOR is not set. Set it or configure note_open_command in config."

### Requirement: Cd to notes directory action
The system SHALL provide an action to change the working directory to a session's notes directory. The action SHALL write the notes directory path to the cd-file (same mechanism as session enter with `on_enter = "cd"`).

#### Scenario: Cd to notes directory
- **WHEN** user triggers the note cd action for a session
- **THEN** system writes the notes directory path to the cd-file
- **AND** the shell wrapper cd's into the notes directory after ez exits

### Requirement: Note path query
The system SHALL provide a way to print the notes directory path for a session to stdout, for use in scripts and automation.

#### Scenario: Print note path
- **WHEN** user runs `ez session note path --name my-session`
- **THEN** system prints the absolute path to the session's notes directory to stdout

#### Scenario: Print note path for current session
- **WHEN** user runs `ez session note path` without `--name`
- **THEN** system detects the current session and prints its notes directory path

### Requirement: CLI subcommands for notes
The system SHALL provide `ez session note` subcommands: `open`, `cd`, and `path`. When `--name` is omitted, the system SHALL resolve the current session using existing detection (tmux env or worktree path matching). The `--repo` flag SHALL be supported for explicit repo targeting.

#### Scenario: CLI open note
- **WHEN** user runs `ez session note open --name feature-auth`
- **THEN** system opens the session's README.md in the configured editor

#### Scenario: CLI cd to notes
- **WHEN** user runs `ez session note cd --name feature-auth`
- **THEN** system writes the notes directory path to the cd-file

#### Scenario: CLI note path
- **WHEN** user runs `ez session note path --name feature-auth`
- **THEN** system prints the absolute path to the notes directory

#### Scenario: CLI without name resolves current session
- **WHEN** user runs `ez session note open` without `--name` while inside a registered session's worktree
- **THEN** system detects the current session and opens its note

### Requirement: Notes cleanup on session delete
The system SHALL delete the session's notes directory when the session is deleted. The cleanup SHALL happen in the synchronous delete flow (before the detached reap worker). If the notes directory does not exist, cleanup SHALL be a no-op.

#### Scenario: Delete session with notes
- **WHEN** user deletes a session that has a notes directory
- **THEN** system removes `<data_dir>/ez/repos/<repo-id>/notes/<session-id>/` and all its contents

#### Scenario: Delete session without notes
- **WHEN** user deletes a session that has no notes directory
- **THEN** cleanup is a no-op (no error)

#### Scenario: Cascade delete cleans up descendant notes
- **WHEN** user deletes a session that has child sessions with notes
- **THEN** system removes notes directories for all deleted sessions (parent and descendants)

### Requirement: Note preview in fzf
The system SHALL display a "Note" section in the session preview pane when the session has a README.md in its notes directory. The content SHALL be rendered via `bat --style=plain --color=always --line-range=:20 <path>`. If `bat` is not installed or the README does not exist, the section SHALL be skipped.

#### Scenario: Preview with note content
- **WHEN** user highlights a session in the browser and the session has a notes README.md with content
- **THEN** the preview pane shows a "Note" section with the first 20 lines rendered by `bat`

#### Scenario: Preview without notes
- **WHEN** user highlights a session that has no notes directory
- **THEN** the preview pane does not show a "Note" section

#### Scenario: Preview without bat installed
- **WHEN** user highlights a session with notes but `bat` is not installed
- **THEN** the "Note" section is skipped (no error, no fallback)
