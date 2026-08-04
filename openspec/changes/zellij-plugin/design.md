## Context

The tmux plugin (`plugins/tmux/tmux-plugin`, 356 lines of bash) is the reference implementation of a multiplexer adapter: it hooks `on_session_create/delete/enter/exit/rename`, `on_bind`, `on_view`, `on_view_select`, and returns `post_shell_commands` that the shell wrapper sources after ez exits. Two things make it more than a plugin, though:

1. **tmux user options as a session registry.** `mark_ez_managed()` stamps `@ez_managed`, `@ez_repo_id`, `@ez_session_id`, `@ez_session_name`, `@ez_session_path` onto the tmux session. The Rust core reads those back in `src/session/current.rs` (current-session resolution) and `src/main.rs` (`cd_to_session`), and the plugin's `on_view` uses `@ez_managed` to mark running sessions.
2. **Terminal teardown.** Killing the multiplexer session you're attached to would SIGHUP the delete hooks, so `spawn_detached_reap` re-execs `ez session reap-delete` under `setsid` with `TMUX=""`.

Zellij 0.44 (verified locally, `zellij 0.44.3`) has the needed primitives:

| tmux | zellij |
|---|---|
| `tmux new-session -d -s N -c DIR -e K=V` | `cd DIR && env K=V zellij attach --create-background N` |
| `tmux has-session -t =N` | `zellij list-sessions -sn \| grep -Fx N` |
| `tmux attach-session -t =N` | `zellij attach N` |
| `tmux switch-client -t =N` | `zellij action switch-session --cwd DIR N` |
| `tmux rename-session -t =OLD NEW` | `zellij --session OLD action rename-session NEW` |
| `tmux kill-session -t =N` | `zellij delete-session --force N` (or `kill-session N`) |
| `tmux set-option -t =N @key val` | **no equivalent** |
| `tmux set-environment -t =N K V` | **no equivalent** |

Probes run against the installed zellij confirmed: `/` is rejected in session names ("Session name cannot contain '/'"), `attach -b` creates a detached session whose panes inherit the launching cwd (verified via `dump-layout` → `cwd "/private/tmp"`), and `zellij --session X action rename-session Y` works from outside the session.

The two missing primitives are the whole design problem: there is no per-session key/value store and no way to mutate a running session's environment.

## Goals / Non-Goals

**Goals:**
- A bundled `zellij` plugin with the same surface as the tmux plugin: create, attach/switch, auto-attach, rename, delete, browser view.
- Current-session detection and `ez cd-to-session` work inside zellij, with no persisted plugin state to go stale.
- Both plugins can be enabled at once without keybind or behavior collisions.
- Zero behavior change for existing tmux users.

**Non-Goals:**
- Sharing bash code between the two plugins. Bundled plugins are single files embedded with `include_str!`; a shared library would need a second bundled artifact and a resolution scheme. Duplication of the shared shape (~60 lines) is the cheaper trade for now.
- Zellij layouts, floating panes, or tab templates on session create.
- Propagating env changes into a running zellij session (impossible without a zellij feature).
- Refactoring the tmux plugin onto the new `ez session list --all --json` command; that is a follow-up.

## Decisions

### 1. Derive session identity from the name instead of stamping metadata

tmux stores `@ez_repo_id`/`@ez_session_name` on the session; zellij cannot. The two options were a sidecar state file per zellij session (`$EZ_CONFIG_DIR/plugins/zellij/state/<name>.json`) or a **deterministic, reversible name encoding**.

Chosen: name encoding. `encode(repo_basename) + "__" + encode(session_name)`, where `encode` replaces every character outside `[A-Za-z0-9_-]` with `_`. Resolution then walks registered repos/sessions and compares the encoded name to `$ZELLIJ_SESSION_NAME`.

Why: no writes, nothing to clean up on rename/delete, no staleness after an `ez` crash or a manual `zellij delete-session`, and the plugin and the Rust core cannot disagree about state — they only have to agree about one pure function. The cost is a theoretical collision (repo `a_b`/session `c` vs repo `a`/session `b__c`); resolution takes the first match and ordering is stable, which is the same weak guarantee tmux's `<repo>/<session>` naming already has.

The `__` separator (not `/`, which zellij rejects, and not `-`, which appears constantly in branch names) keeps the encoding readable in `zellij list-sessions`.

### 2. Generalize current-session resolution behind a small enum, keep tmux first

`src/session/current.rs` grows a third source between the tmux options and the cwd fallback:

```rust
enum CurrentSessionSource {
    Tmux(PathBuf),
    Zellij(PathBuf),   // new
    Worktree(PathBuf),
}
```

`resolve_current_session` tries, in order: tmux `@ez_repo_id`+`@ez_session_name` → tmux `@ez_session_path` → `$ZELLIJ_SESSION_NAME` name match → cwd match. Ordering matters when a user runs zellij inside tmux (or vice versa): the innermost multiplexer wins only if it is tmux, which is the existing behavior for tmux-in-zellij and is fine because both resolve to the same session in practice; nesting the other way is rare and the cwd fallback still covers it.

