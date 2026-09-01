## MODIFIED Requirements

### Requirement: Detect non-managed worktrees
The system SHALL detect git worktrees that exist on disk but are not tracked as ez sessions. Detection SHALL consume the already-parsed `git worktree list --porcelain` output from the worktree info cache (shared with branch resolution) instead of running `git worktree list --porcelain` independently. The parsed worktree paths and branch names SHALL be filtered by subtracting the main repo path and all managed session paths. The result SHALL be a list of unmanaged worktrees with their path and branch name (or short SHA for detached HEAD). Worktrees whose path does not exist on disk (prunable) SHALL be excluded.

#### Scenario: Repo with unmanaged worktrees
- **WHEN** a repo has 3 git worktrees and 1 is tracked as an ez session
- **THEN** detection returns 1 unmanaged worktree (the main repo worktree is also excluded)
- **AND** no additional `git worktree list` subprocess is spawned (reuses the cache)

#### Scenario: All worktrees managed
- **WHEN** every git worktree (except the main repo) is tracked as a session
- **THEN** detection returns an empty list

#### Scenario: Detached HEAD worktree
- **WHEN** a non-managed worktree has a detached HEAD
- **THEN** detection returns it with the short commit SHA instead of a branch name

#### Scenario: Non-git repo
- **WHEN** the repo is not a git repo (`is_git = false`)
- **THEN** detection returns an empty list without running git commands

#### Scenario: Prunable worktree excluded
- **WHEN** `git worktree list` includes a worktree whose path no longer exists on disk
- **THEN** that worktree is excluded from the results
