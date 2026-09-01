## Why

The daemon module (`src/daemon/mod.rs`) uses Unix-only APIs (`libc::kill`, `libc::flock`, `libc::signal`, `std::os::unix::io::AsRawFd`), preventing compilation on Windows. Since ez-workspaces targets git worktree workflows that work identically on Windows, the daemon needs cross-platform support.

## What Changes

- Extract five platform-specific operations into helper functions behind `#[cfg(unix)]` / `#[cfg(windows)]` gates: process-alive check, process termination, stop-signal handler, exclusive file lock, and file unlock.
- Windows implementations use inline `extern "system"` FFI against the Win32 API (`OpenProcess`, `TerminateProcess`, `SetConsoleCtrlHandler`, `LockFileEx`, `UnlockFileEx`), adding no new dependencies.
- Move the `libc` crate from `[dependencies]` to `[target.'cfg(unix)'.dependencies]` in `Cargo.toml`.
- Update the `bg-pr-daemon` spec to use platform-neutral language instead of Unix-specific mechanism names.

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities

- `bg-pr-daemon`: Requirements that reference Unix-specific mechanisms (`kill -0`, `SIGTERM`, `flock`) are revised to use platform-neutral language, since the implementation now abstracts these behind cross-platform helpers.

## Impact

- **Code**: `src/daemon/mod.rs` — all platform-specific code isolated into six helper functions at the bottom of the file; the rest of the module is unchanged.
- **Dependencies**: `Cargo.toml` — `libc` moves to a Unix-only target dependency. No new crates added.
- **Build**: The project now compiles and installs on Windows (`cargo build` / `cargo install --path .`).
