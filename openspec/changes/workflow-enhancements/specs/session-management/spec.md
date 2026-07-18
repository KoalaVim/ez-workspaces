# Session Management (Delta)

## MODIFIED Requirements

### Requirement: Create session
The system SHALL create a new session with a unique UUID, user-facing name, and timestamp. If a parent session ID is provided, the new session SHALL be a child of that parent. If no parent is specified, the system SHALL default `parent_id` to the repo's default (main) session, making the new session a child of `main`. On creation, the system SHALL run `OnSessionCreate` plugin hooks in `mutates_session_path`-first order so downstream plugins see the resolved path. After worktree creation, the system SHALL fetch the target branch from the remote (in addition to the base branch) to ensure the local ref is up to date. If the `--bare` flag is set, the system SHALL skip `OnSessionCreate` hooks for plugins that operate on worktrees (e.g. git-worktree) and set `bare = true` on the session.

#### Scenario: Create root-level session
- **WHEN** user runs `ez session new feature-auth`
- **THEN** system creates a session named `feature-auth` with `parent_id` set to the default (main) session's UUID, runs `OnSessionCreate` hooks, and saves to `sessions.toml`

#### Scenario: Create child session
- **WHEN** user runs `ez session new sub-task --parent feature-auth`
- **THEN** system creates a session named `sub-task` with `parent_id` set to `feature-auth`'s UUID

#### Scenario: Duplicate name rejected
- **WHEN** user creates a session with a name that already exists in the same repo
- **THEN** system returns `SessionAlreadyExists` error

#### Scenario: Branch reuse prompt
- **WHEN** the session name matches an existing git branch in the repo
- **THEN** system prompts the user to reuse the existing branch or recreate from the latest base

#### Scenario: Enhanced branch fetch on create
- **WHEN** a session is created and the target branch exists on the remote
- **THEN** system fetches the target branch from origin (e.g. `git fetch origin <branch>`) in addition to the base branch fetch
- **THEN** the worktree is created with the latest remote state of that branch

#### Scenario: Create bare session
- **WHEN** user runs `ez session new my-bookmark --bare`
- **THEN** system creates a session with `bare = true`, no worktree path, and does NOT invoke the git-worktree plugin's `OnSessionCreate` hook

## ADDED Requirements

### Requirement: Session bare flag
The `Session` model SHALL include a `bare` boolean field (default `false`). Bare sessions have no associated worktree path. Plugin hooks that operate on worktrees SHALL check this flag and skip worktree operations when `bare` is `true`.

#### Scenario: Bare flag serialization
- **WHEN** a bare session is saved to `sessions.toml`
- **THEN** the entry includes `bare = true` and the `path` field is absent or null

#### Scenario: Legacy sessions default to non-bare
- **WHEN** `sessions.toml` is loaded with entries lacking the `bare` field
- **THEN** those sessions default to `bare = false`

### Requirement: Last-accessed timestamp on sessions
The `Session` model SHALL include a `last_accessed` timestamp field. This timestamp SHALL be updated to the current time whenever a session is entered. It SHALL default to the session's `created_at` value for new sessions.

#### Scenario: Timestamp updated on enter
- **WHEN** user enters a session
- **THEN** the session's `last_accessed` is updated to the current time and persisted

#### Scenario: Default timestamp on creation
- **WHEN** a new session is created
- **THEN** `last_accessed` is initialized to the session's `created_at` value

### Requirement: Session from dirty changes
The system SHALL support creating a new session by moving the current session's uncommitted changes to a new worktree. The workflow SHALL: (1) run `git stash` in the current worktree, (2) create a new session with a branch starting from the same commit as the current HEAD, (3) run `git stash pop` in the new worktree. If step 3 fails, the system SHALL warn the user. If step 2 fails, the system SHALL restore the stash in the original worktree.

#### Scenario: Successful session from dirty
- **WHEN** user runs `ez session from-dirty hotfix` while in a dirty worktree
- **THEN** system stashes changes, creates session `hotfix` on same commit, pops stash in new worktree
- **THEN** original worktree is clean, new worktree has the uncommitted changes

#### Scenario: No uncommitted changes
- **WHEN** user runs `ez session from-dirty fix` with a clean worktree
- **THEN** system returns an error "No uncommitted changes to move"

#### Scenario: Not in a registered session
- **WHEN** user runs `ez session from-dirty fix` outside a registered session's worktree
- **THEN** system returns an error "Not in a registered session worktree"

#### Scenario: Rollback on creation failure
- **WHEN** session creation fails after stash
- **THEN** system pops the stash back in the original worktree and reports the error
