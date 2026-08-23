## ADDED Requirements

### Requirement: OnAttachedSessions plugin hook
The system SHALL support an `OnAttachedSessions` plugin hook that allows plugins to report which sessions are currently attached. The hook request SHALL include repo info and the full session list (IDs, names, and paths). The hook response SHALL include an `attached_sessions` field containing a list of session IDs the plugin considers attached. The system SHALL call this hook across all enabled plugins and union the results into a single set of attached session IDs. The hook SHALL be called once per render cycle, not per session.

#### Scenario: Plugin reports attached sessions
- **WHEN** the `OnAttachedSessions` hook is called on an enabled plugin
- **THEN** the plugin queries its multiplexer state, matches against the session list from the request, and returns matching session IDs in `attached_sessions`

#### Scenario: Multiple plugins report attached sessions
- **WHEN** multiple plugins handle `OnAttachedSessions` and each returns different session IDs
- **THEN** the system unions all reported IDs into a single `HashSet<SessionId>`

#### Scenario: Plugin multiplexer not installed
- **WHEN** a plugin handles `OnAttachedSessions` but its multiplexer command is not installed or returns an error
- **THEN** the plugin SHALL return `attached_sessions: []` (empty) with `success: true`

#### Scenario: No plugins handle OnAttachedSessions
- **WHEN** no enabled plugins declare `on_attached_sessions` in their manifest hooks
- **THEN** the attached set SHALL be empty and all sessions render in default color

### Requirement: Render attached sessions in aqua
The system SHALL render the session name in aqua/cyan color when the session is detected as attached. Non-attached sessions SHALL continue to render in bold yellow. This applies to both the interactive browser session list and the CLI `ez session list` output (tree and flat modes).

#### Scenario: Attached session in interactive browser
- **WHEN** a session is attached via any multiplexer
- **THEN** its name in the fzf session list SHALL be rendered in cyan/aqua instead of yellow

#### Scenario: Non-attached session color unchanged
- **WHEN** a session is not attached via any multiplexer
- **THEN** its name SHALL render in bold yellow as before

#### Scenario: Attached session in CLI list
- **WHEN** user runs `ez session list` and a session is attached
- **THEN** its name SHALL be rendered in cyan/aqua in both tree and flat output modes
