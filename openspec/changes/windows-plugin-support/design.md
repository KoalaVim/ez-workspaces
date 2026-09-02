## Context

The plugin runner (`src/plugin/runner.rs`) executes plugins by spawning them directly as executables (`Command::new(&executable)`) and runs plugin-returned shell commands via `Command::new("sh").args(["-c", cmd])`. Both rely on Unix conventions: shebang resolution for the former, `sh` on PATH for the latter. All bundled plugins are bash scripts that depend on `jq` for JSON parsing. On Windows, none of this works — shebangs are ignored, `sh` doesn't exist, and `jq` isn't pre-installed.

The `cross-platform-daemon` change already made the daemon module compile on Windows. This change extends cross-platform support to the plugin runner, the last major subsystem that's Unix-only.

## Goals / Non-Goals

**Goals:**
- Plugin scripts execute correctly on Windows via Git Bash's `bash.exe`
- Clear error message when bash is not found on Windows
- `run_shell_commands` works cross-platform
- README documents Windows prerequisites (Git for Windows, jq)
- `make setup` installs prerequisites per platform

**Non-Goals:**
- Rewriting any plugin scripts — they remain bash
- Supporting Windows-native plugin scripts (PowerShell, .bat)
- Eliminating the jq dependency
- CI testing on Windows (follow-up concern)

## Decisions

### `find_bash()` helper with fallback chain

Add a `find_bash()` function that returns the path to `bash.exe`. On Unix, this is a no-op — plugins are invoked directly. On Windows:

1. Try `bash` on PATH via `which::which("bash")` (or `Command::new("bash").arg("--version")` as a probe)
2. Fall back to `C:\Program Files\Git\usr\bin\bash.exe`
3. Fall back to `C:\Program Files (x86)\Git\usr\bin\bash.exe`
4. Return `EzError::BashNotFound` with a message: "Plugins require bash. Install Git for Windows: https://gitforwindows.org"

Cache the result in a `OnceLock<PathBuf>` so it's resolved once per process.

**Why no registry lookup:** Checking `HKLM\SOFTWARE\GitForWindows\InstallPath` adds complexity for marginal benefit. The two hardcoded paths cover the standard Git for Windows installer. Users with non-standard installs will have bash on PATH.

### Invoke `bash <script>` on Windows, direct execution on Unix

In `execute()`:
- On Unix (unchanged): `Command::new(&executable)` — the OS handles the shebang
- On Windows: `Command::new(find_bash()?).arg(&executable)` — bash interprets the script

**Why not always use bash:** On Unix, direct execution respects the shebang, allowing plugins to be written in any language. This preserves the existing contract.

### `run_shell_commands` uses `bash -c` on Windows

In `run_shell_commands()`:
- On Unix (unchanged): `Command::new("sh").args(["-c", cmd])`
- On Windows: `Command::new(find_bash()?).args(["-c", cmd])`

**Alternative considered:** Using PowerShell — rejected because plugin-returned shell commands are written in bash syntax (e.g., `tmux switch-client -t foo`). Running them through PowerShell would break.

### No new error variant needed

Reuse `EzError::PluginFailed` with a descriptive message for bash-not-found. A dedicated variant is overkill for a single check.

**Update:** Actually, a dedicated `EzError::BashNotFound` variant gives a cleaner error path — the message is always the same and doesn't need a plugin name. Use it.

### `strip_unc_prefix()` for stored paths

`std::fs::canonicalize()` on Windows returns paths with a `\\?\` extended-length prefix (e.g. `\\?\C:\Users\...`). Paths registered before `normalize()` stripped this prefix are stored with it. Bash cannot handle this prefix — `dirname`, `mkdir`, and other commands break on it.

Add a lightweight `strip_unc_prefix()` in `paths.rs` that strips the prefix without re-canonicalizing. Apply it in `load_index()` and `load_sessions()` so all downstream code sees clean paths.

**Why strip on load, not in `build_request()`:** Fixing it at the data loading boundary ensures every consumer (plugins, display, path comparisons) gets clean paths, not just the plugin protocol.

### Makefile `setup` target with platform detection

Use `$(OS)` for Windows detection (`Windows_NT`) and `uname -s` for macOS vs Linux. The target installs:
- Windows: `winget install Git.Git` and `winget install stedolan.jq`
- macOS: `brew install jq` (bash comes with Xcode CLI tools or is pre-installed)
- Linux: `sudo apt-get install -y jq` (with a note for non-Debian distros)

## Risks / Trade-offs

- **Git Bash not installed** — Users without Git for Windows get a clear error. Mitigation: `make setup` installs it; README documents the requirement.
- **jq not installed** — Plugins will fail with a `jq: command not found` error from bash, which surfaces as a `PluginFailed` error. Mitigation: `make setup` installs jq; README documents the requirement. A future enhancement could pre-check for jq.
- **`\\?\` path prefix in stored data** — Legacy repo and session paths stored before `normalize()` stripped the prefix caused bash commands (`mkdir`, `dirname`) to fail. Mitigation: `strip_unc_prefix()` applied on load cleans these paths for all consumers. New registrations already produce clean paths via `normalize()`.
- **`OnceLock` cache** — If bash is installed while ez is running, the cached "not found" result won't pick it up until restart. Acceptable — this is an install-time concern.
