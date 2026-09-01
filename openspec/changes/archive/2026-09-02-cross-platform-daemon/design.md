## Context

The daemon module (`src/daemon/mod.rs`) manages a background process that refreshes PR statuses. It was written for Unix, using `libc` for process signaling (`kill`), signal handling (`signal`/`SIGTERM`), and file locking (`flock`). These APIs do not exist on Windows, blocking compilation.

## Goals / Non-Goals

**Goals:**
- Compile and run on Windows without new crate dependencies
- Preserve identical Unix behavior (no regressions)
- Keep platform-specific code isolated and minimal

**Non-Goals:**
- Full cross-platform test suite for daemon behavior
- Supporting platforms beyond Unix and Windows (e.g., WASI)
- Changing daemon architecture or polling logic

## Decisions

### Inline FFI over a crate dependency

Use `extern "system"` blocks with Win32 function declarations directly in the source instead of adding `windows-sys` or `winapi` as a dependency.

**Why:** Only five Win32 functions are needed (`OpenProcess`, `CloseHandle`, `TerminateProcess`, `SetConsoleCtrlHandler`, `LockFileEx`/`UnlockFileEx`). Inline declarations keep `Cargo.toml` clean and avoid pulling in a large crate for a handful of calls.

**Alternative considered:** `windows-sys` crate — rejected because it adds significant compile-time cost for minimal surface area used.

### Platform helpers instead of cfg-gated blocks inline

Extract each platform operation into a named helper function (`is_process_alive`, `terminate_process`, `install_stop_handler`, `lock_file_exclusive`, `unlock_file`) with `#[cfg(unix)]` and `#[cfg(windows)]` variants, rather than sprinkling `cfg` blocks inside the existing functions.

**Why:** The main module logic reads the same on both platforms. Each helper has a single responsibility and a clear API contract, making it easy to verify correctness per platform.

### TerminateProcess instead of graceful shutdown on Windows

On Unix, `SIGTERM` allows the daemon to run cleanup (remove PID file). On Windows, `TerminateProcess` is immediate — no cleanup handler runs.

**Why:** The `stop_daemon` function already removes the PID file from the caller's side after termination, so the daemon process itself does not need to clean up. The `SetConsoleCtrlHandler` callback handles the `daemon run` loop's own graceful shutdown (Ctrl+C, console close).

### OVERLAPPED struct defined locally

A `#[repr(C)]` `Overlapped` struct is defined in the module for `LockFileEx`/`UnlockFileEx`, zero-initialized.

**Why:** The struct is trivial (5 fields), used only in two functions, and avoids depending on external type definitions.

## Risks / Trade-offs

- **Inline FFI correctness** — Win32 function signatures are hand-written. Mitigation: signatures match MSDN documentation; the functions are well-known and stable.
- **TerminateProcess is forceful** — if the daemon is mid-write when `ez daemon stop` runs, the sessions file could be left in a partial state. Mitigation: the file lock prevents concurrent access, and the interactive `ez` process (the one calling `stop`) removes the PID file itself. The sessions file write is a small atomic `fs::write`.
- **No Windows CI yet** — the Windows path compiles but is not tested in CI. Mitigation: manual verification on the developer's Windows machine; CI coverage is a follow-up concern.
