# Background PR Daemon

## Purpose

Background daemon process that periodically refreshes PR statuses for all sessions across all repos, removing synchronous `gh` calls from the interactive hot path. Includes lifecycle management (auto-start, PID file, graceful shutdown), per-session GitHub user tracking, and the `ez daemon` CLI surface.

## Requirements

### Requirement: Daemon auto-start on ez invocation
Every `ez` invocation SHALL check whether the PR status daemon is running by reading `~/.config/ez/daemon.pid` and verifying the PID is alive (`kill -0`). If the daemon is not running, `ez` SHALL spawn it in the background and continue without blocking.

#### Scenario: Daemon not running on ez launch
- **WHEN** user runs any `ez` command and `daemon.pid` does not exist or the PID is dead
- **THEN** `ez` spawns a new daemon process in the background, writes `daemon.pid`, and proceeds with the original command without delay

#### Scenario: Daemon already running
- **WHEN** user runs any `ez` command and `daemon.pid` exists with a live PID
- **THEN** `ez` proceeds normally without spawning a second daemon

#### Scenario: Stale PID file
- **WHEN** `daemon.pid` exists but the PID is no longer alive
- **THEN** `ez` removes the stale PID file and spawns a new daemon

### Requirement: Daemon polling loop
The daemon SHALL iterate all registered repos and their sessions, refreshing `ez_pr_status` for every session that has `ez_pr_number` set and whose `ez_pr_status_updated` is older than 5 minutes. The daemon SHALL sleep for 5 minutes between polling cycles.

#### Scenario: Refresh stale PR status
- **WHEN** a session has `ez_pr_number` set and `ez_pr_status_updated` is older than 5 minutes
- **THEN** the daemon runs `gh pr view <ez_pr_url> --json state` and updates `ez_pr_status` and `ez_pr_status_updated`

#### Scenario: Skip fresh PR status
- **WHEN** a session has `ez_pr_number` set but `ez_pr_status_updated` is within the last 5 minutes
- **THEN** the daemon skips that session

#### Scenario: gh CLI not available
- **WHEN** `gh` is not installed or not in PATH
- **THEN** the daemon logs a warning and sleeps until the next cycle

#### Scenario: Prioritize recently accessed sessions
- **WHEN** a polling cycle has more than 20 sessions to refresh
- **THEN** the daemon refreshes sessions in order of most recent `last_accessed`, processing at most 20 per cycle

### Requirement: Daemon CLI surface
The system SHALL provide `ez daemon` subcommands for manual control: `ez daemon start` (start if not running), `ez daemon stop` (send SIGTERM and remove PID file), and `ez daemon status` (print whether daemon is running and its PID).

#### Scenario: Manual daemon start
- **WHEN** user runs `ez daemon start` and daemon is not running
- **THEN** daemon starts and PID file is created

#### Scenario: Manual daemon stop
- **WHEN** user runs `ez daemon stop` and daemon is running
- **THEN** daemon receives SIGTERM, exits gracefully, and PID file is removed

#### Scenario: Daemon status check
- **WHEN** user runs `ez daemon status`
- **THEN** system prints whether the daemon is running, its PID, and the log file location

### Requirement: Daemon logging
The daemon SHALL write operational logs to `~/.config/ez/daemon.log`. The log file SHALL be truncated on daemon startup to prevent unbounded growth.

#### Scenario: Log file created on start
- **WHEN** daemon starts
- **THEN** `~/.config/ez/daemon.log` is created (or truncated if existing) and the daemon writes a startup message with its PID

#### Scenario: Refresh activity logged
- **WHEN** daemon refreshes a PR status
- **THEN** a log entry is written with the repo, session name, PR number, and new status

### Requirement: Per-session gh user tracking
When PR metadata is first populated for a session (via auto-detection or PR checkout), the system SHALL also store `ez_pr_gh_user` in the session env, set to the GitHub username of the currently authenticated `gh` user (obtained via `gh api user --jq .login`).

#### Scenario: gh user captured on PR detection
- **WHEN** `detect_pr_for_session` or PR checkout populates `ez_pr_number` for a session
- **THEN** the system also stores `ez_pr_gh_user` with the current `gh` username

#### Scenario: gh user captured on PR checkout
- **WHEN** a session is created via the PR checkout flow and `ez_pr_number` is set
- **THEN** `ez_pr_gh_user` is stored alongside the other PR env vars

### Requirement: Daemon respects gh user context
The daemon SHALL only refresh PR statuses for sessions whose `ez_pr_gh_user` matches the currently active `gh` user. Sessions with a different `ez_pr_gh_user` SHALL be skipped. Sessions with missing `ez_pr_gh_user` SHALL be refreshed and backfilled.

#### Scenario: Matching gh user
- **WHEN** daemon encounters a session with `ez_pr_gh_user` matching the current `gh` user
- **THEN** daemon refreshes that session's PR status normally

#### Scenario: Mismatched gh user
- **WHEN** daemon encounters a session with `ez_pr_gh_user` that differs from the current `gh` user
- **THEN** daemon skips that session and logs a debug message

#### Scenario: Missing gh user on session
- **WHEN** daemon encounters a session with `ez_pr_number` but no `ez_pr_gh_user`
- **THEN** daemon refreshes it using the currently active `gh` user and backfills `ez_pr_gh_user` with that username

### Requirement: File locking for session data
The daemon SHALL acquire an exclusive file lock (`flock`) on the sessions file before reading and writing session data. The lock SHALL be held only for the duration of the read-modify-write cycle.

#### Scenario: Concurrent daemon and interactive write
- **WHEN** the daemon and an interactive `ez` process both attempt to write `sessions.toml` simultaneously
- **THEN** file locking ensures only one writer proceeds at a time, preventing data corruption
