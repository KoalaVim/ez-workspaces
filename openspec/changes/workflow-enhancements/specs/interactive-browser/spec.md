# Interactive Browser (Delta)

## MODIFIED Requirements

### Requirement: Session action loop
When a repo is selected, the browser SHALL enter a session action loop that repeatedly shows the repo's sessions as a tree with box-drawing tree connectors (`├──`, `└──`, `│`) and handles keybind actions until the user selects a session (Enter) or cancels (Escape). The loop re-renders after each action to show updated state. Sessions SHALL be rendered with tree glyphs matching the indentation style used in the Tree view. The loop SHALL support additional keybinds: `Alt-Shift-N` for bare session creation, `alt-s` for session-from-dirty, and `ctrl-s` for sort toggle.

#### Scenario: Select session
- **WHEN** user presses Enter on a session
- **THEN** system runs the `on_enter` action (default: cd into session path)

#### Scenario: Create child session
- **WHEN** user presses Alt-n on a session
- **THEN** system runs the mode selection and name builder, creates a child of the selected session, and re-renders

#### Scenario: Delete session
- **WHEN** user presses Alt-d on a session
- **THEN** system prompts for confirmation (with dirty worktree warning if applicable), deletes the session, and re-renders

#### Scenario: Rename session
- **WHEN** user presses Alt-r on a session
- **THEN** system prompts for a new name, renames the session, and re-renders

#### Scenario: Edit labels
- **WHEN** user presses Alt-l on a session
- **THEN** system prompts for comma-separated labels (prefix `-` to remove), applies changes, and re-renders

#### Scenario: Plugin bind action
- **WHEN** user presses a plugin-registered keybind on a session
- **THEN** system runs the plugin's `OnBind` hook with the session context

#### Scenario: Cd keybind
- **WHEN** user presses the `cd_session` keybind (default Alt-c) on a session
- **THEN** system writes the session's worktree path to the cd-file regardless of the configured `on_enter` action
- **THEN** the browser exits and the shell wrapper cd's into that path

#### Scenario: Cancel returns to view layer
- **WHEN** user presses Escape in the session action loop
- **THEN** system returns to the previous view level (the view that was active before repo selection)

#### Scenario: Tree glyph rendering
- **WHEN** sessions are displayed in the session action loop
- **THEN** sessions are rendered with box-drawing tree connectors showing parent-child relationships (e.g. `├── child-1`, `└── child-2`, `│   └── grandchild`)

#### Scenario: Create bare session
- **WHEN** user presses `Alt-Shift-N` in the session action loop
- **THEN** system prompts for a session name and creates a bare session (no worktree, no git-worktree hook)

#### Scenario: Session from dirty
- **WHEN** user presses `alt-s` in the session action loop on a session with dirty changes
- **THEN** system prompts for a name, stashes changes, creates new session on same commit, pops stash in new worktree

#### Scenario: Sort toggle in session loop
- **WHEN** user presses `ctrl-s` in the session action loop
- **THEN** session list re-renders sorted by LRU or alphabetical (toggled)

## ADDED Requirements

### Requirement: LRU sort toggle
The browser SHALL support a `ctrl-s` keybind in all views (Repo, Workspace, Owner, Label, Tree, session picker) that toggles the sort order between alphabetical and LRU (most recently accessed first). The current sort mode SHALL be displayed in the fzf header. The sort state SHALL persist across view switches within a single browser session.

#### Scenario: Toggle in repo view
- **WHEN** user presses `ctrl-s` in the Repo view
- **THEN** repos re-render sorted by `last_accessed` descending (or back to alphabetical if already LRU)
- **THEN** header shows current sort mode

#### Scenario: Sort indicator in header
- **WHEN** LRU sort is active
- **THEN** the fzf header displays "Sort: LRU" alongside the view name

#### Scenario: Sort persists across views
- **WHEN** user toggles to LRU and switches from Repo view to Workspace view
- **THEN** the Workspace view also uses LRU sort order

### Requirement: Bare session keybind
The browser SHALL support an `Alt-Shift-N` keybind in the session action loop that creates a bare session without triggering the git-worktree plugin. The keybind SHALL be displayed in the session picker's keybind help preview.

#### Scenario: Keybind shown in help
- **WHEN** user is in the session picker and views the preview pane
- **THEN** the keybind help includes `Alt-Shift-N: New bare session`

#### Scenario: Bare session appears after creation
- **WHEN** user creates a bare session via `Alt-Shift-N`
- **THEN** the session action loop re-renders showing the new bare session with a `[bare]` indicator

### Requirement: Session-from-dirty keybind
The browser SHALL support an `alt-s` keybind in the session action loop that creates a new session from the current session's uncommitted changes. The keybind SHALL only be functional when the selected session has a worktree with dirty changes.

#### Scenario: Keybind shown in help
- **WHEN** user is in the session picker and views the preview pane
- **THEN** the keybind help includes `alt-s: Session from dirty`

#### Scenario: Error on clean worktree
- **WHEN** user presses `alt-s` on a session with no uncommitted changes
- **THEN** system displays "No uncommitted changes to move"

#### Scenario: Error on bare session
- **WHEN** user presses `alt-s` on a bare session
- **THEN** system displays "Cannot create from dirty: session has no worktree"
