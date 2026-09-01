## MODIFIED Requirements

### Requirement: Branch name displayed in session picker

The session picker (`session_action_loop`) SHALL display the git branch name for each session entry, positioned after the session name and before labels, PR status, and last-accessed time. The branch name SHALL be formatted as `(branch-name)` with dark green styling.

Branch resolution SHALL use the worktree info cache (built from `git worktree list --porcelain`) instead of spawning `git symbolic-ref --short HEAD` per session. The display format and styling SHALL remain unchanged.

#### Scenario: Session with a worktree path

- **WHEN** a session has a `path` field pointing to a git worktree
- **THEN** the picker resolves the branch via the worktree cache HashMap lookup and displays it as `(branch-name)` in dark green text after the session name

#### Scenario: Session without a path (uses repo root)

- **WHEN** a session has no `path` field (bare session or default session)
- **THEN** the picker resolves the branch from the repo root path via the worktree cache and displays it the same way

#### Scenario: Branch cannot be resolved

- **WHEN** the session's path does not appear in the worktree cache (e.g. path was moved, deleted, or detached HEAD)
- **THEN** no branch indicator is displayed — no placeholder, no error text

### Requirement: Branch name displayed in tree view

The tree view (`browser/views/tree.rs`) SHALL display the git branch name for each session entry using the same format as the session picker. Resolution SHALL use the per-repo worktree cache instead of per-session subprocess calls.

#### Scenario: Tree view session with worktree

- **WHEN** a session in the tree view has a `path` field
- **THEN** the branch is resolved from the repo's worktree cache and displayed as `(branch-name)` in dark green, after the session name

#### Scenario: Tree view session without path

- **WHEN** a session in the tree view has no `path`
- **THEN** the branch is resolved from the repo root's entry in the worktree cache and displayed the same way
