# Session From Dirty (Delta)

## ADDED Requirements

### Requirement: Session from dirty changes via keybind
The browser SHALL support a keybind (default `alt-s`) in the session action loop that creates a new session from the current session's uncommitted changes. The operation SHALL require the current session to have a worktree with unstaged or staged changes.

#### Scenario: Create session from dirty via keybind
- **WHEN** user presses `alt-s` in the session action loop while the current session has dirty changes
- **THEN** system prompts for a new session name
- **THEN** system stashes current changes, creates a new session on the same commit, and pops the stash in the new worktree

#### Scenario: No dirty changes
- **WHEN** user presses `alt-s` but the current session's worktree has no uncommitted changes
- **THEN** system displays an error message "No uncommitted changes to move"

#### Scenario: Keybind not available for bare sessions
- **WHEN** user presses `alt-s` on a bare session (no worktree)
- **THEN** system displays an error message "Cannot create from dirty: session has no worktree"

### Requirement: Session from dirty changes via CLI
The system SHALL provide a CLI command `ez session from-dirty <name>` that creates a new session by moving the current worktree's uncommitted changes to a new session's worktree.

#### Scenario: CLI command creates session from dirty
- **WHEN** user runs `ez session from-dirty hotfix-auth` while in a dirty worktree
- **THEN** system stashes changes, creates session `hotfix-auth` on the same commit, and pops stash in the new worktree

#### Scenario: CLI command outside worktree
- **WHEN** user runs `ez session from-dirty fix` while not in a registered session's worktree
- **THEN** system returns an error "Not in a registered session worktree"

### Requirement: Git stash workflow
The session-from-dirty operation SHALL follow this git workflow:
1. Run `git stash` in the current session's worktree to save uncommitted changes
2. Create a new session (triggering normal `OnSessionCreate` hooks) with the branch starting from the same commit as the current session
3. Run `git stash pop` in the new session's worktree to apply the stashed changes
If any step fails, the system SHALL attempt to restore the original state (pop stash back in the original worktree).

#### Scenario: Successful stash and pop
- **WHEN** the stash/create/pop workflow completes successfully
- **THEN** the original worktree is clean (changes moved to new worktree)
- **THEN** the new worktree contains the previously uncommitted changes

#### Scenario: Stash pop conflict
- **WHEN** `git stash pop` fails in the new worktree (e.g. conflict)
- **THEN** system warns the user that manual conflict resolution is needed in the new worktree
- **THEN** the stash remains in the stash list for manual recovery

#### Scenario: Session creation failure after stash
- **WHEN** session creation fails after the stash was applied
- **THEN** system runs `git stash pop` in the original worktree to restore changes
- **THEN** system reports the error to the user

### Requirement: Same-commit branch base
The new session created by session-from-dirty SHALL have its branch start from the exact same commit as the current session's HEAD. The system SHALL NOT fetch or rebase — the new branch is a direct fork from the current commit.

#### Scenario: Branch starts at same commit
- **WHEN** session-from-dirty creates a new session
- **THEN** the new session's branch HEAD points to the same commit as the source session's HEAD at the time of creation
