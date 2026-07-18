# Session Management

## Purpose

Manage worktree-based sessions organized in a tree hierarchy. Sessions are virtual metadata records — plugins give them physical meaning (e.g. git worktrees, tmux sessions). The session system handles the full lifecycle: create, delete, enter, exit, rename, register, and label. Sessions belong to a specific repo and are stored in `~/.config/ez/repos/<id>/sessions.toml`.

## Requirements

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

### Requirement: Multi-stage session name builder
The system SHALL support configurable name stages when creating a session interactively without a pre-supplied name. Before entering the stages, the system SHALL present a mode selection step allowing the user to choose how to build the name (see name-builder-modes capability). If "Build from parts" mode is selected, each stage is either a `choice` (fzf list with `(custom)` and `(none)` sentinels) or `text` (free-text input). After all configured stages, a final free-text prompt is always shown. Parts are joined with `-`; `(none)` stages contribute nothing. Each stage supports Ctrl-P to go back to the previous stage.

#### Scenario: Default stages
- **WHEN** user creates a session interactively without a name and selects "Build from parts" mode
- **THEN** system prompts through stages: prefix (feat/fix/chore), ticket-prefix (custom), ticket-number (text), then final description
- **THEN** resulting name is the non-empty parts joined by `-` (e.g. `feat-ABC-123-add-dark-mode`)

#### Scenario: Skip stages
- **WHEN** user selects `(none)` for a stage
- **THEN** that stage contributes nothing to the name

#### Scenario: Back navigation
- **WHEN** user presses Ctrl-P during a stage
- **THEN** system returns to the previous stage with prior context preserved

#### Scenario: Name provided on CLI
- **WHEN** user passes a name argument (e.g. `ez session new my-feature`) without `--interactive`
- **THEN** the stage builder is skipped entirely

#### Scenario: Mode selection before stages
- **WHEN** user creates a session interactively without a name
- **THEN** system presents the mode selection step first, then dispatches to the chosen mode handler

### Requirement: Delete session
The system SHALL delete a session by ID, removing it from `sessions.toml`. If the session has children, it SHALL cascade-delete all descendants. Before deleting, the system SHALL check for uncommitted changes in associated worktrees and prompt for confirmation. The system SHALL run `OnSessionDelete` plugin hooks.

#### Scenario: Delete leaf session
- **WHEN** user runs `ez session delete feature-auth`
- **THEN** system confirms, runs `OnSessionDelete` hooks, and removes the session

#### Scenario: Cascade delete with children
- **WHEN** user deletes a session that has child sessions
- **THEN** system lists the children, prompts for confirmation, and deletes the parent and all descendants

#### Scenario: Dirty worktree warning
- **WHEN** a session's worktree has uncommitted changes
- **THEN** system warns about dirty worktrees and requires `--force` or explicit confirmation

#### Scenario: Auto-detect current session for delete
- **WHEN** user runs `ez session delete` without a name
- **THEN** system detects the current session from tmux or worktree directory and prompts

