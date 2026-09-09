## Context

`accept_session` (`src/browser/mod.rs:369-433`) is the single entry point for both `on_enter` and `on_create` when the action names a plugin bind — every create path (`ez session new`, picker alt-n / alt-N / from-dirty) funnels through it. It runs the bind's `OnBind` hook, then:

1. if the response has an effect (`cd_target` or `post_shell_commands`) and no `cd_target`, writes the session path to the cd-file (`:410-412`);
2. if there was no effect, falls back to the same write (`:430`).

The shell wrapper cds only when the cd-file is non-empty (`src/main.rs:163-165`), so "don't cd" needs no wrapper change — just not writing the file.

Note the plain alt-h bind path (`apply_bind_response`) already cds only on an explicit `cd_target`; the bug is specific to the `on_enter`/`on_create` path.

## Goals / Non-Goals

**Goals:**
- One opt-out signal in the hook protocol, honored in the one shared place, so any plugin that navigates elsewhere is fixed — not just herdr.
- Zero behavior change for plugins that don't set it.

**Non-Goals:**
- Reworking `cd_target`/`post_shell_commands` semantics.
- A user-facing config knob for this — the plugin knows whether it navigated; the user shouldn't have to.

## Decisions

**`keep_cwd: bool` on `HookResponse`, default false.** `#[serde(default)]`, and `skip_serializing_if = "std::ops::Not::not"` to keep serialized responses unchanged. Honored in `accept_session` only: when true, skip both the implicit and the fallback `write_cd_target`; still write `post_shell_commands` + session env exports.

- *Alternative — drop the implicit cd whenever `post_shell_commands` is present.* Smaller diff and arguably the honest reading of the protocol, but it silently changes tmux/zellij: after detaching you'd land in the old directory instead of the worktree. Rejected as collateral damage.
- *Alternative — herdr returns `cd_target: $PWD` (a no-op cd).* Zero ez changes, but it's a per-plugin trick that reads as a bug to the next person, and the next plugin with a separate UI hits the same trap.

**Only the `INSIDE_HERDR` branch sets it.** Inside herdr, `open_cmd` focuses another workspace and the invoking shell keeps running where it is — cd is wrong. Outside herdr, `open_and_attach_cmd` attaches herdr in this terminal, so the cd is invisible while attached and useful after detach — leave it.

**Env exports are unaffected.** They go through `post_shell_commands`, not the cd-file, so `keep_cwd` does not lose `ez_pr_*` and friends.

## Risks / Trade-offs

- [A plugin sets `keep_cwd` but its navigation fails, leaving the user nowhere] → herdr only sets it behind `guard` (herdr on PATH + server running); when the guard fails the plugin returns no navigation effect and ez's fallback cd applies as before.
- [Protocol surface grows by a field for one current consumer] → it is one bool with a false default; the alternative is per-plugin workarounds in every plugin that opens its own UI.
- [`on_enter = "herdr"` from a plain terminal still cds] → intended: that branch attaches herdr in the current terminal.
