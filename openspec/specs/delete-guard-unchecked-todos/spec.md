# Delete Guard: Unchecked TODOs

## Purpose

Prevent accidental session deletion when the session's notes contain unchecked markdown todo items, ensuring outstanding work is not lost without explicit acknowledgment.

## Requirements

### Requirement: Pre-delete guard for unchecked todos
The system SHALL check a session's notes README.md for unchecked markdown todo items (`- [ ]`) before allowing deletion. If unchecked todos are found, the system SHALL block deletion and report the outstanding items unless `--force` is passed or the user confirms interactively. The check SHALL apply to all sessions in the delete set (including descendants in a cascade delete).

#### Scenario: Delete blocked by unchecked todos
- **WHEN** user runs `ez session delete my-session` and the session's notes README.md contains `- [ ] push final review`
- **THEN** system reports the unchecked todo items and returns an error
- **AND** the session is NOT deleted

#### Scenario: Delete proceeds with all todos checked
- **WHEN** user runs `ez session delete my-session` and the session's notes only contain checked todos (`- [x] done`)
- **THEN** system proceeds with deletion normally (no guard triggered)

#### Scenario: Force bypasses the guard
- **WHEN** user runs `ez session delete my-session --force` and notes contain unchecked todos
- **THEN** system skips the unchecked-todos check and deletes the session

#### Scenario: No notes directory
- **WHEN** user deletes a session that has no notes directory
- **THEN** the guard is a no-op (no error, deletion proceeds)

#### Scenario: Empty notes README
- **WHEN** user deletes a session whose notes README.md exists but is empty
- **THEN** the guard is a no-op (no unchecked todos found, deletion proceeds)

#### Scenario: Cascade delete checks descendants
- **WHEN** user deletes a session with `--force` (to allow cascade) and a descendant session has unchecked todos in its notes
- **THEN** system reports which descendant sessions have unchecked todos and blocks deletion unless force is used

#### Scenario: Browser delete shows warning
- **WHEN** user triggers delete from the interactive browser and the session has unchecked todos
- **THEN** system displays the unchecked todos as a warning (same as dirty worktree warning)
- **AND** the user can confirm to proceed or cancel

#### Scenario: Multiple unchecked todos reported
- **WHEN** a session has multiple unchecked todos (e.g. 3 items)
- **THEN** the error message lists up to 5 unchecked todo lines to show what's outstanding

#### Scenario: Detection pattern
- **WHEN** a notes README.md contains lines matching the pattern: optional leading whitespace followed by `- [ ]` (dash, space, open bracket, space, close bracket)
- **THEN** those lines are detected as unchecked todos
- **AND** lines with `- [x]` or `- [X]` are NOT detected (they are checked)
