## 1. Daemon Module Foundation

- [x] 1.1 Create `src/daemon/mod.rs` with module structure and `dispatch` function for the `Daemon` CLI subcommand
- [x] 1.2 Add `daemon_pid_file()` and `daemon_log_file()` helpers to `src/paths.rs`
- [x] 1.3 Add `Command::Daemon` variant to `src/cli.rs` with subcommands: `start`, `stop`, `status`, and internal `run` (the actual loop, not user-facing)
- [x] 1.4 Wire `Command::Daemon` into `main.rs` dispatch

## 2. PID File & Lifecycle Management

- [x] 2.1 Implement `write_pid_file()` and `read_pid_file()` in `src/daemon/mod.rs`
- [x] 2.2 Implement `is_daemon_alive()` — read PID file, `kill(pid, 0)` check, clean up stale file
- [x] 2.3 Implement `spawn_daemon()` — fork the current binary with `daemon run`, detach stdio, write PID file
- [x] 2.4 Implement `stop_daemon()` — read PID, send SIGTERM, remove PID file
- [x] 2.5 Implement `daemon_status()` — print running/stopped, PID, log file path

## 3. Daemon Polling Loop

- [x] 3.1 Implement `daemon_run()` — the main loop: set up logging, write PID, loop with 5-minute sleep
- [x] 3.2 Implement `refresh_all_sessions()` — load repo index, iterate repos, load sessions, collect refresh candidates
- [x] 3.3 Sort candidates by `last_accessed` descending, cap at 20 per cycle
- [x] 3.4 Reuse existing `refresh_pr_status` logic (extract from `src/session/mod.rs` into shared helper) for each candidate
- [x] 3.5 Add `flock`-based file locking around `load_sessions`/`save_sessions` calls in the daemon path

## 4. gh User Tracking

- [x] 4.1 Add `get_current_gh_user()` helper — runs `gh api user --jq .login` and returns `Option<String>`
- [x] 4.2 Update `detect_pr_for_session` to store `ez_pr_gh_user` alongside other PR env vars
- [x] 4.3 Update `PrMetadata::to_env()` in `name_builder.rs` to include `ez_pr_gh_user`
- [x] 4.4 In the daemon polling loop, call `get_current_gh_user()` once per cycle and skip sessions where `ez_pr_gh_user` doesn't match

## 5. Auto-Start Integration

- [x] 5.1 Add `ensure_daemon_running()` function — calls `is_daemon_alive()`, spawns if needed
- [x] 5.2 Call `ensure_daemon_running()` at the top of `main()` (after CLI parse, before command dispatch) — fire-and-forget, never blocks or errors

## 6. Remove Synchronous Refresh from Hot Path

- [x] 6.1 Remove `refresh_pr_status(&mut tree, &session.id)` call from `enter_session` in `src/session/mod.rs` (keep `detect_pr_for_session`)
- [x] 6.2 Verify that browser `update_last_accessed` still calls `detect_pr_for_session` but does not refresh synchronously

## 7. Daemon Logging

- [x] 7.1 Set up daemon log file — truncate on start, use `env_logger` or `log` with file target
- [x] 7.2 Add log messages for: daemon start/stop, each refresh cycle (count of sessions refreshed), per-session refresh results, skipped sessions (wrong gh user, fresh data)

## 8. Testing & Validation

- [x] 8.1 Test daemon lifecycle: start → status shows running → stop → status shows stopped
- [x] 8.2 Test auto-start: run `ez` with no daemon → verify daemon.pid created
- [x] 8.3 Test stale PID cleanup: write a fake PID file with dead PID → run `ez` → verify new daemon spawned
- [x] 8.4 Test PR refresh: create a session with stale `ez_pr_status_updated` → run daemon cycle → verify status updated
- [x] 8.5 Test gh user filtering: set `ez_pr_gh_user` to a different user → verify daemon skips the session
