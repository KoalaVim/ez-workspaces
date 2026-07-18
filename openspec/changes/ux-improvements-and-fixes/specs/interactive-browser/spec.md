# Interactive Browser (Delta)

## MODIFIED Requirements

### Requirement: Tree view

The Tree view SHALL render all workspace roots, their repos, and each repo's sessions in a single indented tree with ASCII box-drawing connectors. Selecting a session SHALL enter it using the `accept_session` flow (which handles the configured `on_enter` action including plugin binds like tmux attach), passing the `post_cmd_file` for post-exit commands. Selecting a workspace root SHALL re-render.

#### Scenario: Render full tree

- **WHEN** Tree view is displayed
- **THEN** system shows workspace roots as top-level nodes, repos as children with branch info, and sessions as nested children with star markers for defaults

#### Scenario: Select session in tree

- **WHEN** user selects a session row in the tree
- **THEN** system runs the `accept_session` flow with `post_cmd_file` passthrough, applying the configured `on_enter` action (cd, tmux attach, or other plugin bind)

### Requirement: Session action loop

When a repo is selected, the browser SHALL enter a session action loop that repeatedly shows the repo's sessions as a tree with box-drawing tree connectors (`├──`, `└──`, `│`) and handles keybind actions until the user selects a session (Enter) or cancels (Escape). The loop re-renders after each action to show updated state. Sessions SHALL be rendered with tree glyphs matching the indentation style used in the Tree view.

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

## ADDED Requirements

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
