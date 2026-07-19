# Session Management (Delta)

## MODIFIED Requirements

### Requirement: Rename session
The system SHALL rename a session by ID, updating its name in `sessions.toml` and running `OnSessionRename` plugin hooks. For git-backed sessions, the system SHALL also rename the git branch (`git branch -m <old> <new>`) and move the worktree directory (`git worktree move <old-path> <new-path>`). The session's `path` SHALL be updated to reflect the new worktree location. Optionally, the system SHALL copy Cursor IDE conversations from the old workspace slug to the new one.

#### Scenario: Rename session
- **WHEN** user runs `ez session rename old-name new-name`
- **THEN** system updates the session name and runs `OnSessionRename` hooks

#### Scenario: Rename updates git branch
- **WHEN** user renames a session that has a git worktree
- **THEN** system runs `git branch -m old-name new-name` in the worktree
- **THEN** the branch name in the worktree matches the new session name

#### Scenario: Rename moves worktree directory
- **WHEN** user renames a session that has a worktree at `.ez/repo/old-name`
- **THEN** system runs `git worktree move` to `.ez/repo/new-name`
- **THEN** the session's `path` is updated to the new location

#### Scenario: Rename with Cursor conversation copy
- **WHEN** user renames a session and `copy_cursor_conversations` is enabled
- **THEN** system computes old and new Cursor workspace slugs
- **THEN** copies agent-transcripts and chats directories from old slug to new slug

#### Scenario: Rename bare session
- **WHEN** user renames a bare session
- **THEN** system updates only the session name in metadata (no branch or worktree operations)

#### Scenario: Rename non-git session
- **WHEN** user renames a session under a non-git repo
- **THEN** system updates only the session name (no git branch or worktree operations)

### Requirement: Rename hook protocol
The `OnSessionRename` hook request SHALL include a `rename_context` with `old_name`, `new_name`, `old_path` (optional), and `new_path` (optional). Plugins SHALL use this context to update their own state (e.g. tmux session name, cursor-mcp-auth symlinks).

#### Scenario: Plugin receives rename context
- **WHEN** a session is renamed and `OnSessionRename` hooks fire
- **THEN** each plugin receives `rename_context` with old and new names and paths
