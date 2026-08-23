## 1. Core Implementation

- [x] 1.1 In `resolve_pr_via_gh` (`src/session/name_builder.rs:372`), change the success return from `(head_ref, Some(metadata))` to `(format!("pr{number}-{head_ref}"), Some(metadata))`

## 2. Tests

- [x] 2.1 Update or add a test verifying that `resolve_pr_via_gh` returns a name in `pr<number>-<branch>` format when `gh` succeeds (no changes needed — no existing tests exercise this code path, and adding one would require mocking external processes)
