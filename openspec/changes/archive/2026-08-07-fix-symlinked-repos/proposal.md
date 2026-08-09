## Why

Repo registration canonicalizes paths (`fs::canonicalize` in `add_repo`/`clone_repo`), but every filesystem-scan reader passes the raw `read_dir` path into `RepoIndex::find_by_path`, which compares `PathBuf` values for exact equality. For a repo reached through a symlink the two sides can never agree, so scan-driven surfaces report a registered repo as unregistered and render none of its sessions — while index-driven surfaces (`ez session list --all`, Repo/Owner/Label views) show it correctly.

Reproduced with `~/workspaces/personal/koala/KoalaVim -> ~/.local/share/kvim-envs/main/lazy/KoalaVim`: `ez preview` on the link path prints `(unregistered — select to register)`; on the target path it prints both sessions. The same mismatch makes the browser panic (`.expect("just registered")`) the first time a not-yet-registered symlinked repo is selected.

## What Changes

- Introduce a single path-normalization helper in `paths.rs` and route both the write side (registration) and the read side (index lookup) through it, making "index paths are canonical" an enforced invariant rather than an implicit one.
- Make `RepoIndex::find_by_path` symlink-aware: canonicalize the query once, match against the stored path, and fall back to raw equality when canonicalization fails (deleted or inaccessible path). All seven current call sites are fixed by this one change.
- Fix the auto-register re-lookup in `browse_repo` so registering a symlinked repo returns the new entry instead of panicking.
- Preserve display paths exactly as scanned: the tree and preview continue to show `~/workspaces/personal/koala/KoalaVim`, not the resolved target. Only comparison changes, never presentation.

Not in scope: deduplicating a repo that is reachable from more than one scanned workspace root. Resolving symlinks makes that newly possible, but it needs a second symlink to trigger and is tracked separately.

No breaking changes. The on-disk index format is unchanged and existing entries are already canonical.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `repo-management`: `Repo identity` gains a path-normalization invariant — registered paths are canonical, and lookup by path resolves symlinks before comparing. `Auto-register on browse` gains the requirement that the post-registration re-lookup succeeds for symlinked paths.
- `interactive-browser`: `Tree view`, `Workspace view with drill-down`, and `Preview pane` gain the requirement that a repo reached through a symlink resolves to its index entry — showing its sessions, labels, and metadata — while still displaying the path as scanned.

## Impact

Affected code:

- `src/paths.rs` — new normalization helper
- `src/repo/model.rs` — `RepoIndex::find_by_path`
- `src/repo/mod.rs` — registration routed through the helper (`add_repo`, `clone_repo`, `remove_repo`)
- `src/browser/mod.rs` — `browse_repo` re-lookup panic; drill-down label lookup
- `src/browser/views/tree.rs` — session subtree lookup
- `src/browser/preview.rs` — registered/unregistered determination, session preview, metadata lookup

Not affected: anything cwd-derived. `std::env::current_dir()` calls `getcwd(3)`, which already returns the physical path, so `resolve_repo` and `detect_repo_root` see canonical paths today.

Behavioral risk: one extra `stat` per index lookup. Bounded by repo count per view render.
