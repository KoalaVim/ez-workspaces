## 1. Config: Add clone keybind

- [x] 1.1 Add `clone_repo` field to `KeybindsConfig` in `src/config/model.rs` with default `"alt-a"` and serde default function
- [x] 1.2 Update `KeybindsConfig::default()` to include the new field

## 2. Core: Convert drill-down to support actions

- [x] 2.1 Refactor `drill_into_directory` in `src/browser/mod.rs` to accept `config` parameter (needed for keybind access)
- [x] 2.2 Change `drill_into_directory` from `select_one` to `select_with_actions`, registering the `clone_repo` keybind as an `--expect` key
- [x] 2.3 Handle `ActionResult::Action` for the clone keybind: prompt for URL via `selector.input`, call `repo::clone_repo` with the current directory as target, and return the cloned repo path
- [x] 2.4 Handle clone errors gracefully — display error with `eprintln!` and continue the drill-down loop
- [x] 2.5 Handle empty/cancelled URL input — return to the directory listing

## 3. Browser: Wire clone result into session picker

- [x] 3.1 Update `drill_into_directory` return type or the workspace view caller to handle the "cloned repo" case and transition into `browse_repo` for the newly cloned path
- [x] 3.2 Update callers of `drill_into_directory` in `src/browser/views/workspace.rs` and `src/browser/views/tree.rs` to pass the config parameter

## 4. Preview: Show clone keybind in directory help

- [x] 4.1 Add the clone keybind to the directory preview keybind table in `src/browser/preview.rs`

## 5. Documentation

- [x] 5.1 Update `README.md` with the new clone-from-browser keybind
- [x] 5.2 Update `docs/user-guide.md` with the new clone-from-browser keybind
- [x] 5.3 Update `AGENTS.md` if any module-level descriptions change

## 6. Verification

- [x] 6.1 Run `make check` (fmt + clippy + tests) and fix any warnings or errors
- [ ] 6.2 Manual test: browse workspace → press alt-a → enter URL → verify clone + session picker transition
