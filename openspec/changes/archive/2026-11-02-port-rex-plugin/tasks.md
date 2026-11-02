## 1. Port

- [x] 1.1 Add `plugins/rex/manifest.toml` (name `rex`, hooks `on_bind`, `on_session_delete`, `on_session_rename`, `alt-x` `rex_open` bind in `session` context; keep the bind-collision comment in the herdr style)
- [x] 1.2 Add `plugins/rex/rex-plugin` as a bash port of the external zsh plugin (`printf %q` quoting, self-path resolution, `read -n1` in popup mode), executable bit set
- [x] 1.3 Register the plugin in `src/plugin/bundled.rs`
- [x] 1.4 `make check` passes with zero warnings

## 2. Hardening

- [x] 2.1 Hook mode always prints exactly one JSON `{"success": true, ...}` response, even when `rex`/`jq` fail
- [x] 2.2 Use `--autostart=false` for all `rex` calls in `on_session_delete` / `on_session_rename`
- [x] 2.3 Implement `on_session_rename`: find tab by `rename_context.old_name` in the repo's Rex session, `rex window rename` to `new_name`
- [x] 2.4 Match Rex session/tab labels case-insensitively in the `jq` lookups; pass only IDs to mutating `rex` commands
- [x] 2.5 Forward `REX_CLIENT` as `-C` to `rex do session.select` when set
- [x] 2.6 Add `EZ_PLUGIN_DEBUG_LOG` debug logging like herdr's `dbg`

## 3. Manual testing (real Rex, scratch repo)

- [x] 3.1 Pipe crafted JSON to each hook: no `rex` on PATH, Rex server stopped, unknown hook — all return valid JSON success and start no server
- [x] 3.2 Install via `make install-debug` from a plugin dir with a space in its path; `alt-x` on a fresh session creates Rex session + tab and focuses it
- [x] 3.3 `alt-x` again: no duplicate session/tab; `on_enter = "rex"` and `on_create = "rex"` paths behave the same and leave the invoking shell's cwd unchanged
- [x] 3.4 Session names with `/`, spaces and quotes; bare/default session (path = repo root) — spaces only at hook level: git rejects them in branch names
- [x] 3.5 Rename a session with an open tab, then `alt-x`: tab relabelled, no duplicate
- [x] 3.6 Delete from another tab: tab closed; delete the last tab: Rex session killed; delete from inside the tab: tab kept, delete succeeds; delete a never-opened session: no-op
- [x] 3.7 Remote Control disabled: focus prints the error; with `REX_EZ_POPUP=1` it waits for a key
- [x] 3.8 Pre-existing Rex session whose label differs only in case from the repo name is reused
- [x] 3.9 Fix any bug found; add a scenario to the spec for each fix that changes behavior — none found in the plugin

## 4. Docs

- [x] 4.1 README bundled-plugin list, `docs/user-guide.md` (Rex section: setup, Remote Control requirement, `on_enter = "rex"`), `AGENTS.md` bundled list (`docs/plugin-guide.md` has no bundled list, unchanged)
- [x] 4.2 `docs/design.md` diagram if it enumerates bundled plugins — it shows examples only, unchanged

