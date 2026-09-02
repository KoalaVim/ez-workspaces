## Why

All bundled plugins are bash scripts that use `jq` for JSON parsing. On Unix, shebang resolution and `sh` availability make this transparent. On Windows, the plugin runner cannot execute these scripts — `Command::new(&executable)` fails because Windows doesn't interpret shebangs, and `run_shell_commands` hardcodes `sh -c` which doesn't exist. This blocks the plugin system entirely on Windows, even though the core ez binary now compiles and runs there (see `cross-platform-daemon` change).

## What Changes

- Plugin runner (`src/plugin/runner.rs`) detects Windows and invokes `bash.exe <script>` instead of relying on shebang resolution
- `run_shell_commands` uses `bash -c` instead of `sh -c` on Windows
- A `find_bash()` helper locates bash: tries PATH first, falls back to known Git for Windows install paths, returns a clear error if not found
- `strip_unc_prefix()` helper in `src/paths.rs` strips the `\\?\` extended-length path prefix that `std::fs::canonicalize()` adds on Windows; applied when loading repo index and session trees so bash plugins receive clean paths
- README documents bash (Git for Windows) and jq as Windows prerequisites
- Makefile gains a `setup` target that installs prerequisites via `winget` on Windows

## Capabilities

### New Capabilities
- `windows-plugin-runner`: Cross-platform plugin execution — finding bash on Windows and using it to run plugin scripts and shell commands

### Modified Capabilities
- `plugin-system`: The plugin execution requirement changes to support Windows by explicitly invoking bash rather than relying on OS shebang resolution

## Impact

- `src/plugin/runner.rs` — both `execute()` and `run_shell_commands()` gain platform-aware shell invocation
- `src/paths.rs` — new `strip_unc_prefix()` helper
- `src/repo/store.rs` — strips `\\?\` prefix on repo index load
- `src/session/store.rs` — strips `\\?\` prefix on session tree load
- `Makefile` — new `setup` target
- `README.md` — new prerequisites section for Windows
- No changes to the plugin protocol, manifest format, or any plugin scripts
- tmux/zellij plugins remain Unix-only (they won't be enabled on Windows, but won't cause errors)
