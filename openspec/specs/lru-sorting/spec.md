# LRU Sorting

## Purpose

Provide last-recently-used (LRU) sorting for repos and sessions in the interactive browser. This allows frequently accessed items to float to the top, reducing navigation time for active projects. The sort order is toggleable via keybind and configurable as the default.

## Requirements

### Requirement: Last-accessed timestamp on sessions
The `Session` model SHALL include a `last_accessed` timestamp field. This timestamp SHALL be updated to the current time whenever a session is entered (via the `enter` action). The field SHALL default to the session's `created_at` value for sessions that have never been entered.

#### Scenario: Timestamp updated on enter
- **WHEN** user enters a session
- **THEN** the session's `last_accessed` timestamp is updated to the current time and persisted to `sessions.toml`

#### Scenario: New session default timestamp
- **WHEN** a new session is created
- **THEN** `last_accessed` is initialized to the session's creation time

### Requirement: Last-accessed timestamp on repos
The `RepoMeta` model SHALL include a `last_accessed` timestamp field. This timestamp SHALL be updated to the current time whenever the user browses into a repo (enters the session picker for that repo). The field SHALL default to `registered_at` for repos that have never been browsed.

#### Scenario: Timestamp updated on browse
- **WHEN** user selects a repo in the browser and enters its session picker
- **THEN** the repo's `last_accessed` timestamp is updated to the current time

#### Scenario: New repo default timestamp
- **WHEN** a repo is first registered
- **THEN** `last_accessed` is initialized to `registered_at`

### Requirement: Sort toggle keybind
The browser SHALL support a toggle keybind (default `ctrl-s`) that switches the current view's sort order between alphabetical and LRU (most recently accessed first). The toggle SHALL apply to all views that list repos or sessions (Repo view, Workspace view, Owner view, Label view, session picker). The current sort mode SHALL be indicated in the fzf header.

#### Scenario: Toggle from alpha to LRU
- **WHEN** user presses `ctrl-s` while in alphabetical sort mode
- **THEN** the view re-renders with items sorted by `last_accessed` descending (most recent first)
- **THEN** the header indicates "Sort: LRU"

#### Scenario: Toggle from LRU to alpha
- **WHEN** user presses `ctrl-s` while in LRU sort mode
- **THEN** the view re-renders with items sorted alphabetically by name
- **THEN** the header indicates "Sort: A-Z"

#### Scenario: Sort persists across view switches
- **WHEN** user toggles to LRU sort and then switches to another view
- **THEN** the new view also uses LRU sort order

### Requirement: Default sort configuration
The config SHALL support a `default_sort` field with values `"alpha"` or `"lru"` (default `"alpha"`). This determines the initial sort order when the browser is launched.

#### Scenario: Default alpha sort
- **WHEN** config has `default_sort = "alpha"` or the field is absent
- **THEN** browser starts with alphabetical sorting

#### Scenario: Default LRU sort
- **WHEN** config has `default_sort = "lru"`
- **THEN** browser starts with LRU sorting (most recently accessed first)
