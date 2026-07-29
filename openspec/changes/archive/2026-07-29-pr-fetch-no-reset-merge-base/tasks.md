## 1. Remove merge-base reset logic

- [x] 1.1 Delete the `pr_merge_base_reset` function from `src/browser/mod.rs`
- [x] 1.2 Remove the `pr_merge_base_reset` call in `src/session/mod.rs` (`new_session`, lines 350-354)
- [x] 1.3 Remove the `pr_merge_base_reset` call in `src/browser/mod.rs` (browser session-create flow, lines 488-492)

## 2. Update spec

- [x] 2.1 Remove the "Reset to merge-base after checkout", "Merge-base resolution", and "Reset failure" requirements and scenarios from `openspec/specs/pr-checkout/spec.md`

## 3. Verify

- [x] 3.1 Run `make check` to ensure the build compiles cleanly with no warnings and tests pass
