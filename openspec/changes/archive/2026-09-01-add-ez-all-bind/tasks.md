## 1. Config

- [x] 1.1 Add `view_all` field to `KeybindsConfig` in `src/config/model.rs` with default `"ctrl-a"`

## 2. Return type

- [x] 2.1 Add `SessionLoopResult` enum (`Accepted`, `Cancelled`, `ViewAll`) in `src/browser/mod.rs`
- [x] 2.2 Change `session_action_loop` return type from `Result<bool>` to `Result<SessionLoopResult>`
- [x] 2.3 Update `browse_repo` to handle `SessionLoopResult` (map `ViewAll` to the same behavior as `Cancelled` — fall through)

## 3. Keybind handling

- [x] 3.1 Add `keybinds.view_all` to `expect_keys` in `session_action_loop`
- [x] 3.2 Handle the `view_all` key in the action match — return `SessionLoopResult::ViewAll`

## 4. Browse fallthrough

- [x] 4.1 In `browse()`, when auto-detect or `--repo` gets `ViewAll`, fall through to `views::run` instead of returning
