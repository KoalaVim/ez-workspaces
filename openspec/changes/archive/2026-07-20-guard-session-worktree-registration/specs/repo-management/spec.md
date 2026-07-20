## MODIFIED Requirements

### Requirement: Auto-register on browse
The system SHALL auto-register a repo when the user navigates to it in the interactive browser and it is not yet registered. Before registering, the system SHALL check whether the path is already tracked as a session worktree under any registered repo. If a matching session is found, the system SHALL skip registration and use the owning repo entry instead.

#### Scenario: Auto-register during drill-down
- **WHEN** user drills into a git repo directory in the workspace browser
- **THEN** system auto-registers the repo and proceeds to the session picker

#### Scenario: Skip registration for session worktree
- **WHEN** user drills into a directory that is already tracked as a session worktree under a registered repo
- **THEN** system does NOT register it as a new repo
- **AND** system enters the session picker for the owning repo instead
