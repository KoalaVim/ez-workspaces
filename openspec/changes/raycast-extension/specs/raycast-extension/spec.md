## ADDED Requirements

### Requirement: Browse Repos command
The extension SHALL provide a "Browse Repos" command as the main entry point that displays all registered repos in a searchable Raycast `List`. Each repo item SHALL show the repo name as title, the repo path as subtitle, and labels as tag accessories.

#### Scenario: User opens Browse Repos with repos registered
- **WHEN** user opens the "Browse Repos" command and repos are registered
- **THEN** all repos from `ez repo list --json` are displayed as list items with name, path, and labels

#### Scenario: User opens Browse Repos with no repos
- **WHEN** user opens the "Browse Repos" command and no repos are registered
- **THEN** an empty state message is shown indicating no repos are registered

#### Scenario: ez binary not found
- **WHEN** the `ez` binary is not found in PATH
- **THEN** an error toast is shown with instructions to install `ez`

### Requirement: View mode switching
The Browse Repos command SHALL support switching between view modes via a `List.Dropdown` in the search bar. Available modes SHALL be: Repo (flat list), Owner (grouped by path parent directory), Label (grouped by label), and Workspace (grouped by workspace root).

#### Scenario: Switch to Owner view
- **WHEN** user selects "Owner" from the view dropdown
- **THEN** repos are grouped into `List.Section` elements where each section title is the parent directory name

#### Scenario: Switch to Label view
- **WHEN** user selects "Label" from the view dropdown
- **THEN** repos are grouped into `List.Section` elements where each section title is a label name, and repos without labels appear in an "Unlabeled" section

#### Scenario: Switch to Workspace view
- **WHEN** user selects "Workspace" from the view dropdown
- **THEN** repos are grouped into `List.Section` elements where each section title is the workspace root path

### Requirement: Repo action panel
Each repo item SHALL have an action panel with the following actions: push to session list (default/Enter), open in Finder, open in Terminal, open in Cursor, copy path to clipboard, and remove repo (with confirmation).

#### Scenario: User presses Enter on a repo
- **WHEN** user presses Enter on a repo item
- **THEN** the session list for that repo is pushed onto the navigation stack

#### Scenario: User opens repo in Finder
- **WHEN** user selects "Show in Finder" action on a repo
- **THEN** Finder opens with the repo directory revealed

#### Scenario: User copies repo path
- **WHEN** user selects "Copy Path" action on a repo
- **THEN** the repo's absolute path is copied to the clipboard

#### Scenario: User removes a repo
- **WHEN** user selects "Remove" action on a repo and confirms the alert
- **THEN** the system runs `ez repo remove <name>` and the repo disappears from the list

### Requirement: Session list drill-down
When a user selects a repo, the extension SHALL push a session list view showing all sessions for that repo. Sessions SHALL be rendered with tree hierarchy (indentation and glyphs in title), LRU timestamp as accessory text, and bare/default badges as tag accessories.

#### Scenario: Repo has sessions with tree hierarchy
- **WHEN** user pushes into a repo that has parent and child sessions
- **THEN** sessions are displayed with tree indentation glyphs (├──, └──) reflecting the parent-child hierarchy

#### Scenario: Repo has no sessions
- **WHEN** user pushes into a repo that has no sessions
- **THEN** an empty state message is shown indicating no sessions exist

#### Scenario: Session has PR metadata
- **WHEN** a session has `ez_pr_number` and `ez_pr_url` in its env
- **THEN** a PR tag accessory is shown with the PR number and status

### Requirement: Session action panel
Each session item SHALL have an action panel with: enter session in Terminal (default), open path in Cursor, open path in Finder, copy path, open PR in browser (if PR metadata exists), and delete session (with confirmation).

#### Scenario: User enters a session
- **WHEN** user presses Enter on a session item
- **THEN** Terminal.app opens and runs `ez session enter <name> --repo <repo>`

#### Scenario: User opens session in Cursor
- **WHEN** user selects "Open in Cursor" action on a session with a path
- **THEN** the Cursor editor opens targeting the session's worktree path

#### Scenario: User deletes a session
- **WHEN** user selects "Delete" action on a session and confirms the alert
- **THEN** the system runs `ez session delete <name> --repo <repo> --force` and the session disappears from the list

#### Scenario: User opens PR in browser
- **WHEN** user selects "Open PR" action on a session with `ez_pr_url` in env
- **THEN** the default browser opens the PR URL

#### Scenario: Bare session actions
- **WHEN** user views actions for a bare session (no worktree path)
- **THEN** path-dependent actions (Open in Cursor, Show in Finder, Copy Path) are hidden

### Requirement: Cross-repo session search
The extension SHALL provide a "Search Sessions" command that lists all sessions across all repos in a flat searchable list, grouped by repo name using `List.Section`.

#### Scenario: User searches for a session by name
- **WHEN** user opens "Search Sessions" and types a session name
- **THEN** Raycast filters to matching sessions across all repos

#### Scenario: Session found in specific repo
- **WHEN** matching sessions are displayed
- **THEN** each session shows its repo name as section header and has the same action panel as the drill-down session list

### Requirement: Terminal integration via AppleScript
The extension SHALL open Terminal.app via AppleScript to execute `ez` commands that require a terminal context (entering sessions). The command string SHALL be properly escaped to prevent injection.

#### Scenario: Enter session opens Terminal
- **WHEN** user triggers "Enter Session" action
- **THEN** Terminal.app activates, a new window/tab runs `ez session enter <name> --repo <repo>`, and Raycast closes

#### Scenario: Open path in Terminal
- **WHEN** user triggers "Open in Terminal" action on a repo or session
- **THEN** Terminal.app activates with a new window/tab cd'd to the target path
