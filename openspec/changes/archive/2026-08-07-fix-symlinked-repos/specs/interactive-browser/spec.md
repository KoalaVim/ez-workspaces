## ADDED Requirements

### Requirement: Symlinked repos in scan-driven views
Views that discover repos by scanning the filesystem — Tree, Workspace drill-down, and the preview pane they feed — SHALL resolve a scanned directory to its registered repo entry even when that directory is a symlink to the registered location. Such a repo SHALL render exactly as a directly-reached one: as registered, with its session tree, labels, and repo metadata.

These views SHALL display the path as scanned, not the resolved target. The symlink is resolved for lookup only; presentation is unchanged.

#### Scenario: Sessions render under a symlinked repo in the Tree view
- **WHEN** a workspace root contains a symlink to a registered repo and the Tree view is displayed
- **THEN** the repo's sessions render as nested children, with star markers for defaults, identically to a directly-reached repo

#### Scenario: Select a session under a symlinked repo
- **WHEN** user selects a session row nested under a symlinked repo in the Tree view
- **THEN** system runs the `accept_session` flow for that session, entering the session's own worktree path

#### Scenario: Preview a symlinked repo
- **WHEN** user highlights a repo reached through a symlink
- **THEN** the preview pane shows the Sessions tree and Repo Labels
- **AND** does NOT show `(unregistered — select to register)`

#### Scenario: Preview a session under a symlinked repo
- **WHEN** user highlights a session in the session action loop for a repo reached through a symlink
- **THEN** the preview renders the session-specific preview
- **AND** does NOT show `Repo not registered`

#### Scenario: Display path is the scanned path
- **WHEN** a repo reached through a symlink is rendered in the Tree view or its preview
- **THEN** the path shown is the symlink path the user navigated through, not the resolved target

#### Scenario: Labels render for a symlinked repo during drill-down
- **WHEN** user drills into a directory containing a symlink to a registered, labelled repo
- **THEN** the entry renders with its branch and its labels
