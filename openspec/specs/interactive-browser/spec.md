# Interactive Browser

## Purpose

Provide a fast, keyboard-driven interactive browsing experience for navigating workspaces, repos, and sessions. The browser is the primary user interface, launched by the bare `ez` command. It presents multiple views of the same data, supports view switching via keybinds, and provides an action loop for session management operations — all rendered through the `InteractiveSelector` trait abstraction.

## Requirements

### Requirement: View system with switching
The browser SHALL support multiple top-level views: Tree, Workspace, Repo, Owner, Label, and Plugin views. Users SHALL switch between views using configurable keybinds without restarting the browser. The dispatch loop re-enters the chosen view on each switch.

#### Scenario: Switch between views
- **WHEN** user presses Ctrl-e (default) while in the Workspace view
- **THEN** browser exits the current fzf instance and renders the Repo view

#### Scenario: View header shows available switches
- **WHEN** any view is displayed
- **THEN** the fzf header shows the current view name and all available view-switch keybinds (including plugin views)

#### Scenario: Select starting view
- **WHEN** user runs `ez --select-by tree`
- **THEN** browser starts in Tree view instead of the configured default

#### Scenario: Default view from config
- **WHEN** user runs bare `ez` without `--select-by`
- **THEN** browser starts in the view specified by `default_select_by` config (default: `workspace`)

### Requirement: Tree view
The Tree view SHALL render all workspace roots, their repos, and each repo's sessions in a single indented tree with ASCII box-drawing connectors. Selecting a session SHALL enter it using the `accept_session` flow (which handles the configured `on_enter` action including plugin binds like tmux attach), passing the `post_cmd_file` for post-exit commands. Selecting a workspace root SHALL re-render.

#### Scenario: Render full tree
- **WHEN** Tree view is displayed
- **THEN** system shows workspace roots as top-level nodes, repos as children with branch info, and sessions as nested children with star markers for defaults

#### Scenario: Select session in tree
- **WHEN** user selects a session row in the tree
- **THEN** system runs the `accept_session` flow with `post_cmd_file` passthrough, applying the configured `on_enter` action (cd, tmux attach, or other plugin bind)

### Requirement: Workspace view with drill-down
The Workspace view SHALL first present a list of configured workspace roots. On selection, it SHALL drill into directories level by level until a git repo is found, then transition to the session picker. Directories with a `.git` folder are shown with branch info; hidden directories (starting with `.`) are excluded.

#### Scenario: Drill into directories
- **WHEN** user selects a workspace root
- **THEN** system lists subdirectories; git repos show branch and labels; non-repos show as blue directories
- **THEN** selecting a non-repo directory drills deeper; selecting a repo transitions to the session picker

#### Scenario: Back navigation during drill-down
- **WHEN** user presses Escape during directory drill-down (not at the top level)
- **THEN** system returns to the parent directory

#### Scenario: Jump to workspace
- **WHEN** user runs `ez --workspace personal`
- **THEN** system skips the root picker and starts drill-down in the matching workspace root

### Requirement: Repo view
The Repo view SHALL display a flat list of all registered repos with name, path, branch, and labels. Selecting a repo SHALL transition to its session picker. The view SHALL also support session actions (new, delete, rename, labels) and view-switch keybinds.

#### Scenario: Display all repos
- **WHEN** Repo view is displayed
- **THEN** system shows all registered repos with their current branch and labels

### Requirement: Owner view
The Owner view SHALL group registered repos by owner (parsed from remote URL). Each owner is a header; repos are listed under their owner.

#### Scenario: Group by owner
- **WHEN** Owner view is displayed
- **THEN** repos are grouped under headers like `rust-lang`, `ofirg`, etc.

### Requirement: Label view
The Label view SHALL group repos and sessions by their user-defined labels. Each label is a header; items carrying that label are listed underneath. The view SHALL support edit-labels keybind.

#### Scenario: Group by label
- **WHEN** Label view is displayed
- **THEN** items are grouped under their label headers (e.g. `backend`, `wip`)

### Requirement: Plugin views
Plugin views SHALL be provided by enabled plugins via manifest `[[views]]` entries. They appear alongside core views in the header and keybind list. The browser calls `OnView` to get items and `OnViewSelect` when the user selects one.

#### Scenario: Tmux view
- **WHEN** tmux plugin is enabled and user presses Ctrl-a
- **THEN** browser calls `OnView` hook, displays tmux session items in fzf
- **THEN** on selection, calls `OnViewSelect` which returns `post_shell_commands` to switch tmux client

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
- **THEN** the session line displays a colored PR status indicator (e.g. `[PR #42 open]` in green, `[PR #42 merged]` in magenta)

#### Scenario: Full rename in browser
- **WHEN** user presses Alt-r on a git-backed session and enters a new name
- **THEN** system renames the git branch, moves the worktree directory, updates the session metadata, and re-renders

### Requirement: Auto-detect current repo
The browser SHALL auto-detect whether the user is inside a registered repo (or one of its worktrees) and skip directly to the session picker. This uses `git rev-parse --git-common-dir` to resolve the main repo root from worktrees.

#### Scenario: Auto-detect from repo
- **WHEN** user runs `ez` while inside a registered repo
- **THEN** browser skips the view layer and shows the session picker for that repo

#### Scenario: Auto-detect from worktree
- **WHEN** user runs `ez` while inside a worktree of a registered repo
- **THEN** browser resolves the common repo root and shows the session picker

#### Scenario: Force full browser
- **WHEN** user runs `ez --all`
- **THEN** auto-detection is skipped and the full view layer is shown

### Requirement: Preview pane
The browser SHALL show a preview pane (right 50%) in fzf. The preview calls back into the ez binary (`ez preview <path>`) to render repo info, git status, or keybind help depending on context.

#### Scenario: Repo preview
- **WHEN** user highlights a repo or directory in the browser
- **THEN** preview pane shows git branch, recent commits, and repo info

#### Scenario: Session actions preview
- **WHEN** user is in the session picker
- **THEN** preview pane shows keybind help for available session actions

### Requirement: Label input parsing
The browser SHALL parse comma-separated label edit strings where labels prefixed with `-` indicate removal.

#### Scenario: Mixed add and remove
- **WHEN** user enters `foo, bar, -baz`
- **THEN** system adds `foo` and `bar`, removes `baz`

#### Scenario: Bare dash ignored
- **WHEN** user enters `-`
- **THEN** system ignores the bare dash (no add, no remove)

### Requirement: Consistent back navigation
The browser SHALL implement consistent back navigation across all levels. Escape SHALL always return to the previous navigation level: from the session action loop to the view layer, from directory drill-down to the parent directory, from views to exit. Back navigation SHALL never skip levels or exit the browser unexpectedly.

#### Scenario: Escape in session loop returns to view
- **WHEN** user presses Escape in the session action loop after selecting a repo from any view
- **THEN** system returns to the view that was active before repo selection (Workspace, Repo, Tree, etc.)

#### Scenario: Escape in directory drill-down
- **WHEN** user presses Escape while drilling into a workspace directory (not at root)
- **THEN** system returns to the parent directory

#### Scenario: Escape at top level exits
- **WHEN** user presses Escape at the top level of any view (workspace root list, repo list, tree view)
- **THEN** system exits the browser with code 130

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