The zellij branch needs no subprocess — `$ZELLIJ_SESSION_NAME` is exported into every pane — so it is cheaper than the tmux branch (which shells out to `tmux show-options`).

`encode_mux_name()` lives in `current.rs` and is unit-tested; the plugin's bash `encode()` mirrors it. This duplication is the one place the design accepts a two-implementation invariant, so the tests spell out the cases (dots, slashes, colons, spaces, unicode → `_`).

### 3. `cd_to_session` routes through the shared resolver

`src/main.rs::cd_to_session` currently requires `$TMUX` and reads `@ez_session_path` directly. It becomes a thin wrapper over `resolve_current_session(None)` that writes the resolved session's path (falling back to the repo path when `session.path` is `None`). This deletes the duplicate tmux-option reading, makes the command work under zellij and bare shells, and keeps the tmux path first so existing behavior is preserved.

### 4. `ez session list --all --json` + `EZ_BIN` instead of awk-parsing TOML

The tmux plugin's `on_view` re-implements TOML parsing in ~50 lines of awk over `index.toml` and each `sessions.toml`. Rather than copy that, add:

- `EZ_BIN` (from `std::env::current_exe()`) to the plugin process environment in `src/plugin/runner.rs`. Plugins cannot rely on `ez` in `PATH` resolving to the binary — in an interactive shell it's the wrapper *function*, and plugins run with a non-interactive bash.
- `ez session list --all --json`, emitting `[{id, name, path, sessions: [...]}, ...]`. One subprocess for the whole view instead of one per repo (`repo list --json` + N × `session list --json`).

`--all` is rejected together with `--repo`. Non-JSON `--all` prints each repo's tree under a header, which is a small bonus for humans.

### 5. Attach vs switch is decided in the emitted shell command, not by the plugin

Like the tmux plugin, the returned `post_shell_commands` string branches at run time on the user's environment, because the plugin process is not the user's shell:

```sh
if [ -n "$ZELLIJ" ]; then
  zellij action switch-session '<name>'
else
  zellij attach '<name>'
fi
```

Attaching from inside a zellij session is refused by zellij itself, so the branch is mandatory, not cosmetic.

Both branches assume the session already exists: every path that emits this command (`on_bind`, `on_view_select`, `on_session_enter` with `auto_attach`) calls `ensure_session` first, which creates it detached at the right cwd via `attach --create-background`. `switch-session` also accepts `--cwd` and reportedly creates missing sessions, but that could not be verified without hijacking an attached client, so the implementation does not depend on it — cwd is established at creation time instead.

### 6. Delete uses `delete-session --force`, gated by a setting

`zellij kill-session` leaves a resurrectable "EXITED" entry when session serialization is on, which would litter the view with dead names after every `ez session delete`. Default `force_delete = true` runs `zellij delete-session --force <name>` (kills if running, then removes the serialized copy). Users who rely on zellij session resurrection set `force_delete = false` to get kill-only semantics.

### 7. Reaper env clearing and `reap_delay_ms` lookup

`spawn_detached_reap` gains `.env("ZELLIJ", "").env("ZELLIJ_SESSION_NAME", "").env("ZELLIJ_PANE_ID", "").env("TMUX_PANE", "")` alongside the existing `TMUX` clearing. Without this the reaper's zellij plugin invocation would see itself as "inside" the session it is about to delete, and `resolve_current_session` inside the reaper would resolve to the doomed session.

The hard-coded `plugin_settings.tmux.reap_delay_ms` lookup becomes tmux-then-zellij with the same 200 ms default, so zellij-only users can tune it.

### 8. Socket path length: shorten the name to what the path can hold

zellij names one IPC socket per session, `$ZELLIJ_SOCKET_DIR/contract_version_N/<name>`, so the 103-byte `sun_path` limit lands on the session name. Measured on macOS: the default prefix (`$TMPDIR` = a 46-char `/var/folders/...` path) is 79 bytes, leaving **24 bytes** for the name. `hypersonic__type-aware-lint` is 27 and fails. tmux never has this problem because all its sessions share one server socket at `/tmp/tmux-$UID/default`, which is why its plugin can use an unbounded `<repo>/<session>`.

Shortening the *directory* looks like the cheaper fix and is zellij's own documented advice, but pointing `ZELLIJ_SOCKET_DIR` at a *new* directory partitions the user's sessions: probing confirmed that through a different directory zellij falls back to its serialization cache and lists the user's *live* sessions as `EXITED - attach to resurrect`, so attaching would start a duplicate server under the same name.

A **symlink** (`/tmp/zj-<uid>` → `$TMPDIR/zellij-<uid>`) was tried as a way to get a short path to the *same* directory, and shipped briefly. It is wrong, and the reason generalises: `bind(2)` measures the path string it is given, so the socket is created, but every *other* zellij process rebuilds that path from its own environment. A plain shell, the built-in session manager, and the zellij server hosting whatever session the user is currently in all use the default directory, so a name that only fits the short form yields a session **ez can reach and nothing else can** — silently absent from `zellij list-sessions` (it skips names whose path is over the limit rather than erroring), reported dead-and-undeletable by the session manager, and, when the sourced attach command runs `zellij action switch-session` from inside another session, the *server* recomputes the long path and dumps zellij's raw "socket path is too long" error into the user's terminal. Reproduced end to end, which is what retired the approach. Env cannot be retrofitted onto processes ez did not spawn, so no variation of this works.

