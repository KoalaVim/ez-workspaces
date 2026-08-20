## MODIFIED Requirements

### Requirement: Register existing worktree
The system SHALL register an existing git worktree as a session without running `OnSessionCreate` hooks. It resolves the worktree root and common repo via `git rev-parse`, matches that repo to the registered repo index, and writes a `Session` with `path` set to the existing worktree. If no parent is specified, the system SHALL default `parent_id` to the repo's default (main) session.

The system SHALL also support inline registration from the browser, where the repo entry is already known. In this case, registration SHALL accept a repo ID, worktree path, and optional session name (defaulting to the branch name), create the session record, and return the created session for immediate use (e.g., entering via `on_enter`).

#### Scenario: Register from current directory
- **WHEN** user runs `ez session register` inside a worktree
- **THEN** system detects the worktree root, matches the repo, and creates a session with the current branch name as a child of the default (main) session

#### Scenario: Register with explicit name and parent
- **WHEN** user runs `ez session register --name my-session --parent main`
- **THEN** system creates a session with the given name as a child of `main`

#### Scenario: Register defaults to main parent
- **WHEN** user runs `ez session register --name my-session` without `--parent`
- **THEN** system creates the session as a child of the default (main) session

#### Scenario: Inline registration from browser
- **WHEN** user selects a non-managed worktree in the session picker
- **THEN** system registers it as a session using the worktree path and branch name
- **AND** returns the created session for the caller to enter