### Requirement: Enter session
The system SHALL enter a session by applying the `on_enter` action. The default action is `cd` (write the session's worktree path to the cd-file). The action can be overridden to a plugin-bind name (e.g. `tmux`), which runs that bind's `OnBind` hook. If the plugin bind produces no navigation effect, the system SHALL fall back to `cd`.

#### Scenario: Default cd enter
- **WHEN** user enters a session with `on_enter = "cd"`
- **THEN** system writes the session's path to the cd-file for the shell wrapper to cd into

#### Scenario: Plugin bind enter
- **WHEN** user enters a session with `on_enter = "tmux"`
- **THEN** system finds the matching session-context plugin bind and runs its `OnBind` hook
- **THEN** if the hook returns `cd_target` or `post_shell_commands`, those are applied

#### Scenario: Plugin bind fallback
- **WHEN** the plugin bind produces no navigation effect or fails
- **THEN** system falls back to plain `cd` into the session path

### Requirement: Exit session
The system SHALL exit the current session by running `OnSessionExit` plugin hooks.

#### Scenario: Exit current session
- **WHEN** user runs `ez session exit`
- **THEN** system runs `OnSessionExit` hooks for the current session

### Requirement: Rename session
The system SHALL rename a session by ID, updating its name in `sessions.toml` and running `OnSessionRename` plugin hooks.

#### Scenario: Rename session
- **WHEN** user runs `ez session rename old-name new-name`
- **THEN** system updates the session name and runs `OnSessionRename` hooks

### Requirement: Register existing worktree
The system SHALL register an existing git worktree as a session without running `OnSessionCreate` hooks. It resolves the worktree root and common repo via `git rev-parse`, matches that repo to the registered repo index, and writes a `Session` with `path` set to the existing worktree. If no parent is specified, the system SHALL default `parent_id` to the repo's default (main) session.

#### Scenario: Register from current directory
- **WHEN** user runs `ez session register` inside a worktree
- **THEN** system detects the worktree root, matches the repo, and creates a session with the current branch name as a child of the default (main) session

#### Scenario: Register with explicit name and parent
- **WHEN** user runs `ez session register --name my-session --parent main`
- **THEN** system creates a session with the given name as a child of `main`

#### Scenario: Register defaults to main parent
- **WHEN** user runs `ez session register --name my-session` without `--parent`
- **THEN** system creates the session as a child of the default (main) session

### Requirement: Session tree hierarchy
Sessions SHALL be organized in a tree using `parent_id` pointers. The system SHALL support operations: list roots, find children, find ancestors, find descendants, and render as an indented tree.

#### Scenario: Render tree
- **WHEN** user runs `ez session list`
- **THEN** system renders sessions as an indented tree with root sessions at the top level and children indented below their parents

#### Scenario: Flat list
- **WHEN** user runs `ez session list --flat`
- **THEN** system renders sessions as a flat list without tree structure

### Requirement: Default session
The system SHALL auto-create a `main` session (marked `is_default = true`) when a repo's session list is empty. This ensures every repo always has at least one session.

#### Scenario: Auto-create on first access
- **WHEN** user browses a repo with no sessions
- **THEN** system creates a `main` session marked as default and presents it in the picker

### Requirement: Current session detection
The system SHALL detect the current session from: (1) the tmux `@ez_session_path` user option on the current tmux session, or (2) matching the current working directory against known session worktree paths.

#### Scenario: Detect from tmux
- **WHEN** user is inside a tmux session managed by ez
- **THEN** system reads `@ez_session_path` to identify the current session

#### Scenario: Detect from worktree path
- **WHEN** user is inside a directory that matches a registered session's worktree path
- **THEN** system identifies the current session by path matching

### Requirement: Session labels
Sessions SHALL support arbitrary string labels for grouping and filtering. Labels can be added, removed, and listed via CLI commands.

#### Scenario: Add labels
- **WHEN** user runs `ez session label add feature-x wip urgent`
- **THEN** system adds the labels `wip` and `urgent` to session `feature-x`

#### Scenario: List labels grouped
- **WHEN** user runs `ez session label list` without a session name
- **THEN** system lists all sessions grouped by their labels

### Requirement: Post-create action
The system SHALL support a configurable `on_create` action that runs immediately after a session is created interactively. The default is `none` (do nothing). It can be set to `cd` or a plugin-bind name (e.g. `tmux`).

#### Scenario: Auto-attach after create
- **WHEN** `on_create = "tmux"` and user creates a session in the browser
- **THEN** system runs the tmux bind's hook after creation, attaching to the new tmux session

### Requirement: Interactive builder flag
The system SHALL support a `--interactive` / `-i` flag on `ez session new` that forces the multi-stage name builder (starting from mode selection) even when a name is provided on the CLI. This allows users to use the interactive builder while passing other flags like `--parent`.

#### Scenario: Force interactive with name provided
- **WHEN** user runs `ez session new my-feature --interactive`
- **THEN** system ignores the provided name and enters the interactive mode selection and name builder flow

#### Scenario: Force interactive without name
- **WHEN** user runs `ez session new -i`
- **THEN** system enters the interactive mode selection and name builder flow (same as omitting the name)

#### Scenario: Short flag
- **WHEN** user runs `ez session new -i --parent feature-auth`
- **THEN** system enters interactive builder flow and creates the session as a child of `feature-auth`

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
