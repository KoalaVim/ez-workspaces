# Bare Sessions (Delta)

## ADDED Requirements

### Requirement: Bare session creation via keybind
The browser SHALL support an `Alt-Shift-N` keybind in the session action loop that creates a bare session. A bare session SHALL have `bare = true` in its metadata and SHALL NOT trigger the git-worktree plugin's `OnSessionCreate` hook. The session SHALL have no worktree path.

#### Scenario: Create bare session via keybind
- **WHEN** user presses `Alt-Shift-N` in the session action loop
- **THEN** system prompts for a session name (using the name builder)
- **THEN** system creates a session with `bare = true` and no worktree path
- **THEN** the git-worktree plugin's `OnSessionCreate` hook is NOT invoked

#### Scenario: Bare session displayed in tree
- **WHEN** a bare session exists in a repo's session tree
- **THEN** it is displayed with a visual indicator (e.g. dimmed or marked `[bare]`) distinguishing it from worktree sessions

### Requirement: Bare session creation via CLI flag
The `ez session new` command SHALL support a `--bare` flag that creates a session without triggering the git-worktree plugin. The resulting session SHALL have `bare = true` and no worktree path.

#### Scenario: Create bare session via CLI
- **WHEN** user runs `ez session new my-bookmark --bare`
- **THEN** system creates a session named `my-bookmark` with `bare = true`
- **THEN** the git-worktree plugin's `OnSessionCreate` hook is NOT invoked
- **THEN** the session has no associated worktree path

#### Scenario: Bare flag combined with parent
- **WHEN** user runs `ez session new child-bookmark --bare --parent feature-x`
- **THEN** system creates a bare session as a child of `feature-x`

### Requirement: Bare session model flag
The `Session` model SHALL include a `bare` boolean field (default `false`). When `bare` is `true`, the session SHALL have no `path` (or path is `None`). Plugin hooks that operate on worktrees SHALL check this flag and skip worktree operations.

#### Scenario: Session serialization with bare flag
- **WHEN** a bare session is saved to `sessions.toml`
- **THEN** the entry includes `bare = true` and omits the `path` field

#### Scenario: Existing sessions default to non-bare
- **WHEN** `sessions.toml` is loaded with entries that lack the `bare` field
- **THEN** those sessions default to `bare = false`

### Requirement: Entering a bare session
Entering a bare session SHALL NOT attempt to cd into a worktree path. If `on_enter` is `"cd"`, the system SHALL display a message indicating the session has no worktree. Plugin-bind enter actions (e.g. tmux) SHALL still be invoked for bare sessions.

#### Scenario: Cd enter on bare session
- **WHEN** user enters a bare session with `on_enter = "cd"`
- **THEN** system displays a message "Session has no worktree path" and does not cd

#### Scenario: Plugin bind enter on bare session
- **WHEN** user enters a bare session with `on_enter = "tmux"`
- **THEN** system invokes the tmux plugin bind's `OnBind` hook (plugin decides behavior)
