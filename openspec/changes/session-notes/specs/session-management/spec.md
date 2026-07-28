## MODIFIED Requirements

### Requirement: Delete session
The system SHALL delete a session by ID, removing it from `sessions.toml`. If the session has children, it SHALL cascade-delete all descendants. Before deleting, the system SHALL check for uncommitted changes in associated worktrees and prompt for confirmation. The system SHALL run `OnSessionDelete` plugin hooks. The system SHALL also delete the session's notes directory from the data dir if it exists, and for cascade deletes, SHALL clean up notes directories for all deleted descendants.

#### Scenario: Delete leaf session
- **WHEN** user runs `ez session delete feature-auth`
- **THEN** system confirms, runs `OnSessionDelete` hooks, removes the session, and deletes its notes directory if present

#### Scenario: Cascade delete with children
- **WHEN** user deletes a session that has child sessions
- **THEN** system lists the children, prompts for confirmation, deletes the parent and all descendants, and cleans up notes directories for all deleted sessions

#### Scenario: Dirty worktree warning
- **WHEN** a session's worktree has uncommitted changes
- **THEN** system warns about dirty worktrees and requires `--force` or explicit confirmation

#### Scenario: Auto-detect current session for delete
- **WHEN** user runs `ez session delete` without a name
- **THEN** system detects the current session from tmux or worktree directory and prompts
