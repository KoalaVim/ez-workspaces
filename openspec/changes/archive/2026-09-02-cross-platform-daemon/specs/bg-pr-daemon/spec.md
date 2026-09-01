## MODIFIED Requirements

### Requirement: Daemon auto-start on ez invocation
Every `ez` invocation SHALL check whether the PR status daemon is running by reading `~/.config/ez/daemon.pid` and verifying the PID is alive. If the daemon is not running, `ez` SHALL spawn it in the background and continue without blocking.

#### Scenario: Daemon not running on ez launch
- **WHEN** user runs any `ez` command and `daemon.pid` does not exist or the PID is dead
- **THEN** `ez` spawns a new daemon process in the background, writes `daemon.pid`, and proceeds with the original command without delay

#### Scenario: Daemon already running
- **WHEN** user runs any `ez` command and `daemon.pid` exists with a live PID
- **THEN** `ez` proceeds normally without spawning a second daemon

#### Scenario: Stale PID file
- **WHEN** `daemon.pid` exists but the PID is no longer alive
- **THEN** `ez` removes the stale PID file and spawns a new daemon

### Requirement: Daemon CLI surface
The system SHALL provide `ez daemon` subcommands for manual control: `ez daemon start` (start if not running), `ez daemon stop` (terminate the daemon process and remove PID file), and `ez daemon status` (print whether daemon is running and its PID).

#### Scenario: Manual daemon start
- **WHEN** user runs `ez daemon start` and daemon is not running
- **THEN** daemon starts and PID file is created

#### Scenario: Manual daemon stop
- **WHEN** user runs `ez daemon stop` and daemon is running
- **THEN** daemon is terminated, exits, and PID file is removed

#### Scenario: Daemon status check
- **WHEN** user runs `ez daemon status`
- **THEN** system prints whether the daemon is running, its PID, and the log file location

### Requirement: File locking for session data
The daemon SHALL acquire an exclusive file lock on the sessions file before reading and writing session data. The lock SHALL be held only for the duration of the read-modify-write cycle.

#### Scenario: Concurrent daemon and interactive write
- **WHEN** the daemon and an interactive `ez` process both attempt to write `sessions.toml` simultaneously
- **THEN** file locking ensures only one writer proceeds at a time, preventing data corruption
