## MODIFIED Requirements

### Requirement: Branch name displayed in session picker

The session picker (`session_action_loop`) SHALL display the git branch name for each session entry, positioned after the session name and before labels, PR status, and last-accessed time. The branch name SHALL be formatted as `(branch-name)` with dark green styling. During an active rebase, the branch name SHALL still be displayed, recovered from git's rebase state files.

#### Scenario: Session with a worktree path

- **WHEN** a session has a `path` field pointing to a git worktree
- **THEN** the picker resolves the branch via `WorktreeInfo` (batch) or `BranchCache` (fallback) and displays it as `(branch-name)` in dark green text after the session name

#### Scenario: Session without a path (uses repo root)

- **WHEN** a session has no `path` field (bare session or default session)
- **THEN** the picker resolves the branch from the repo root path and displays it the same way

#### Scenario: Branch cannot be resolved

- **WHEN** the branch cannot be resolved (not a rebase — e.g., `git checkout --detach`, missing path, non-git directory)
- **THEN** no branch indicator is displayed — no placeholder, no error text

#### Scenario: Session mid-rebase

- **WHEN** a session's worktree is mid-rebase (HEAD detached due to rebase)
- **THEN** the branch name is recovered from rebase state files and displayed as `(branch-name|REBASE)` in dark green text

### Requirement: Shared branch formatting helper

A `format_branch_indicator()` function SHALL exist in `browser/mod.rs` that takes an optional branch name and returns the formatted display string. Both the session picker and tree view SHALL use this function.

#### Scenario: Branch name provided

- **WHEN** called with `Some("feature-x")`
- **THEN** returns `" (feature-x)"` with dark green styling applied

#### Scenario: No branch name

- **WHEN** called with `None`
- **THEN** returns an empty string

#### Scenario: Branch name with rebase suffix

- **WHEN** called with `Some("feature-x|REBASE")`
- **THEN** returns `" (feature-x|REBASE)"` with dark green styling applied
