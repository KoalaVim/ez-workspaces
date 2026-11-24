## Purpose

Provide a full-screen, single-process native terminal browser for ez. It offers the same views, actions and keybinds as the fzf browser without spawning a process for each navigation step, so view switching, preview and startup feel instant.

## ADDED Requirements

### Requirement: Native browser activation
The system SHALL run the native browser when `selector.backend = "native"`, and SHALL otherwise keep using the fzf browser. The native browser SHALL support the same entry points as the fzf browser, including `--select-by`, `--workspace`, `--repo`, `--cd-file` and `--post-cmd-file`.

#### Scenario: Native backend selected
- **WHEN** config has `[selector] backend = "native"` and the user runs `ez`
- **THEN** the native full-screen browser opens in the configured default view

#### Scenario: Fzf remains default
- **WHEN** `selector.backend` is absent or set to `"fzf"`
- **THEN** the fzf browser runs with unchanged behavior

#### Scenario: Starting view flag
- **WHEN** the user runs `ez --select-by tree` with the native backend
- **THEN** the native browser opens in the Tree view

### Requirement: Full-screen rendering
The native browser SHALL render on the terminal's alternate screen and SHALL restore the terminal (main screen, cursor, cooked mode) on every exit path, including errors, Ctrl-C and panics.

#### Scenario: Normal exit restores terminal
- **WHEN** the user exits the native browser
- **THEN** the shell's previous screen content and scrollback are visible and the terminal is in normal mode

#### Scenario: Panic restores terminal
- **WHEN** the native browser panics
- **THEN** the terminal is restored before the panic message is printed

### Requirement: In-memory view switching
The native browser SHALL provide the Tree, Workspace, Repo, Owner, Label and plugin views. Switching between views with the configured view keybinds SHALL NOT restart the process or re-read data that is already loaded. Each view SHALL keep its filter query and cursor position while the user is in another view.

#### Scenario: Switch and return preserves state
- **WHEN** the user types `api` in the Repo view, moves the cursor to the third match, switches to Tree view, and then switches back
- **THEN** the Repo view shows query `api` with the cursor on the same item

#### Scenario: View switch does not flicker
- **WHEN** the user presses a view-switch key
- **THEN** the new view replaces the old one in the next frame without the screen being cleared to the shell

### Requirement: Navigation stack
The native browser SHALL keep a navigation stack of levels (for example: view, then repo session list, then action menu). Esc SHALL pop one level and SHALL exit the browser only at the top-most level. Ctrl-C SHALL exit the browser from any level with the interrupted status.

#### Scenario: Esc goes back one level
- **WHEN** the user is in a repo's session list opened from the Repo view and presses Esc
- **THEN** the Repo view is shown again with its previous cursor position

#### Scenario: Esc at top exits
- **WHEN** the user presses Esc in a top-level view
- **THEN** the browser exits without writing a cd target

#### Scenario: Ctrl-C quits from any depth
- **WHEN** the user presses Ctrl-C inside a modal prompt nested in a session list
- **THEN** the browser exits immediately with the interrupted status

### Requirement: Fuzzy filtering
Every list in the native browser SHALL support incremental fuzzy filtering on the visible text (ANSI styling excluded). Matching characters SHALL be highlighted and results ranked by match quality. Filtering SHALL update on every keystroke.

#### Scenario: Filter narrows list
- **WHEN** the user types `ezw` in a list containing `ez-workspaces` and `kv`
- **THEN** only `ez-workspaces` is shown, with the matched characters highlighted

#### Scenario: Styling ignored in matching
- **WHEN** an item's display contains ANSI color codes
- **THEN** matching only considers the visible characters

### Requirement: In-process preview
The native browser SHALL render the preview pane in-process, with the same sections and content as the fzf preview for repos, sessions, unmanaged worktrees, non-git repos and directories. Preview content SHALL be cached per item for the browser session and refreshed when the underlying data changes. Sections that need slow work (git log, bat-rendered notes, PR status) SHALL load in the background without blocking cursor movement.

#### Scenario: Instant preview on cursor move
- **WHEN** the user moves the cursor to an item whose preview has already been rendered
- **THEN** the cached preview is shown in the same frame

#### Scenario: Slow section loads asynchronously
- **WHEN** the user moves to a session whose recent commits are not yet loaded
- **THEN** the preview shows the fast sections immediately with a loading placeholder for commits, and the commits replace the placeholder when ready

#### Scenario: Plugin view preview
- **WHEN** a plugin view item is highlighted
- **THEN** the preview is produced by the plugin as in the fzf browser, run in the background

