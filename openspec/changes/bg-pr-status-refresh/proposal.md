## Why

PR status refresh currently happens synchronously during `enter_session` and browser selection, blocking the user while `gh pr view` runs. This means statuses can sit stale for long periods between interactions, and the user pays a latency cost every time they enter a session with a PR. A background daemon that periodically refreshes all PR statuses would keep data fresh without blocking any interactive command.

## What Changes

- Add a new `ez daemon` subcommand that runs a background process polling all sessions with `ez_pr_number` set, refreshing their `ez_pr_status` via `gh pr view` on a configurable interval (default ~5 minutes)
- On every `ez` invocation (browser launch, session enter, etc.), check if the daemon is running; if not, spawn it automatically in the background
- Use a PID file (`~/.config/ez/daemon.pid`) for liveness detection
- Remove the synchronous `refresh_pr_status` call from `enter_session` — the daemon handles it now
- Keep the synchronous `detect_pr_for_session` in `enter_session` (first-time detection still needs to happen inline so the user sees it immediately)
- Store the GitHub user (`ez_pr_gh_user`) that was used to fetch PR data for each session, so the daemon can re-use the correct identity when refreshing (supports users with multiple `gh` auth contexts)

## Capabilities

### New Capabilities

- `bg-pr-daemon`: Background daemon process that periodically refreshes PR statuses for all sessions across all repos. Includes lifecycle management (auto-start, PID file, graceful shutdown) and the `ez daemon` CLI surface.

### Modified Capabilities

- `pr-status`: The "PR status refreshed on enter" requirement changes — refresh is no longer triggered synchronously on session enter but instead handled by the background daemon. The 5-minute staleness window is replaced by the daemon's polling interval.

## Impact

- `src/main.rs` — add daemon liveness check on startup, add `Command::Daemon` variant
- `src/cli.rs` — add `daemon` subcommand (`start`, `stop`, `status`)
- `src/session/mod.rs` — remove synchronous `refresh_pr_status` call from `enter_session`
- New module `src/daemon/` — daemon loop, PID file management, repo/session iteration
- `src/paths.rs` — add `daemon_pid_file()` helper
- Dependencies: no new external crates needed (uses `std::process::Command` for daemonization, existing `gh` CLI for PR queries)