Chosen: **make the name fit the path in effect**, computed in the plugin as `103 - len(socket_dir) - len(contract_dir) - 2`. Names that fit are left verbatim (so Linux and short names anywhere are unaffected); longer ones become `<encoded-session-prefix>_<4 hex of md5 of the full name>`. The bytes go to the ez session name because that is what the user recognises in zellij's own session list, and the digest is taken over the *full* `<repo>__<session>` so two repos sharing a branch name — and two long branch names sharing a truncated prefix — stay distinct. The earlier objection to shortening (decision 1 derivability) is answered by not re-deriving the budget on the Rust side: `mux_name_matches` accepts any prefix of the encoded session name that carries the right digest, so it matches whatever budget was in force at creation time, on any machine. md5 is already a dependency (`session::cursor`), and `md5sum`/`md5`/`openssl` cover the bash side.

The contract-version directory is read from disk rather than hardcoded, since the number changes with zellij releases; when it does not exist yet the widest two-digit form is assumed, because a budget one byte too generous is exactly what produces an unreachable session. `socket_dir` (or an already-exported `ZELLIJ_SOCKET_DIR`) still wins and simply raises the budget, which is the documented way to keep full-length names — with the namespace split as its stated cost.

The pre-flight budget check does not replace error propagation. `ensure_session` originally discarded the exit code and returned success, so `on_bind` handed the shell an attach command for a session that had never been created; the user saw zellij's error dumped raw by their own shell with nothing tying it to ez. Propagating the failure (`ENSURE_ERROR` + `success: false` for `on_bind`/`on_view_select`, stderr warning + success for lifecycle hooks) surfaces it inside ez, where `apply_bind_response` prints it as a plugin bind failure. A "socket path is too long" error surviving all of this now means the budget itself was wrong, so that branch advises a shorter `socket_dir` and reports the budget it used.

### 9. Keys: `Alt-z` bind, `Ctrl-z` view

Checked against fzf defaults (`.claude/skills/fzf-binds`) and the existing browser map (`Ctrl-t/w/e/o/g/a/d/s/p`, `Alt-n/N/s/r/d/l/a`): `alt-z` and `ctrl-z` are both free. `ctrl-y` was rejected (fzf `yank`), `ctrl-b` was rejected (tmux prefix swallows it before fzf sees it under tmux).

## Risks / Trade-offs

- **[Two implementations of the name encoding diverge]** → The Rust `encode_mux_name` has unit tests enumerating the tricky characters, and the bash side uses one `tr -c 'A-Za-z0-9_-' '_'`-equivalent expression; a mismatch surfaces immediately as "current session not found" in manual verification, which is in the task list.
- **[Name collisions between repo/session pairs]** → First match wins with stable ordering; documented. Same exposure as the existing tmux naming. Shortened names add a 4-hex digest of the full name, so they collide no more readily than the encoding they replace.
- **[A shortened name depends on the socket-path budget, so it changes if `$TMPDIR` or `socket_dir` changes]** → Reverse matching ignores the budget (`mux_name_matches` accepts any prefix carrying the right digest), so detection keeps working; what changes is that the *next* create computes a different name and leaves the old zellij session orphaned under the old one. Same failure mode as renaming a session outside ez, and `zellij delete-session` clears it.
- **[`zellij attach --create-background` requires zellij ≥ 0.40]** → The plugin checks `command -v zellij` and degrades to success/no-op or a clear error; a version older than 0.40 fails the create with a zellij error message written to stderr, visible under `--debug`. Documented as a requirement in the user guide.
- **[Env only applies at zellij session creation]** → Specified and documented; the workaround is deleting the zellij session (or `ez session delete` + recreate) after changing env. tmux users do not hit this because `set-environment` exists.
- **[`ctrl-z` reaching the terminal as SIGTSTP instead of fzf]** → fzf reads keys in raw mode, so it receives `ctrl-z` itself; verified during manual testing of the view. If a user's terminal steals it, the view is still reachable by enabling only one multiplexer plugin and using the `--select-by zellij` flag.
- **[Both plugins enabled, both creating sessions per ez session]** → Harmless but doubles resource use; the user guide recommends enabling one.
- **[Bundled executable bloat]** → One more ~10 KB bash script in the binary; negligible.

## Migration Plan

Additive only. Existing installs pick up the new bundled plugin on the next run (extraction is automatic), but nothing happens until the user runs `ez plugin enable zellij`. `plugins/zellij/*` must be committed with the executable bit set, and `bundled.rs` extraction re-applies `0o755` on write. Rolling back is `ez plugin disable zellij` plus reverting the commit; no on-disk state is created by this change, so there is nothing to clean up.

## Open Questions

- Should `ez session list --all` (non-JSON) become the default for a bare `ez session list` outside a repo? Out of scope here; noted for a later UX pass.
- Whether to migrate the tmux plugin's `on_view` onto `--all --json` once it has proven out in the zellij plugin.
