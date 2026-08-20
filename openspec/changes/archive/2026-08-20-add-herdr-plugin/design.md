## Context

ez-workspaces bundles multiplexer plugins (tmux, zellij) that map ez session lifecycle events to multiplexer-native session/workspace management. herdr is another workspace manager that organizes worktrees into persistent workspaces with panes and agents. The herdr plugin already exists as a working implementation in the contributor's dotfiles and follows the same thin-integration pattern as the existing multiplexer plugins.

The plugin source is at `~/.dotfiles/configs/ez-workspaces/plugins/herdr/` and consists of two files: `manifest.toml` and `herdr-plugin` (a bash executable).

## Goals / Non-Goals

**Goals:**
- Bundle the herdr plugin so it ships with ez-workspaces and is auto-extracted like tmux and zellij
- Provide session lifecycle integration: open/focus herdr workspace on enter, close on delete, rename on rename
- Expose `alt-h` keybind for opening herdr workspace from the session picker
- Expose `auto_open` and `close_workspace_on_delete` config options

**Non-Goals:**
- Adding an `on_view` / view integration for herdr (herdr manages its own workspace list)
- Creating worktrees from within herdr — ez owns worktree creation; herdr discovers them as plain git worktrees
- Modifying the plugin system or any existing plugins

## Decisions

**Copy the plugin files verbatim into `plugins/herdr/`.**
The plugin is already production-tested in the contributor's dotfiles. The manifest and executable follow the established plugin conventions (JSON-over-stdio, `jq` for parsing, graceful degradation when herdr is absent). No adaptation is needed beyond placing them in the repo.

**Use `alt-h` as the keybind.**
The manifest comment documents key allocation: `alt-a` is taken by tmux, `alt-z` by zellij, and `alt-h` is free. The mnemonic ("h" for herdr) is clear and avoids collisions with core and existing plugin binds.

**No `mutates_session_path` or priority.**
Unlike git-worktree, herdr does not modify the session path — it only opens/focuses an external workspace. The plugin runs after path-mutating plugins by default, which is correct since it needs the resolved `session.path`.

## Risks / Trade-offs

- **[herdr CLI not installed]** → The plugin guards every hook with `herdr_available()` and `server_running()` checks; when herdr is absent or stopped, it returns `{success: true}` and does nothing. No risk to users without herdr.
- **[SIGPIPE in server check]** → The plugin deliberately avoids `herdr status | grep -q` under `set -o pipefail` to prevent false negatives from SIGPIPE; it captures output to a variable instead.
- **[Worktree already deleted on session delete]** → The git-worktree plugin runs before herdr and may remove the worktree directory. The plugin has a `workspace_by_checkout` fallback that queries herdr's workspace list by `checkout_path` even when the directory is gone.
