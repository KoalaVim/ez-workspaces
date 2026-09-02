## 1. Error Variant

- [x] 1.1 Add `BashNotFound` variant to `EzError` in `src/error.rs` with a message directing users to install Git for Windows

## 2. Bash Discovery

- [x] 2.1 Add `find_bash()` function in `src/plugin/runner.rs` that resolves `bash.exe` on Windows using the fallback chain (PATH → `C:\Program Files\Git\usr\bin\bash.exe` → `C:\Program Files (x86)\Git\usr\bin\bash.exe`) and caches the result in a `OnceLock<PathBuf>`
- [x] 2.2 On Unix, `find_bash()` returns `PathBuf::from("sh")` (no-op, preserves existing behavior)

## 3. Plugin Execution

- [x] 3.1 Modify `execute()` in `src/plugin/runner.rs`: on Windows, spawn `bash.exe <script>` via `find_bash()`; on Unix, keep `Command::new(&executable)`
- [x] 3.2 Modify `run_shell_commands()` in `src/plugin/runner.rs`: on Windows, use `bash.exe -c` via `find_bash()`; on Unix, keep `sh -c`

## 4. Documentation

- [x] 4.1 Add Windows prerequisites section to README documenting Git for Windows and jq requirements
- [x] 4.2 Add `setup` target to Makefile with platform detection: `winget` on Windows, `brew` on macOS, `apt-get` on Linux

## 5. Path Normalization

- [x] 5.1 Add `strip_unc_prefix()` helper in `src/paths.rs` that strips the `\\?\` extended-length path prefix on Windows (no-op on Unix)
- [x] 5.2 Apply `strip_unc_prefix()` in `src/repo/store.rs` `load_index()` to clean repo paths on load
- [x] 5.3 Apply `strip_unc_prefix()` in `src/session/store.rs` `load_sessions()` to clean session paths on load

## 6. Verification

- [x] 6.1 Build on Windows (`cargo build`) and verify no compilation errors
- [x] 6.2 Run `ez plugin list` on Windows and verify plugins are listed
- [x] 6.3 Test git-worktree plugin on Windows: create a session and verify worktree is created
