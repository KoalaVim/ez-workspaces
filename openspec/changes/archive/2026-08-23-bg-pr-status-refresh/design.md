## Context

PR status is currently refreshed synchronously in two places:
1. `enter_session` (`src/session/mod.rs:1076-1079`) — calls `detect_pr_for_session` then `refresh_pr_status` inline, blocking until `gh pr view` returns
2. `update_last_accessed` (`src/browser/mod.rs:954`) — calls `detect_pr_for_session` on browser selection

The `refresh_pr_status` function checks a 5-minute staleness window via `ez_pr_status_updated` and runs `gh pr view <url> --json state`. This blocks the interactive path for ~1-2 seconds per session enter.

Sessions store PR metadata as env vars: `ez_pr_number`, `ez_pr_url`, `ez_pr_status`, `ez_pr_status_updated`. Session data lives in per-repo TOML files at `~/.config/ez/repos/<id>/sessions.toml`.

Users may have multiple GitHub accounts (personal, work) authenticated via `gh auth`. The daemon needs to know which account was used to fetch each session's PR data so it can switch contexts when refreshing.

## Goals / Non-Goals

**Goals:**
- Move PR status polling out of the interactive hot path
- Keep PR statuses fresh across all repos/sessions without user interaction
- Auto-start the daemon transparently — no manual setup required
- Track which `gh` user fetched each session's PR data and reuse that identity on refresh

**Non-Goals:**
- Real-time PR status updates (webhook-based) — polling on an interval is sufficient
- Daemon management UI — `ez daemon status/stop` is enough for debugging
- PR auto-detection in the daemon — first-time detection stays in the interactive path because the user expects immediate feedback when entering a new-PR session
- Configurable polling interval via config file (hardcode ~5 min for now, can be made configurable later)

## Decisions

### 1. Daemon lifecycle: PID file with liveness check

The daemon writes its PID to `~/.config/ez/daemon.pid` on startup. Every `ez` invocation reads this file and checks if the PID is alive (`kill -0`). If the process is gone, the stale PID file is cleaned up and a new daemon is spawned.

**Alternatives considered:**
- Unix socket for IPC: More robust but adds complexity for a process that doesn't need to receive commands at runtime. The daemon just polls and writes files.
- Systemd/launchd service: Would require platform-specific installation steps and breaks the "just works" promise.

### 2. Daemon is a forked `ez` process, not a separate binary

`ez daemon start` (also triggered automatically) forks the current `ez` binary with `daemon run` args, detaches stdio, and exits. The child runs the polling loop. This avoids needing a separate binary or install step.

The daemon double-forks (or uses `setsid` on Linux / direct fork on macOS) to fully detach from the parent terminal session.

### 3. Per-session `gh` user tracking via `ez_pr_gh_user`

When PR metadata is first populated (via `detect_pr_for_session` or `resolve_pr_via_gh`), we also run `gh auth status --json` to capture the active GitHub username and store it as `ez_pr_gh_user` in the session env. The daemon reads this value before refreshing and, if it differs from the currently active `gh` user, passes `--hostname` or skips that session (depending on whether `gh auth switch` is feasible in a non-interactive context).

**Approach:** Run `gh api user --jq .login` at detection time to get the username. At refresh time, compare against current `gh api user --jq .login`. If they differ, skip the refresh for that session. If `ez_pr_gh_user` is missing (legacy session from before this change), refresh it anyway using the current `gh` user and backfill `ez_pr_gh_user` — this avoids leaving existing sessions permanently stale.

### 4. Daemon iterates all repos and sessions

The daemon loads the repo index (`~/.config/ez/repos/index.toml`), iterates each repo's `sessions.toml`, and refreshes every session that has `ez_pr_number` set and whose `ez_pr_status_updated` is older than the polling interval. This reuses existing `repo::store` and `session::store` functions.

### 5. Logging to a daemon-specific log file

The daemon writes logs to `~/.config/ez/daemon.log` (rotating or truncating on startup). This gives users a way to debug issues without requiring `--debug` on an interactive session.

## Risks / Trade-offs

- **File contention**: The daemon writes `sessions.toml` while an interactive `ez` process might also be writing it. → Mitigation: Use file-level locking (`flock`) around reads and writes to `sessions.toml`. The lock is held briefly (just the read-modify-write cycle).
- **gh rate limits**: Polling many PRs frequently could hit GitHub API rate limits. → Mitigation: The 5-minute interval with staleness checks means each PR is queried at most ~12 times/hour. For users with many sessions, add a per-cycle cap (e.g., refresh at most 20 PRs per cycle, prioritizing most-recently-accessed sessions).
- **Orphaned daemon**: If the daemon crashes or hangs, the PID file stays. → Mitigation: PID liveness check on every `ez` invocation cleans up stale PID files automatically.
- **Multiple `gh` users**: The daemon can only refresh PRs for the currently authenticated `gh` user. → Mitigation: Track `ez_pr_gh_user` per session, skip mismatched sessions. The interactive path still refreshes inline with the correct auth context.
