## 1. Dependencies

- [x] 1.1 Move `libc` from `[dependencies]` to `[target.'cfg(unix)'.dependencies]` in `Cargo.toml`. The crate is only used for Unix-specific syscalls in the daemon module; keeping it as a universal dependency pulls in Windows stubs that provide none of the symbols the code actually calls (`kill`, `flock`, `signal`).

## 2. Platform helpers in daemon module

Add five cross-platform helper functions at the bottom of `src/daemon/mod.rs`, each with a `#[cfg(unix)]` and `#[cfg(windows)]` variant. All Win32 functions are declared via inline `extern "system"` blocks — no new crate dependency.

- [x] 2.1 `is_process_alive(pid: u32) -> bool` — Unix: call `libc::kill(pid as pid_t, 0)` and return whether it succeeds (signal 0 is a no-op existence check). Windows: call `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid)` — a non-zero handle means the process exists; close the handle immediately with `CloseHandle`.
- [x] 2.2 `terminate_process(pid: u32)` — Unix: send `SIGTERM` via `libc::kill`. Windows: open the process with `PROCESS_TERMINATE` access, call `TerminateProcess(handle, 1)` for an immediate exit with code 1, then `CloseHandle`. Note: `TerminateProcess` does not run the in-process ctrl handler; this is acceptable because the caller (`stop_daemon`) already removes the PID file from its own side.
- [x] 2.3 `install_stop_handler()` — Unix: register an `extern "C"` function as the `SIGTERM` handler via `libc::signal`; the handler sets the `SHOULD_STOP` atomic to `true`. Windows: register an `unsafe extern "system"` callback via `SetConsoleCtrlHandler` that sets the same atomic on any control event (Ctrl+C, console close, etc.) and returns 1 (handled).
- [x] 2.4 `lock_file_exclusive(file: &fs::File)` / `unlock_file(file: &fs::File)` — Unix: use `libc::flock` with `LOCK_EX` (blocking exclusive) and `LOCK_UN` via `AsRawFd`. Windows: define a `#[repr(C)] Overlapped` struct (zero-initialized, matching the Win32 `OVERLAPPED` layout), then call `LockFileEx` with `LOCKFILE_EXCLUSIVE_LOCK` and `UnlockFileEx` via `AsRawHandle`. Lock the maximum byte range (`u32::MAX, u32::MAX`) to match `flock` whole-file semantics.

## 3. Refactor call sites

Replace every direct `libc` call in the module's business logic with the new helpers, so the main functions read identically on both platforms.

- [x] 3.1 In `is_daemon_alive`: replace `unsafe { libc::kill(pid as libc::pid_t, 0) } == 0` with `is_process_alive(pid)`.
- [x] 3.2 In `stop_daemon`: replace the `unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) }` block with `terminate_process(pid)`. Update the log message from "sending SIGTERM" to "terminating".
- [x] 3.3 In `daemon_run`: replace the inline `extern "C" fn handle_sigterm` definition and `libc::signal(SIGTERM, ...)` call with `install_stop_handler()`. Remove the now-unused module-level `handle_sigterm` function.
- [x] 3.4 In `with_session_lock`: replace `libc::flock(file.as_raw_fd(), libc::LOCK_EX)` and `libc::flock(file.as_raw_fd(), libc::LOCK_UN)` with `lock_file_exclusive(&file)` and `unlock_file(&file)`.
- [x] 3.5 Remove the module-level `use std::os::unix::io::AsRawFd` import — the platform-specific imports now live inside their respective `#[cfg]` helper functions.

## 4. Spec update

Update `openspec/specs/bg-pr-daemon/spec.md` to use platform-neutral language so the spec describes observable behavior, not Unix implementation details.

- [x] 4.1 Auto-start requirement (line 10): change "verifying the PID is alive (`kill -0`)" to "verifying the PID is alive" — the mechanism is an implementation detail that now varies by platform.
- [x] 4.2 CLI surface requirement (line 44): change "send SIGTERM and remove PID file" to "terminate the daemon process and remove PID file". Update scenario text similarly: "daemon receives SIGTERM, exits gracefully" → "daemon is terminated, exits".
- [x] 4.3 File locking requirement (line 96): change "exclusive file lock (`flock`)" to "exclusive file lock" — both `flock` (Unix) and `LockFileEx` (Windows) provide the same guarantee.

## 5. Verification

- [x] 5.1 Run `cargo build` on Windows and confirm zero errors and zero warnings related to the daemon module.
- [x] 5.2 Run `cargo install --path .` on Windows and confirm the `ez.exe` binary is placed in `~/.cargo/bin/` and launches without immediate crash (`ez --version` or `ez daemon status`).
