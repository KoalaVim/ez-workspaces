## Context

The workspace view's `drill_into_directory` function lets users browse workspace root directories level by level. Currently it uses `select_one` which only supports Enter (select) and Escape (back). There is no way to perform actions (like cloning) while browsing directories.

The `repo::clone_repo` function already handles cloning a git URL into a target path, registering it, and detecting metadata. The missing piece is an interactive trigger from within the browser.

## Goals / Non-Goals

**Goals:**
- Let users clone a repo into the currently browsed directory without leaving the browser.
- Transition directly into the cloned repo's session picker after a successful clone.
- Follow existing patterns: configurable keybind, preview pane help, colored output.

**Non-Goals:**
- Supporting clone from views other than workspace drill-down (Repo view, Owner view, etc.) — those don't browse filesystem directories.
- Supporting clone of non-git repos (e.g. svn).
- Adding a "clone from GitHub search" feature — the user provides the full URL.

## Decisions

### Decision 1: Convert `drill_into_directory` from `select_one` to `select_with_actions`

The current drill-down uses `select_one` which doesn't support action keybinds. To add a clone keybind, we need to switch to `select_with_actions` and handle the action result. This is the same pattern used by the session action loop and the repo view.

**Alternative**: Add a separate fzf `--bind` flag to spawn an inline shell command. Rejected because it bypasses the Rust control flow, can't transition to the session picker after clone, and doesn't integrate with the keybind config system.

### Decision 2: Use `alt-a` as the default keybind

`alt-a` is mnemonic for "add" (clone and add a repo). It doesn't conflict with any existing keybind in the drill-down context. The keybind is only registered when in the directory browser, not in session or repo views.

**Alternative**: `alt-c` — already used for `cd_session`. `ctrl-n` — could conflict with fzf's built-in next-line. `alt-a` is free and intuitive.

### Decision 3: Clone target directory is the currently browsed directory

When the user presses the clone keybind, the URL prompt appears. The repo is cloned into a subdirectory of the currently browsed directory (same behavior as `ez clone <url>` — the directory name is derived from the URL). This matches the mental model: "I'm looking at `~/workspace/personal/` and I want to clone a repo here."

### Decision 4: Prompt via `selector.input` for the URL

Reuse the existing `InteractiveSelector::input` method to prompt for the git URL. This keeps the UX consistent with other fzf-based prompts (rename, labels). No default value is provided since URLs aren't predictable.

### Decision 5: Run `git clone` synchronously with visible output

The clone operation shells out to `git clone` (via `repo::clone_repo`). Since this can take time, the user should see git's progress output. The existing `clone_repo` already uses `.status()` which inherits stdio, so progress is visible.

## Risks / Trade-offs

- **[Long clone blocks the browser]** → Acceptable since `git clone` progress is visible and the user expects to wait. Ctrl-C will kill the child process and return to the browser.
- **[Invalid URL wastes time]** → Git clone fails quickly on invalid URLs and the error is displayed. The browser re-renders the directory listing so the user can try again.
- **[Keybind collision in future]** → Mitigated by making it configurable in `KeybindsConfig`, same as all other keybinds.
