# Non-Git Repos (Delta)

## ADDED Requirements

### Requirement: Non-git directory registration
The system SHALL allow registering directories that do not contain a `.git` folder as tracked repos. The `RepoEntry` model SHALL include an `is_git` boolean flag (defaulting to `true` for existing repos). When `is_git` is `false`, the system SHALL skip all git-specific operations (branch detection, remote parsing, worktree resolution).

#### Scenario: Add non-git directory
- **WHEN** user runs `ez add` inside a directory without `.git`
- **THEN** system registers the directory with `is_git = false` in the repo index

#### Scenario: Add non-git directory by path
- **WHEN** user runs `ez add /path/to/plain-dir` where the path has no `.git`
- **THEN** system registers the directory with `is_git = false`

#### Scenario: Existing git repos unaffected
- **WHEN** user runs `ez add` inside a git repository
- **THEN** system registers it with `is_git = true` (default behavior unchanged)

### Requirement: Plugin skip for non-git repos
The git-worktree plugin SHALL skip all worktree operations (create, delete, rename) for repos where `is_git` is `false`. The `OnSessionCreate` hook SHALL receive the `is_git` flag in the hook context and MUST NOT attempt worktree creation for non-git repos.

#### Scenario: Session create skips worktree for non-git repo
- **WHEN** a session is created under a non-git repo
- **THEN** the git-worktree plugin's `OnSessionCreate` hook returns without creating a worktree

#### Scenario: Session delete skips worktree removal
- **WHEN** a session is deleted under a non-git repo
- **THEN** the git-worktree plugin's `OnSessionDelete` hook returns without removing any worktree

### Requirement: Sessions as directory bookmarks
Sessions under non-git repos SHALL function as simple directory bookmarks. The session's `path` SHALL point to the repo's root directory. Entering such a session SHALL cd into the repo directory without any worktree resolution.

#### Scenario: Enter non-git session
- **WHEN** user enters a session belonging to a non-git repo
- **THEN** system cd's into the repo's root directory

#### Scenario: Session path for non-git repo
- **WHEN** a session is created under a non-git repo
- **THEN** the session's `path` is set to the repo's root directory (no worktree subdirectory)

### Requirement: Browser display for non-git repos
Non-git repos SHALL be visually distinguishable in the browser. They SHALL NOT display branch information. The preview pane SHALL show directory contents instead of git status.

#### Scenario: Non-git repo in workspace view
- **WHEN** a non-git repo is displayed in the Workspace or Repo view
- **THEN** system shows the repo without branch info, with a visual indicator that it is not a git repo

#### Scenario: Preview pane for non-git repo
- **WHEN** user highlights a non-git repo in the browser
- **THEN** preview pane shows directory listing instead of git log/status
