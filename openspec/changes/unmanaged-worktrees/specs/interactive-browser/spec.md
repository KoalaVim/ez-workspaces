## MODIFIED Requirements

### Requirement: Session action loop
When a repo is selected, the browser SHALL enter a session action loop that repeatedly shows the repo's sessions as a tree with box-drawing tree connectors (`├──`, `└──`, `│`) and handles keybind actions until the user selects a session (Enter) or cancels (Escape). The loop re-renders after each action to show updated state. Sessions SHALL be rendered with tree glyphs matching the indentation style used in the Tree view. The loop SHALL support additional keybinds: `Alt-Shift-N` for bare session creation, `alt-s` for session-from-dirty, `ctrl-s` for sort toggle, `alt-i` for opening the session's note in the configured editor, and `alt-I` for cd-ing to the session's notes directory.

After the managed session items, the picker SHALL append a "Not Registered" section showing non-managed git worktrees detected via `git worktree list`. These items SHALL appear below a non-interactive header line reading "Not Registered". Each non-managed worktree SHALL display the branch name (dimmed) and worktree path (dimmed). Selecting a non-managed worktree SHALL register it as a session under the default (main) session and enter it using the configured `on_enter` action. The "Not Registered" section SHALL only appear when there are non-managed worktrees to show.

#### Scenario: Non-managed worktrees shown below sessions
- **WHEN** a repo has git worktrees not tracked as sessions
- **THEN** the session picker shows them below managed sessions under a "Not Registered" header
- **AND** each item displays the branch name and path, visually dimmed

#### Scenario: Select non-managed worktree registers and enters
- **WHEN** user selects a non-managed worktree in the picker
- **THEN** system registers it as a session (child of default session) and enters it via `on_enter`
- **AND** on the next render, the worktree appears as a normal managed session

#### Scenario: No non-managed worktrees hides section
- **WHEN** all git worktrees are tracked as sessions
- **THEN** the "Not Registered" section does not appear in the picker

#### Scenario: Non-interactive header line
- **WHEN** user selects the "Not Registered" header line
- **THEN** nothing happens (the loop re-renders)

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

#### Scenario: Open note keybind
- **WHEN** user presses `alt-i` in the session action loop on a session
- **THEN** system ensures the notes directory and README.md exist
- **AND** opens the README.md in the configured editor via fzf's `execute(...)` (blocking, hands terminal to editor)
- **AND** after the editor exits, the session loop re-renders

#### Scenario: Cd to notes keybind
- **WHEN** user presses `alt-I` in the session action loop on a session
- **THEN** system ensures the notes directory exists
- **AND** writes the notes directory path to the cd-file
- **AND** the browser exits and the shell wrapper cd's into the notes directory

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
- **THEN** the session line displays a colored PR status indicator (e.g. `[PR #42 open]` in green, `[PR #42 merged]` in magenta)

#### Scenario: Full rename in browser
- **WHEN** user presses Alt-r on a git-backed session and enters a new name
- **THEN** system renames the git branch, moves the worktree directory, updates the session metadata, and re-renders