### Requirement: Background data loading
The native browser SHALL load slow data (git branch, PR status, plugin view items) on background threads and update affected rows in place once results arrive. The UI SHALL stay responsive to input while loads are pending.

#### Scenario: Branch indicators fill in
- **WHEN** the browser opens and branch data for some repos is not cached
- **THEN** those rows show a placeholder, and update to the branch name as each lookup completes, without the user doing anything

#### Scenario: Input during loading
- **WHEN** background loads are pending and the user types a filter query
- **THEN** the list filters immediately

### Requirement: Snapshot first paint
The native browser SHALL persist a snapshot of the last rendered view rows to the ez data directory on exit. On start, it SHALL draw from the snapshot before any fresh data is loaded, then replace the rows with fresh data. A missing, corrupt or version-mismatched snapshot SHALL be ignored silently (logged at debug level).

#### Scenario: Relaunch draws immediately
- **WHEN** a plugin bind triggers `#EZ_RELAUNCH` and the shell wrapper restarts ez
- **THEN** the first frame shows the rows from the previous run before fresh data finishes loading

#### Scenario: Stale snapshot corrected
- **WHEN** a session was deleted outside ez after the snapshot was written
- **THEN** the session appears in the first frame and disappears once fresh data is loaded

#### Scenario: Corrupt snapshot ignored
- **WHEN** the snapshot file cannot be parsed
- **THEN** the browser starts normally from fresh data and logs the error at debug level

### Requirement: Action and effect parity
The native browser SHALL support every action available in the fzf browser: entering sessions, new/bare/from-dirty session, delete, rename, cd, labels, notes, sort toggle, clone/remove repo, and plugin binds. These actions SHALL produce the same `cd_file`, `post_cmd_file` and `#EZ_RELAUNCH` outputs as the fzf browser.

#### Scenario: Enter session writes cd target
- **WHEN** the user accepts a session
- **THEN** the session path is written to the cd file and the browser exits, exactly as in the fzf browser

#### Scenario: Plugin bind relaunch
- **WHEN** a plugin bind returns post shell commands with `accept = false`
- **THEN** the browser writes the commands and the relaunch marker to the post-cmd file and exits

### Requirement: Suspend for external programs
When an action runs an interactive external program (editor for notes or config, `$EDITOR`, plugins that need the terminal), the native browser SHALL leave the alternate screen and restore normal terminal mode before the program runs. It SHALL re-enter and redraw when the program exits.

#### Scenario: Open note in editor
- **WHEN** the user presses the note-open key on a session
- **THEN** the editor runs on the normal screen, and when it exits the browser redraws at the same position

### Requirement: Modal prompts
Interactive prompts started from the native browser (name builder stages, free-text input, confirmation, multi-select, label editing) SHALL render as modals over the browser on the same terminal. They SHALL support the same back (Ctrl-P), cancel (Esc) and interrupt (Ctrl-C) behavior as their fzf counterparts.

#### Scenario: Name builder in a modal
- **WHEN** the user presses the new-session key
- **THEN** the name builder stages appear as modals over the session list, and the list is shown again after the session is created or the flow is cancelled

#### Scenario: Back in a modal stage
- **WHEN** the user presses Ctrl-P in the second name-builder stage
- **THEN** the first stage modal is shown again

### Requirement: Fzf-style keybind compatibility
The native browser SHALL interpret keybind strings from `[keybinds]` and from plugin manifests using fzf key syntax, including `ctrl-<k>`, `alt-<k>`, `alt-shift-<k>`, uppercase letters as shift, and named keys such as `enter`, `tab`, `f1`–`f12`. An unparseable key string SHALL produce a warning naming the setting, and that binding SHALL be disabled without preventing startup.

#### Scenario: Default keybinds work
- **WHEN** the native browser runs with default config
- **THEN** `alt-n` creates a session and `ctrl-t` switches to Tree view

#### Scenario: Uppercase means shift
- **WHEN** `note_cd = "alt-I"`
- **THEN** Alt-Shift-i triggers note cd

#### Scenario: Invalid key string
- **WHEN** a keybind is set to `"hyper-x"`
- **THEN** the browser starts, shows a warning naming that keybind, and that action has no key

### Requirement: Keybind help in preview
The native browser SHALL show the same keybind tables as the fzf browser (main-view and session-action tables) in the preview pane, reflecting the user's configured keys and plugin binds.

#### Scenario: Custom key shown in help
- **WHEN** `new_session = "alt-c"` is configured
- **THEN** the keybind table in the preview lists `alt-c` for new session
