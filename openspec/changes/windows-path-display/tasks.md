## 1. Core fix

- [x] 1.1 In `paths::normalize` (`src/paths.rs:98`), add a `#[cfg(windows)]` block after `canonicalize` succeeds that strips the `\\?\` prefix from the resolved path before returning it. Use `resolved.to_string_lossy()` to check for and strip the prefix, then return `PathBuf::from(stripped)`.

## 2. Replace direct canonicalize calls

Replace `path.canonicalize()` with `paths::normalize(path)` (or `crate::paths::normalize(path)`) at call sites whose results flow to display or registered-path comparison.

- [x] 2.1 `src/session/mod.rs` — replace all direct `.canonicalize()` calls (~12 sites at lines 487, 496, 507, 529, 546, 564, 1821, 1864, 1874, 1881, 1895, 1901, 1907) with `crate::paths::normalize(&path)`. Adjust error handling where `canonicalize` was `?`-unwrapped, since `normalize` never fails (returns input on error).
- [x] 2.2 `src/browser/mod.rs` — replace `.canonicalize()` calls (~3 sites at lines 256, 262, 1262) with `crate::paths::normalize`.
- [x] 2.3 `src/repo/mod.rs` — replace `.canonicalize()` call (line 451) with `crate::paths::normalize`.
- [x] 2.4 `src/session/cursor.rs` — replace `.canonicalize()` call (line 38) with `crate::paths::normalize`.
- [x] 2.5 `src/session/current.rs` — replace `.canonicalize()` call (line 343) with `crate::paths::normalize`.

## 3. Verification

- [x] 3.1 Run `cargo build` on Windows and confirm zero errors.
- [x] 3.2 Run `cargo install --path .` and launch `ez` — verify repo paths in the browser display with `~` instead of `\\?\C:\Users\...`.
