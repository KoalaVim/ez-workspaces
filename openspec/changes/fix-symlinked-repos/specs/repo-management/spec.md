## ADDED Requirements

### Requirement: Path normalization
The system SHALL normalize filesystem paths through a single helper before comparing them against registered repo paths. Normalization SHALL resolve symlinks and return the canonical absolute path, and SHALL fall back to the input path unchanged when the path cannot be resolved (it does not exist, is a broken symlink, or is inaccessible). Normalization SHALL never fail.

Registered repo paths SHALL always be canonical. Both the registration path and the lookup path SHALL express their path handling in terms of this helper so the invariant cannot drift.

#### Scenario: Normalize a symlinked path
- **WHEN** the system normalizes `~/workspaces/personal/koala/KoalaVim`, a symlink to `~/.local/share/kvim-envs/main/lazy/KoalaVim`
- **THEN** the result is `/Users/<user>/.local/share/kvim-envs/main/lazy/KoalaVim`

#### Scenario: Normalize a direct path
- **WHEN** the system normalizes a path that contains no symlinks
- **THEN** the result is the same absolute path

#### Scenario: Normalize an unresolvable path
- **WHEN** the system normalizes a path that does not exist on disk
- **THEN** the result is the input path unchanged, and no error is raised

## MODIFIED Requirements

### Requirement: Repo identity
Each repo SHALL have a unique `id` (slug derived from path), a `name` (directory name), an absolute `path`, a `registered_at` timestamp, and an `is_git` boolean flag (default `true`). Per-repo metadata SHALL include optional `remote_url`, `default_branch`, labels, and plugin state. For non-git repos, `remote_url` and `default_branch` SHALL be `None`.

The stored `path` SHALL be canonical: registration resolves symlinks before writing the entry, so `id` and `path` are derived from the resolved target rather than from whatever path the user typed.

Lookup by path SHALL be symlink-aware. The system SHALL normalize the queried path and match a repo whose stored `path` equals either the queried path as given or its normalized form. Matching on the path as given SHALL be retained so that entries whose directory no longer exists on disk remain addressable.

#### Scenario: Slug derivation
- **WHEN** a repo at `~/workspace/personal/my-repo` is registered
- **THEN** the repo gets an id like `personal-my-repo` derived from the workspace-relative path

#### Scenario: Non-git repo identity
- **WHEN** a non-git directory at `~/workspace/notes` is registered
- **THEN** the repo gets `is_git = false`, `remote_url = None`, `default_branch = None`

#### Scenario: Register through a symlink
- **WHEN** the user registers `~/workspaces/personal/koala/KoalaVim`, a symlink to `~/.local/share/kvim-envs/main/lazy/KoalaVim`
- **THEN** the stored `path` is the resolved target, and the `id` is derived from it

#### Scenario: Look up a repo by its symlink path
- **WHEN** the system looks up `~/workspaces/personal/koala/KoalaVim` and the index holds the resolved target
- **THEN** the lookup returns that repo entry

#### Scenario: Look up a repo by its canonical path
- **WHEN** the system looks up the resolved target path directly
- **THEN** the lookup returns the same repo entry

#### Scenario: Look up a repo whose directory no longer exists
- **WHEN** the system looks up a registered path that has since been deleted
- **THEN** the lookup still returns that repo entry by exact path match

#### Scenario: Look up an unregistered path
- **WHEN** the system looks up a path that matches no registered repo, whether symlinked or not
- **THEN** the lookup returns no entry

### Requirement: Auto-register on browse
The system SHALL auto-register a repo when the user navigates to it in the interactive browser and it is not yet registered. Before registering, the system SHALL check whether the path is already tracked as a session worktree under any registered repo. If a matching session is found, the system SHALL skip registration and use the owning repo entry instead.

After registering, the system SHALL resolve the newly created entry successfully even when the browsed path is a symlink. If the entry cannot be resolved, the system SHALL surface a recoverable error rather than panicking.

#### Scenario: Auto-register during drill-down
- **WHEN** user drills into a git repo directory in the workspace browser
- **THEN** system auto-registers the repo and proceeds to the session picker

#### Scenario: Skip registration for session worktree
- **WHEN** user drills into a directory that is already tracked as a session worktree under a registered repo
- **THEN** system does NOT register it as a new repo
- **AND** system enters the session picker for the owning repo instead

#### Scenario: Auto-register a symlinked repo
- **WHEN** user selects an unregistered git repo reached through a symlink
- **THEN** system registers the resolved target, resolves the new entry, and proceeds to the session picker without panicking

#### Scenario: Browse an already-registered symlinked repo
- **WHEN** user selects a repo reached through a symlink whose resolved target is already registered
- **THEN** system enters the session picker for the existing entry
- **AND** system does NOT attempt to register it again or report it as already registered
