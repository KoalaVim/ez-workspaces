# Interactive Browser (Delta)

## MODIFIED Requirements

### Requirement: Consistent back navigation
The browser SHALL implement consistent back navigation across all levels. Escape SHALL always return to the previous navigation level: from the session action loop to the view layer, from directory drill-down to the parent directory, from views to exit. Back navigation SHALL never skip levels or exit the browser unexpectedly. Cancelling SHALL never move the user to a navigation level they did not arrive from.

The root navigation level depends on how ez was launched. When the session picker is the entry point — bare `ez` inside a registered repo or one of its worktrees, or `ez --repo <path>` — the session picker IS the root level, and cancelling it SHALL exit ez. It SHALL NOT fall through to the global browser view. Reaching the global browser view from the session picker is done deliberately with the `view_all` keybind (default `ctrl-a`), not by cancelling.

Escape and Ctrl-C SHALL behave identically wherever cancellation is offered.

Cancelling SHALL exit with a success status, the same as cancelling at the top level of any view. Cancellation is a normal outcome, not an error: the shell wrapper propagates ez's exit status to the calling shell, so a failure status would mark every cancelled browse as an error in the user's prompt.

#### Scenario: Escape in session loop returns to view
- **WHEN** user presses Escape in the session action loop after selecting a repo from any view
- **THEN** system returns to the view that was active before repo selection (Workspace, Repo, Tree, etc.)

#### Scenario: Escape in directory drill-down
- **WHEN** user presses Escape while drilling into a workspace directory (not at root)
- **THEN** system returns to the parent directory

#### Scenario: Escape at top level exits
- **WHEN** user presses Escape at the top level of any view (workspace root list, repo list, tree view)
- **THEN** system exits the browser with a success status

#### Scenario: Escape in an auto-detected repo exits
- **WHEN** user runs bare `ez` inside a registered repo, the session picker opens by auto-detect, and the user presses Escape
- **THEN** ez exits with a success status
- **AND** the global browser view is not shown
- **AND** the shell returns to the directory it was launched from

#### Scenario: Ctrl-C in an auto-detected repo exits
- **WHEN** user runs bare `ez` inside a registered repo and presses Ctrl-C in the session picker
- **THEN** ez exits with a success status
- **AND** the global browser view is not shown

#### Scenario: Escape with `--repo` exits
- **WHEN** user runs `ez --repo <path>` and presses Escape in the session picker
- **THEN** ez exits with a success status
- **AND** the global browser view is not shown

#### Scenario: Escape inside a worktree of an auto-detected repo exits
- **WHEN** user runs bare `ez` inside a worktree of a registered repo and presses Escape in the session picker
- **THEN** ez exits with a success status

#### Scenario: View-all keybind still reaches the global view
- **WHEN** user is in a root-entry session picker and presses the `view_all` keybind
- **THEN** the global browser view opens in the default view mode
- **AND** cancelling there exits ez

#### Scenario: Keybind help labels Escape by navigation level
- **WHEN** user views the session picker's preview-pane keybind help in a root-entry session picker
- **THEN** the help shows Escape as `Quit`

#### Scenario: Keybind help shows go-back when nested
- **WHEN** user views the session picker's preview-pane keybind help after reaching it by selecting a repo from a view
- **THEN** the help shows Escape as `Go back`

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

#### Scenario: Cancel from an auto-detected session picker exits
- **WHEN** user cancels the session picker that auto-detect opened
- **THEN** ez exits with a success status rather than falling through to the view layer

### Requirement: Session action loop
When a repo is selected, the browser SHALL enter a session action loop that repeatedly shows the repo's sessions as a tree with box-drawing tree connectors (`├──`, `└──`, `│`) and handles keybind actions until the user selects a session (Enter) or cancels (Escape). The loop re-renders after each action to show updated state. Sessions SHALL be rendered with tree glyphs matching the indentation style used in the Tree view. The loop SHALL support additional keybinds: `Alt-Shift-N` for bare session creation, `alt-s` for session-from-dirty, `ctrl-s` for sort toggle, `alt-i` for opening the session's note in the configured editor, and `alt-I` for cd-ing to the session's notes directory.

Before rendering items, the loop SHALL build a worktree info cache by calling `git worktree list --porcelain` once. This cache SHALL be used for both per-session branch display (HashMap lookup instead of `git symbolic-ref` subprocess) and unmanaged worktree detection. The cache SHALL be rebuilt on each loop iteration to reflect any branch changes caused by actions.

After the managed session items, the picker SHALL append a "Not Registered" section showing non-managed git worktrees detected from the same worktree cache. These items SHALL appear below a non-interactive header line reading "Not Registered". Each non-managed worktree SHALL display the branch name (dimmed) and worktree path (dimmed). Selecting a non-managed worktree SHALL register it as a session under the default (main) session and enter it using the configured `on_enter` action. The "Not Registered" section SHALL only appear when there are non-managed worktrees to show.

#### Scenario: Session branches resolved from worktree cache
- **WHEN** the session action loop renders items for a repo with 11 sessions
- **THEN** the system calls `git worktree list --porcelain` once (not `git symbolic-ref` 11 times)
- **AND** each session's branch indicator is resolved via HashMap lookup from the parsed output

#### Scenario: Re-render after sort toggle uses single git call
- **WHEN** user presses `ctrl-s` to toggle sort in the session picker
- **THEN** the loop rebuilds the worktree cache with one `git worktree list` call
- **AND** does not spawn per-session `git symbolic-ref` calls

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
- **WHEN** user presses Escape in the session action loop, having reached it by selecting a repo from a view
- **THEN** system returns to the previous view level (the view that was active before repo selection)

#### Scenario: Cancel exits when the session action loop is the root view
- **WHEN** user presses Escape in the session action loop, having reached it directly via auto-detect or `--repo`
- **THEN** ez exits with a success status and does not open any other view

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

