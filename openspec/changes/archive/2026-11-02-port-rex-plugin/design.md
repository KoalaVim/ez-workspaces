## Context

Source: an external zsh plugin (~90 lines, `on_bind` + `on_session_delete`). It is meant to be driven by `on_enter = "rex"` / `on_create = "rex"`, and supports running ez from a Rex popup via `REX_EZ_POPUP=1`, where it holds errors on screen before the popup closes.

Rex model: sessions contain windows (shown as tabs) contain blocks (panes). Lookup is by label via `rex ls --json` / `rex window ls -s <sid> --json`, because `rex new` / `rex window new` report `label: null` in their JSON output. Every `rex` command defaults to `--autostart=true`, i.e. it spawns a local server if none is running.

## Goals / Non-Goals

**Goals:**
- Same user-visible behavior as the external plugin, shipped as a bundled plugin.
- Fix the robustness gaps listed in the proposal, found and verified by hands-on testing in a real Rex.

**Non-Goals:**
- `on_attached_sessions` indicator, `on_session_enter`/`on_session_create` hooks, config schema options — add later if wanted.
- Disambiguating two repos with the same name (see Risks).
- Changing ez core (no Rust changes beyond the bundled list).

## Decisions

**Port to bash.** All other bundled plugins are bash; zsh isn't guaranteed on Linux. zsh-isms map directly: `${(q)x}` → `printf %q`, `${0:A}` → resolve `$0` with `cd "$(dirname "$0")" && pwd -P`, `read -k1` → `read -n1 -r`. Alternative (keep zsh) rejected for consistency.

**Keep the `focus` sub-command handed back as a post-shell command.** The work runs in the user's shell so its errors land on the terminal (and the popup can hold them), not in ez's plugin pipe; it also keeps the hook fast.

**Match labels case-insensitively, act by ID.** Rex resolves label targets case-insensitively and lets two things share a label (ambiguous targets exit 3). So the plugin looks up labels itself in `--json` listings (case-insensitive, first match) and passes IDs to every mutating command, never labels.

**`--autostart=false` on cleanup paths.** Delete/rename must not spawn a Rex server just to discover there's nothing to clean. The focus path keeps autostart: the user explicitly asked to open a tab.

**JSON on every exit path.** Hook mode drops `set -e` reliance for rex/jq calls (each guarded with `|| true` / empty checks) and prints the response from a single place, so a failing `rex` can't leave ez with empty stdout. `focus` mode keeps `set -eu` since it reports to the terminal.

**Rename via `rex window rename -s <sid> <wid> <new>`**, found by `rename_context.old_name` in the repo's Rex session. Without it, the next `alt-x` creates a second tab and the old one is orphaned (delete would then miss it too).

## Risks / Trade-offs

- [Two repos with the same name (e.g. different owners) share one Rex session] → accepted; matches the external plugin's behavior. Revisit with an owner-qualified label if it bites.
- [Rex CLI JSON shape changes] → all parsing goes through `jq` with `// empty` fallbacks, so shape drift degrades to a no-op on cleanup and a visible error on focus.
- [Switching requires Rex.app Settings > Rex Server > Remote Control] → focus step prints a clear error; documented in user guide.
- [`session.select` is an app action; with several app clients connected, `rex do` without `-C` may target the wrong one or fail] → pass `-C "$REX_CLIENT"` when set; otherwise accept Rex's default and surface its error. Verify during manual testing.

