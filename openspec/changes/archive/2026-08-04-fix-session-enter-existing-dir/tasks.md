## 1. Rust: Add relaunch marker support

- [x] 1.1 Add `write_relaunch_marker` function in `src/browser/mod.rs` that appends `#EZ_RELAUNCH\n` to the post-cmd-file
- [x] 1.2 Call `write_relaunch_marker` in `session_action_loop` (around line 700) after `apply_bind_response` when a plugin bind action has effect and the browser is exiting

## 2. Shell wrapper: Marker-gated re-invocation

- [x] 2.1 Update bash/zsh wrapper in `print_shell_init` (`src/main.rs`) to check for `#EZ_RELAUNCH` marker before looping — always source post_cmd, but only `continue` when the marker is present
- [x] 2.2 Update fish wrapper with equivalent marker-based logic
- [x] 2.3 Update pwsh wrapper with equivalent marker-based logic

## 3. Verify

- [x] 3.1 Build and test: `ez session enter model-store-editor-api` from `~/workspaces/work/hypersonic` succeeds with env exports applied and no spurious re-invocation
- [x] 3.2 Verify browser plugin bind re-invocation still works (marker is written, wrapper loops)
