## 1. Normalization helper

- [x] 1.1 Add `paths::normalize(path: &Path) -> PathBuf` in `src/paths.rs`: canonicalize, falling back to the input path on error. Infallible.
- [x] 1.2 Unit tests in `src/paths.rs`: symlinked path resolves to target, direct path is unchanged, non-existent path returns input unchanged. Canonicalize the expected value in fixtures so `/tmp` → `/private/tmp` does not break the macOS run.

## 2. Symlink-aware lookup

- [x] 2.1 Change `RepoIndex::find_by_path` in `src/repo/model.rs` to normalize the query once and match `r.path == path || r.path == canonical`. Keep the raw-equality arm so entries whose directory was deleted stay addressable.
- [x] 2.2 Unit tests in `src/repo/model.rs`: lookup by symlink path hits, lookup by canonical path hits, lookup of a deleted registered path still hits by exact match, lookup of an unrelated path misses.
- [x] 2.3 Route registration through the helper in `src/repo/mod.rs` (`add_repo`, `clone_repo`), preserving the existing existence check so registering a non-existent directory still errors.

## 3. Auto-register panic

- [x] 3.1 Replace `.expect("just registered")` in `src/browser/mod.rs:141` with a returned `EzError`, so a future regression surfaces as a message rather than a crash.
- [x] 3.2 Verify the post-registration re-lookup now succeeds for a symlinked path (fixed by 2.1) and that browsing an already-registered symlinked repo enters the session picker instead of reporting `RepoAlreadyRegistered`.

## 4. Verify the affected surfaces

- [x] 4.1 Confirm no scan site needs its own change — `views/tree.rs:103`, `preview.rs:33/45/196/297`, and `browser/mod.rs:126/794` should all be fixed by 2.1 alone. Fix any that are not.
- [x] 4.2 Confirm display paths are untouched: the Tree view and preview still print the scanned symlink path, not the resolved target.

## 5. Manual verification

- [x] 5.1 `ez preview ~/workspaces/personal/koala/KoalaVim` shows both sessions and no `(unregistered — select to register)`, and still prints the symlink path in its header.
- [x] 5.2 `ez preview ~/.local/share/kvim-envs/main/lazy/KoalaVim` output is unchanged from before the fix.
- [x] 5.3 In `ez all tree`, KoalaVim shows its session subtree under `~/workspaces/personal/koala`, and selecting a session enters its worktree.
- [x] 5.4 Create a throwaway symlink to an unregistered git repo, select it in the browser, and confirm it registers and opens the session picker without panicking.

## 6. Checks

- [x] 6.1 `cargo test` passes. 102 passed, 0 failed.
- [x] 6.2 `cargo clippy --all-targets` clean, zero warnings. `rustfmt --check` clean on all four touched files. `make check` still fails on a **pre-existing** fmt diff in `src/session/current.rs:254` (committed in `63eb51e`, untouched by this change) — left alone as out of scope.
