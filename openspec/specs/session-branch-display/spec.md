## Purpose

Display the git branch name alongside session entries in the interactive browser's session picker and tree view, so users can see which branch each session points to without leaving the picker.

## Requirements

### Requirement: Branch name displayed in session picker

The session picker (`session_action_loop`) SHALL display the git branch name for each session entry, positioned after the session name and before labels, PR status, and last-accessed time. The branch name SHALL be formatted as `(branch-name)` with dark green styling.

#### Scenario: Session with a worktree path

- **WHEN** a session has a `path` field pointing to a git worktree
- **THEN** the picker resolves the branch via `git symbolic-ref --short HEAD` on that path and displays it as `(branch-name)` in dark green text after the session name

#### Scenario: Session without a path (uses repo root)

- **WHEN** a session has no `path` field (bare session or default session)
- **THEN** the picker resolves the branch from the repo root path and displays it the same way

#### Scenario: Branch cannot be resolved

- **WHEN** `git symbolic-ref --short HEAD` fails (detached HEAD, missing path, non-git directory)
- **THEN** no branch indicator is displayed — no placeholder, no error text

### Requirement: Branch name displayed in tree view

The tree view (`browser/views/tree.rs`) SHALL display the git branch name for each session entry using the same format and resolution logic as the session picker.

#### Scenario: Tree view session with worktree

- **WHEN** a session in the tree view has a `path` field
- **THEN** the branch is resolved from that path and displayed as `(branch-name)` in dark green, after the session name

#### Scenario: Tree view session without path

- **WHEN** a session in the tree view has no `path`
- **THEN** the branch is resolved from the repo root and displayed the same way

### Requirement: Shared branch formatting helper

A `format_branch_indicator()` function SHALL exist in `browser/mod.rs` that takes an optional branch name and returns the formatted display string. Both the session picker and tree view SHALL use this function.

#### Scenario: Branch name provided

- **WHEN** called with `Some("feature-x")`
- **THEN** returns `" (feature-x)"` with dark green styling applied

#### Scenario: No branch name

- **WHEN** called with `None`
- **THEN** returns an empty string
