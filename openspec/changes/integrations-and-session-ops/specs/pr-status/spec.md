# PR Status (Delta)

## ADDED Requirements

### Requirement: PR status storage in session env
Sessions associated with a GitHub PR SHALL store PR metadata in `session.env`: `ez_pr_number` (PR number), `ez_pr_url` (full URL), and `ez_pr_status` (one of `open`, `merged`, `closed`). The status SHALL be fetched via `gh pr view <number> --repo <remote> --json state`.

#### Scenario: PR status set on PR checkout
- **WHEN** a session is created via the PR checkout workflow
- **THEN** `ez_pr_status` is set to the PR's current state (e.g. `open`)

#### Scenario: PR status refreshed on enter
- **WHEN** user enters a session that has `ez_pr_number` set and the cached status is older than 5 minutes
- **THEN** system runs `gh pr view` to refresh `ez_pr_status` and persists the update

#### Scenario: gh CLI not available for refresh
- **WHEN** `gh` is not installed during a refresh attempt
- **THEN** system keeps the existing `ez_pr_status` value (no error, just stale data)

### Requirement: PR status display in session picker
The session picker SHALL display a PR status indicator next to sessions that have `ez_pr_number` set. The indicator SHALL be colored: green for `open`, magenta for `merged`, red for `closed`.

#### Scenario: Open PR indicator
- **WHEN** a session has `ez_pr_status = "open"` and `ez_pr_number = "42"`
- **THEN** the session picker displays `[PR #42 open]` in green next to the session name

#### Scenario: Merged PR indicator
- **WHEN** a session has `ez_pr_status = "merged"`
- **THEN** the indicator displays `[PR #42 merged]` in magenta

#### Scenario: No PR number
- **WHEN** a session does not have `ez_pr_number` in its env
- **THEN** no PR indicator is shown

### Requirement: PR status in preview pane
The preview pane SHALL display PR metadata (number, URL, status) for sessions that have PR information in their env.

#### Scenario: Preview shows PR info
- **WHEN** user highlights a session with PR metadata in the browser
- **THEN** preview pane includes PR number, URL, and current status
