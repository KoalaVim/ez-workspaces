## Context

`ez` carries two parallel notions of repo identity.

```
  INDEX-DRIVEN                          SCAN-DRIVEN
  ────────────                          ───────────
  session list --all                    all tree
  Repo / Owner / Label views            workspace drill-down
                                        preview

  identity = index entry                identity = read_dir() path
  ✓ always correct                      ✗ breaks on symlink
```

`RepoIndex::find_by_path` is the sole bridge between them, and it leaks:

```
        WRITE PATH (canonicalizing)          READ PATH (not)
        ───────────────────────────          ───────────────

  ez add ~/…/koala/KoalaVim            fs::read_dir(~/…/koala)
          │                                     │
          ▼                                     ▼
  repo/mod.rs:55                        entry.path()
  fs::canonicalize(p)                  = ~/…/koala/KoalaVim
          │                              (link, NOT resolved)
          ▼                                     │
  index.toml                                    ▼
  path = /Users/amitt/.local/…/    index.find_by_path(that)
         lazy/KoalaVim                          │
              │                                 ▼
              └──► PathBuf == PathBuf ◄──── never equal
                          │
                          ▼
                        None
                "unregistered", 0 sessions
```

This is structural, not a race: registration always canonicalizes, scans never do, so the two sides are *guaranteed* to disagree for any symlinked repo.

Current call sites of `find_by_path` and what each does wrong today:

| Site | Path source | Symptom |
|---|---|---|
| `views/tree.rs:103` | `read_dir` | no sessions rendered under the repo |
| `preview.rs:33` | selector value | `(unregistered — select to register)` |
| `preview.rs:45` | selector value | `Repo not registered` on session preview |
| `preview.rs:196,297` | selector value | labels / metadata missing |
| `browser/mod.rs:126` | tree or drill selection | falls through to `add_repo` → `RepoAlreadyRegistered` |
| `browser/mod.rs:139` | same, after registering | **panic** on `.expect("just registered")` |
| `browser/mod.rs:794` | drill `read_dir` | labels missing in display |
| `repo/mod.rs:96` | canonical (registration) | correct already |
| `repo/mod.rs:365,373` | canonicalized by caller | correct already |

The `mod.rs:139` panic is the sharpest edge. For a repo that is not yet registered:

```rust
repo::add_repo(Some(repo_path))?;        // canonicalizes, registers the TARGET
let index = repo::store::load_index()?;
index
    .find_by_path(repo_path)             // still the LINK path → None
    .cloned()
    .expect("just registered")           // 💥
```

It is only masked today because KoalaVim was already registered, so `add_repo` returns `RepoAlreadyRegistered` and the `?` exits before the `expect`.

## Goals / Non-Goals

**Goals:**

- A repo reached through a symlink resolves to its index entry everywhere a directly-reached repo does.
- One intervention point, not seven. Adding a future `find_by_path` caller must not reintroduce the bug.
- Display paths stay exactly as scanned — the tree keeps showing `~/workspaces/personal/koala/KoalaVim`.
- No on-disk format change, no migration.

**Non-Goals:**

- Deduplicating a repo reachable from two scanned workspace roots. Newly possible once lookups resolve, but requires a second symlink to trigger. Tracked separately.
- Changing where session worktrees are created. They land beside the canonical target (`…/lazy/.ez/KoalaVim/<branch>`), which is correct; this change does not touch it.
- Supporting symlinks *inside* a repo, or symlinked worktree paths. Out of scope.

## Decisions

### Normalize in the lookup, not at each scan site

Four places could absorb the fix:

```
  read_dir ──► entry.path ──► find_by_path ──► index.path
      │            │               │               │
      D            B               A               C
   don't        resolve at      resolve in     store both
   follow        the scan       the lookup       paths
```

**Chosen: A.** One function covers all seven call sites plus any future one, and it leaves display untouched, since only the comparison changes.

- **B — resolve at each scan site.** Rejected: seven edits instead of one, no protection against the eighth caller, and it changes what the tree *displays* (the preview would print `/Users/amitt/.local/share/kvim-envs/…` under a `koala/` heading), losing the mental model the symlink exists to provide.
- **C — store `path` + `canonical_path`.** Rejected: schema migration for no added capability, and it still leaves the comparison question open when two links point at one target.
- **D — skip symlinked directories in the scan.** Rejected: removes the feature.

### Canonicalize the query only, not the stored entries

Because registration is the only writer and it always canonicalizes, index paths are already canonical. So the lookup needs one `stat` for the query, not one per entry:

```rust
pub fn find_by_path(&self, path: &Path) -> Option<&RepoEntry> {
    let canonical = paths::normalize(path);
    self.repos
        .iter()
        .find(|r| r.path == path || r.path == canonical)
}
```

The raw-equality arm is kept as a cheap fallback for entries whose path no longer exists on disk (repo moved or deleted) — those still resolve by exact match, so `ez repo remove` keeps working on a stale entry.

The alternative — canonicalizing every entry on every lookup — is O(N) syscalls per lookup and O(N²) across a tree render, for no correctness gain given the write-side invariant.

### Make the invariant explicit

Add `paths::normalize(path) -> PathBuf` (canonicalize, falling back to the input on error) and route the write side through it too. Today `add_repo` uses `fs::canonicalize(p)?`, which *errors* when the path does not exist; `normalize` is infallible. The registration path should keep its existence check — a non-existent directory is a real error there — but expressing both sides in terms of one helper is what stops the two from drifting again.

### Fix the re-lookup by construction

Rather than re-searching the index after `add_repo`, `browse_repo` should not need `.expect` at all. Two options:

- Have `add_repo` return the `RepoEntry` it created, so `browse_repo` uses it directly.
- Keep the re-lookup, which decision A now makes correct.

**Chosen: keep the re-lookup**, since A already fixes it, and change the `.expect` to a proper `EzError` so a future regression surfaces as a message rather than a panic. Returning the entry from `add_repo` is a cleaner API but widens the diff into `clone_repo` and the CLI dispatch for no user-visible gain here.

## Risks / Trade-offs

- **One extra `stat` per index lookup** → Bounded by repos-per-render (single digits to low tens in practice); `find_by_path` is not in a hot loop. If it ever matters, hoist the canonicalization above the loop in the tree builder.
- **`canonicalize` fails on a broken symlink or unreadable path** → `normalize` falls back to the input, and the raw-equality arm still matches. A broken link is skipped by the scan's `is_dir()` check anyway.
- **A repo reachable from two roots now renders twice** → Accepted, explicitly out of scope. Not a regression: today both copies render as unregistered, so this trades two wrong rows for two right ones.
- **Registration behavior for a symlinked path is unchanged** → `ez add ~/link` still registers the *target* path, so the repo id stays target-derived (`lazy-koalavim`, not `koala-koalavim`). Slightly opaque, but changing it would break existing metadata directories. Not touched.
- **macOS `/tmp` → `/private/tmp`** → Test fixtures that build paths under `/tmp` must canonicalize expectations, or assertions comparing a constructed path to a normalized one will fail on macOS and pass on Linux.

## Migration Plan

None required. The on-disk index is unchanged, and existing entries are already canonical because every writer canonicalizes. Rollback is a straight revert.

## Open Questions

- Should `find_by_path` also match when the *stored* path is a symlink — i.e. an entry written before registration canonicalized, if any such vintage exists? Current thinking: no. All writers canonicalize, and the raw-equality arm covers any stragglers.
