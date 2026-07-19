# Interactive Browser (Delta)

## MODIFIED Requirements

### Requirement: Session action loop
When a repo is selected, the browser SHALL enter a session action loop that repeatedly shows the repo's sessions as a tree with box-drawing tree connectors (`├──`, `└──`, `│`) and handles keybind actions until the user selects a session (Enter) or cancels (Escape). The loop re-renders after each action to show updated state. Sessions SHALL be rendered with tree glyphs matching the indentation style used in the Tree view. The loop SHALL support additional keybinds: `Alt-Shift-N` for bare session creation, `alt-s` for session-from-dirty, and `ctrl-s` for sort toggle. Sessions with PR metadata SHALL display a colored PR status indicator.

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
- **THEN** system prompts for a new name, renames the session (including branch and worktree if applicable), and re-renders

#### Scenario: Edit labels
- **WHEN** user presses Alt-l on a session
- **THEN** system prompts for comma-separated labels (prefix `-` to remove), applies changes, and re-renders

#### Scenario: Plugin bind action
- **WHEN** user presses a plugin-registered keybind on a session
- **THEN** system runs the plugin's `OnBind` hook with the session context

#### Scenario: Cd keybind
- **WHEN** user presses the `cd_session` keybind (default Ctrl-d) on a session
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

#### Scenario: PR status indicator in session display
- **WHEN** a session has `ez_pr_number` and `ez_pr_status` in its env
- **THEN** the session line displays a colored PR status indicator (e.g. `[PR #42 open]` in green)

## ADDED Requirements

### Requirement: Full rename in browser
The rename action in the session action loop (Alt-r) SHALL perform a full rename: update session name, rename git branch, move worktree directory, and run `OnSessionRename` hooks. For bare or non-git sessions, only the metadata name is updated.

#### Scenario: Browser rename updates branch and worktree
- **WHEN** user presses Alt-r on a git-backed session and enters a new name
- **THEN** system renames the git branch, moves the worktree directory, updates the session metadata, and re-renders

#### Scenario: Browser rename for bare session
- **WHEN** user presses Alt-r on a bare session
- **THEN** system updates only the session name in metadata
