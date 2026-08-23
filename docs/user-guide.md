# User Guide

## Installation

```bash
# From git URL
cargo install --locked --git https://github.com/user/ez-workspaces.git

# From a cloned repo
git clone https://github.com/user/ez-workspaces.git
cd ez-workspaces
cargo install --locked --path .
```

### Shell Integration

Add to your shell RC file:

```bash
# ~/.zshrc or ~/.bashrc
eval "$(ez init-shell zsh)"
```

```fish
# ~/.config/fish/config.fish
ez init-shell fish | source
```

This creates a shell wrapper that enables `cd`-on-enter when you select a session and runs post-exit commands from plugins (e.g., tmux attach).

### Shell Completions

```bash
# Zsh
ez completions zsh > ~/.zfunc/_ez

# Bash
eval "$(ez completions bash)"

# Fish
ez completions fish > ~/.config/fish/completions/ez.fish
```

## First Steps

### 1. Configure workspace roots

Run the interactive setup:

```bash
ez config
```

This guides you through workspace roots, shell, selector, plugins, and timeout.

Or configure individually:

```bash
ez config add-root ~/workspace/personal
ez config add-root ~/workspace/work
ez config set default_shell zsh
```

### 2. Register repos

```bash
# Register current directory
ez add

# Register a specific path (git repo or plain directory)
ez add ~/workspace/personal/my-project

# Clone and register
ez clone git@github.com:user/repo.git
```

`ez add` works on non-git directories too — sessions become directory bookmarks without worktree management.

### 3. Enable plugins

Built-in plugins are bundled in the binary and auto-extracted on first use:

```bash
ez plugin enable git-worktree
ez plugin enable tmux

# Cursor IDE integration (enable all three for full support)
ez plugin enable cursor-mcp-auth           # Share MCP OAuth tokens across worktrees
ez plugin enable cursor-trusted-workspace  # Auto-trust worktree workspaces
ez plugin enable cursor-mcp-approvals      # Auto-approve MCP servers in worktrees
ez plugin enable kv                        # Per-session KoalaVim environments
```

### 4. Create and use sessions

```bash
# Create a session (with git-worktree plugin, this creates a worktree)
# New sessions are created as children of the default (main) session unless --parent is specified
ez session new feature-login

# Create a child session under a specific parent
ez session new api-tests --parent feature-login

# Force the interactive name builder even when passing a name
ez session new my-name --interactive

# List sessions (tree view with box-drawing connectors)
ez session list
# main *
# ├── feature-login
# │   └── api-tests

# Register an existing worktree as a session (defaults to current directory and branch name)
# Also defaults under main unless --parent is specified
ez session register /path/to/worktree

# Enter a session (cd's to worktree by default; see on_enter below)
ez session enter feature-login

# Delete a named session (cascades with --force)
ez session delete feature-login --force

# Delete the current session (detects tmux @ez_session_name or current worktree, then prompts)
ez session delete
```

If `ez session new <name>` finds that the branch is already checked out in another git worktree, the git-worktree plugin reports the existing path and suggests `ez session register <path> --name <name>`. Registered worktrees are treated as session worktrees, so deleting that session runs the normal worktree cleanup hook.

#### Unmanaged worktrees ("Not Registered")

The session picker automatically detects git worktrees that exist on disk but aren't tracked as ez sessions. These appear below your managed sessions under a **Not Registered** header:

```
main ★
├── feature-auth
└── fix-typo

Not Registered
  old-experiment → /path/to/old-experiment
  stale-branch   → /path/to/stale-branch
```

Selecting a non-registered worktree **registers it as a session** (under `main`) and enters it in one step. On the next render, it appears as a normal managed session.

Pressing the **delete** keybind on a non-registered worktree removes it with `git worktree remove --force` (after confirmation). If the worktree has uncommitted changes, the confirmation prompt warns you. The associated branch is kept.

This is useful for discovering worktrees created outside of ez (e.g. via `git worktree add`), orphaned worktrees from failed cleanups, or worktrees created by other tools.

#### Interactive session naming

When you create a new session *without* passing a name (`ez session new` with no
arg, or `Alt-n` in the browser), ez first presents a **mode picker** (unless
only one mode is configured):

| Mode | Description |
|------|-------------|
| **Full name** | Type the entire session name directly |
| **Build from parts** | Step through configured stages (prefix → ticket → name) |
| **From GitHub PR** | Paste a GitHub PR URL — extracts `pr<number>` and optionally resolves the branch name via the `OnNameResolve` plugin hook |
| **From Jira URL** | Paste a Jira URL — extracts `PROJ-123` then prompts for an optional suffix |

Use `--interactive` / `-i` to force the mode picker even when a name is provided
on the CLI: `ez session new my-name --interactive`.

Configure which modes are available:

```toml
name_builder_modes = ["full_name", "build_from_parts", "github_pr", "jira_url"]
```

##### Build from parts mode

In "Build from parts" mode, ez walks you through a short staged prompt and
joins the parts with `-`:

Stages come in two kinds:

- **`choice` (default)** — fzf list with the configured choices plus a
  `(none)` row. You can pick a choice, type a custom value and Enter to use
  it (when the typed text doesn't match any item), or pick `(none)` to skip
  the part.
- **`text`** — skips the fzf list and goes straight to a text prompt. Empty
  input is treated like `(none)` (the part is skipped).

Once you've picked at least one part, each subsequent stage shows the
name-so-far (e.g. `feat-ABC-`) as a header above the keybind hints so you can
see the name taking shape as you go.

`Ctrl-P` goes back to the previous stage in either kind; `Esc` cancels. The
final descriptive-name stage is implicit (always added), text-mode, and
cannot be empty.

`(none)` parts contribute nothing to the joined name. Default stages produce
names like `feat-PROJ-123-add-login-button`:

```toml
# in ~/.config/ez/config.toml — these are the defaults
[[session_name_stages]]
name = "prefix"
kind = "choice"
choices = ["feat", "fix", "chore"]

[[session_name_stages]]
name = "ticket-prefix"
kind = "choice"
choices = []  # add e.g. ["JIRA", "PROJ"]; empty just shows (none) — type your prefix and Enter

[[session_name_stages]]
name = "ticket-number"
kind = "text"  # skips fzf, prompts for free text directly
```

> Stage order is the order of `[[session_name_stages]]` blocks in the file.
> Move a block up or down to reorder the prompts. `kind` defaults to
> `"choice"` if omitted.

Passing a name on the CLI (`ez session new my-name`) skips the staged prompt
entirely. The default `main` session is also unaffected — it's always named
`main`.

#### Branch-name collision prompt

When the session name (however it was determined) matches an **existing local git
branch**, ez pauses and asks how you want to proceed:

```
Branch 'feature-login' already exists.
  [N] use the existing branch  (default)
  [y] recreate from the latest base (origin/main or parent) — discards 'feature-login'
Recreate? [y/N]:
```

- **Press Enter (or N)** — the existing branch is checked out into the new worktree as-is.
  Its commits, stashes, and history are preserved.
- **Type `y`** — the branch is deleted and re-created from the latest base (same start
  point the git-worktree plugin would use for a brand-new branch: `origin/main`,
  `origin/master`, or the parent session's HEAD). All previous commits on that branch
  are discarded.

In a non-interactive context (e.g. piped stdin), the prompt receives EOF and defaults
to **reuse**, so existing scripts keep working without modification.

### Bare sessions

Create a session without a worktree:

```bash
ez session new placeholder --bare
```

Or press **Alt-Shift-N** in the browser. Bare sessions are useful as bookmarks or placeholders — they appear in the tree with a `[bare]` indicator. The git-worktree plugin skips worktree creation for bare sessions.

### Session from dirty changes

Move uncommitted changes from the current session to a new one:

```bash
ez session from-dirty new-feature
```

Or press **Alt-s** in the browser. This stashes your uncommitted changes, creates a new session on the same commit, and applies the stash in the new session's worktree.

### Session notes

Each session can have a notes directory with a `README.md` for tracking context, TODOs, or anything else:

```bash
# Open the session's README.md in $EDITOR
ez session note open --name feature-auth

# Cd into the notes directory
ez session note cd --name feature-auth

# Print the notes directory path (for scripting)
ez session note path --name feature-auth

# Omit --name to use the current session
ez session note open
```

In the browser, press **Alt-i** to open a session's note in your editor, or **Alt-I** to cd into its notes directory. Notes are shown in the preview pane when present (rendered via `bat`).

Configure the open command in `config.toml`:

```toml
note_open_command = "$EDITOR"   # default, resolves from environment
# note_open_command = "code"    # or use a specific editor

[keybinds]
note_open = "alt-i"   # default
note_cd = "alt-I"     # default
```

Notes are stored in the OS data directory (`~/.local/share/ez/` on Linux, `~/Library/Application Support/ez/` on macOS) and are cleaned up when a session is deleted.

### 5. Browse interactively

Run bare `ez` to get an fzf-powered browser:

1. Select a workspace root
2. Drill into directories (repos show `[branch]`)
3. Press **Alt-a** to clone a new repo into the current directory, or select a repo to see its sessions
4. Select a session to enter it

At any top-level selector, press a keybind to switch views:

- **Ctrl-t** — Tree view: all workspaces → repos → sessions in one tree
- **Ctrl-w** — Workspace view (default): root → drill → session picker
- **Ctrl-e** — Repo view: flat list of every registered repo
- **Ctrl-o** — Owner view: repos grouped by GitHub-style owner (parsed from remote URL)
- **Ctrl-g** — Label view: items grouped by user-defined labels
- **Ctrl-a** — Tmux view (plugin): ez-managed tmux sessions — select to attach/switch
- **Ctrl-z** — Zellij view (plugin): all ez sessions with their zellij state — select to attach/switch

Plugin views appear automatically when enabled plugins register them. The tmux plugin adds `Ctrl-a`, the zellij plugin adds `Ctrl-z`.

In the repo view:

- **Alt-l** — Edit labels on the selected repo
- **Alt-d** — Remove the selected repo from ez
- **Ctrl-s** — Toggle sort (alphabetical / LRU)

### Jumping back to a session's worktree

From any pane inside a multiplexer session created by the tmux or zellij plugin, return to the session's worktree with:

```bash
ez cd-to-session
```

This resolves the current session — from the tmux `@ez_session_path` user option, from the zellij session name, or from the working directory — and `cd`s your shell to its worktree (via the shell wrapper installed by `ez init-shell`). Useful after navigating elsewhere or when opening a new pane that didn't inherit the cwd.

> Under tmux, the option is only set when the tmux session is created by the plugin (on `session new`, the `Ctrl-a` bind, or the tmux view). Pre-existing sessions created before this feature won't have it — recreate them or trigger `Ctrl-a` from the picker to stamp it. Under zellij nothing is stamped: identification comes from the session's name, so any session the plugin created is recognized.

Inside the session picker:

- **Alt-n** — New child session
- **Alt-Shift-N** — New bare session (no worktree)
- **Alt-s** — Session from dirty (move uncommitted changes to new session)
- **Alt-r** — Rename session
- **Alt-d** — Delete session
- **Alt-a** — Attach to the session's tmux session (plugin)
- **Alt-z** — Attach to the session's zellij session (plugin)
- **Alt-l** — Edit labels (comma-separated, prefix `-` to remove, e.g. `wip, -stale`)
- **Ctrl-d** — Cd into session worktree (bypasses on_enter action like tmux)
- **Ctrl-s** — Toggle sort (alphabetical / LRU)

You can also launch a specific view directly: `ez --select-by repo`, `ez --select-by label`, etc. To change the default view, set `default_select_by = "repo"` in your config (or run `ez config set default_select_by repo`).

Repos and sessions are sorted by last-accessed time (LRU) by default. Press **Ctrl-s** to toggle between LRU and alphabetical sort. To change the default:

```toml
default_sort = "lru"    # lru | alpha
```

### Configuring what Enter does (`on_enter`)

By default, pressing **Enter** on a session (or running `ez session enter <name>`) **cd's into the session's worktree**. You can change this to any session plugin-bind by name:

```bash
# Attach to (or create) the tmux session instead of cd-ing
ez config set on_enter tmux

# Override per-invocation (overrides config)
ez --on-enter tmux
ez --on-enter cd          # force cd even if config says tmux
```

`on_enter` is matched against a session plugin-bind's **label**, **bind name**, or **plugin name** — so `"tmux"` resolves to the tmux plugin's `tmux_attach` bind (the same action as pressing **Alt-a** in the picker). If the named bind is unavailable (plugin disabled, tmux not installed), ez silently falls back to `cd`.

Set it in `~/.config/ez/config.toml`:

```toml
on_enter = "tmux"    # cd | tmux (or any session plugin-bind label/name)
on_create = "tmux"   # none | cd | tmux (or any session plugin-bind label/name)
```

### Configuring what happens after creating a session (`on_create`)

By default, creating a session (picker **Alt-n** or `ez session new <name>`) just creates it and does nothing else. You can make it immediately jump in:

```bash
# After creating a session, cd into its worktree
ez config set on_create cd

# After creating a session, attach to (or create) its tmux session
ez config set on_create tmux

# Per-invocation override
ez --on-create tmux session new my-feature
```

In the interactive picker, when `on_create` is set, **Alt-n** creates the session, performs the action, and exits (just like pressing Enter on an existing session). With `"none"` (default) it stays in the picker as it does today.

If the named bind is unavailable (plugin disabled, tmux not installed), ez silently falls back to `cd`.

### Zellij Plugin

The `zellij` plugin is the zellij counterpart of the tmux plugin: every ez session gets a zellij session named `<repo>__<session>` (shortened when zellij's socket path can't hold it, see below), created detached at the session's worktree.

```bash
ez plugin enable zellij
```

Requires `zellij` **0.40 or newer** on your `PATH` (for `attach --create-background` and `action switch-session`). If zellij is missing, lifecycle hooks silently no-op — session creation is never blocked.

- **Alt-z** in the session picker — create if needed, then attach (or switch, when you're already inside zellij)
- **Ctrl-z** in any other view — list every ez session with `●` (zellij session running) or `○` (not running); selecting one attaches to it
- **Rename** propagates to the zellij session; **delete** kills it (and by default removes zellij's serialized copy so dead names don't linger)

**Configuration:**

```toml
[plugin_settings.zellij]
auto_attach = false     # attach/switch automatically when entering a session
force_delete = true     # also delete zellij's resurrectable copy on session delete
socket_dir = ""         # ZELLIJ_SOCKET_DIR for ez's zellij sessions (see "name length")
reap_delay_ms = 200     # delay before delete hooks run in the detached reaper
```

Set `on_enter = "zellij"` (or `on_create = "zellij"`) to make Enter attach instead of `cd`.

**Naming:** unlike tmux, zellij has no per-session option store, so a session is identified purely by its name. Every byte outside `[A-Za-z0-9_-]` becomes `_`, and the two parts are joined with `__` — repo `my.repo` + session `feat/ABC-1` becomes `my_repo__feat_ABC-1`. `ez cd-to-session` and current-session auto-detection (e.g. `ez session delete` with no name) work off that name, so nothing goes stale after a crash or a manual `zellij delete-session`.

**Session name length (handled automatically):** tmux multiplexes every session over one server socket, so tmux session names have no length limit. zellij instead creates one socket *per session*, named after the session, under `$ZELLIJ_SOCKET_DIR/contract_version_N/` (default `$TMPDIR/zellij-$UID`). A unix socket path can be at most 103 bytes, and macOS's `$TMPDIR` is a ~46-character `/var/folders/...` path, leaving only about **24 bytes** for the name — not enough for `acme-widgets__refactor-auth-flow`.

Names that don't fit are shortened: the repo prefix is replaced by a 4-hex digest of the full name, and the session name is truncated if it is still too long.

```
acme-widgets + main               → acme-widgets__main            (fits, kept as-is)
acme-widgets + refactor-auth-flow → refactor-auth-flow_7239
acme-widgets + feat-ABC-123-add-dark-mode-toggle → feat-ABC-123-add-da_eb18
```

The readable part is the ez session name, which is what you pick from in zellij's own session list; the digest covers the repo *and* the untruncated name, so two repos sharing a branch name — and two long branch names sharing a prefix — stay distinct. `ez cd-to-session` and current-session detection understand both forms, so nothing depends on knowing which one a session got.

Shortening happens because *reachability* depends on it, not for tidiness: every zellij process builds the socket path from its own environment — a plain `zellij attach`, the built-in session manager, the server hosting the session you're currently in. A name that only fits some shorter path yields a session ez can reach and nothing else can: missing from `zellij list-sessions`, shown as dead by the session manager, impossible to attach to or delete.

To keep full-length names, give zellij a short socket directory, either for ez's sessions only:

```toml
[plugin_settings.zellij]
socket_dir = "/tmp/zellij-1000"   # your uid
```

or globally, in your shell rc (`export ZELLIJ_SOCKET_DIR=/tmp/zellij-$(id -u)`), which ez respects and which every other zellij client picks up too. Either way the budget grows and names are left verbatim. Note that a `socket_dir` different from zellij's default puts ez's sessions in a **separate namespace** from sessions started by a plain `zellij` — that is the trade-off for keeping the names.

**Limitation:** zellij has no equivalent of `tmux set-environment`, so session env vars are applied only when the zellij session is created. If a session's env changes later, recreate its zellij session to pick up the new values.

Both multiplexer plugins can be enabled at once — their keys don't overlap (`Alt-a`/`Ctrl-a` for tmux, `Alt-z`/`Ctrl-z` for zellij) — but each session then gets both a tmux and a zellij session, so most people enable just one.

### KoalaVim (kv) Plugin

The `kv` plugin gives each session its own isolated KoalaVim environment. When you create a session, the plugin forks the `main` kv env so the editor gets a separate config, cache, and state directory. Requires the `kv` CLI to be installed.

```bash
ez plugin enable kv
```

On session create, the plugin runs `kv env fork main <session-name>`. On enter, it sets `KV_ENV=<session-name>` so `kv` uses the right environment. On delete, it cleans up with `kv env delete`. On rename, it runs `kv env rename`.

If `kv` is not installed, the plugin silently no-ops — it won't block session creation.

**Configuration:**

```toml
[plugin_settings.kv]
source_env = "main"                            # which kv env to fork from (default: "main")
repos = "KoalaVim, my-editor"    # only activate on these repo names (empty = no repos)
```

The `repos` field is required to activate the plugin — set it to a comma-separated list of repo directory names (the folder name, not the full path). If empty, the plugin skips all repos.

## Labels

Tag any repo or session to group and filter them.

```bash
# Add labels
ez repo label add my-repo backend core
ez session label add feature-x --repo my-repo wip

# Remove labels
ez repo label remove my-repo core

# List
ez repo label list              # all labels grouped
ez repo label list my-repo      # labels on one repo
ez repo list --label backend    # filter repo list

# Browse by label
ez --select-by label
```

Labels on the currently selected item can also be edited interactively in the browser by pressing **Alt-l**. Labels are stored in the repo's metadata (`~/.config/ez/repos/<id>/repo.toml`) and in per-session metadata (`sessions.toml`).

## Session Hierarchy

Sessions form a tree. New sessions are created as children of the default (main) session unless `--parent` is specified. Use `--parent` to nest under a different session:

```
main *                    # auto-created default
├── feature-auth          # child of main (default)
│   ├── backend-api       # child of feature-auth
│   └── frontend-ui       # child of feature-auth
└── bugfix-crash          # child of main (default)
```

The default "main" session is auto-created when you first access a repo. It points to the repo's working directory. Box-drawing connectors (tree glyphs) show parent-child relationships in `ez session list` and in the session picker.

### Return-to-ez after multiplexer detach

When `on_enter` is set to `tmux` or `zellij`, the shell wrapper automatically re-enters the ez browser after you detach from the multiplexer session (`Ctrl-b d` in tmux, `Ctrl-o d` in zellij). This creates a seamless workflow loop: browse → attach → work → detach → browse again. No additional config needed — the loop is driven by the attach command returning control, so it works the same for either multiplexer.

## Non-git Sessions

Sessions work without git. Without the git-worktree plugin, sessions are purely virtual — just metadata with a name, parent relationships, and environment variables. This is useful for organizing work contexts even in non-git projects.

## Colored Output

All output is colored by default. To disable:

```bash
ez --no-color session list
```

Or set the `NO_COLOR` environment variable (respected automatically).

## Escape / Back Navigation

In interactive menus (browsing directories, config wizard), pressing **Escape** goes back to the previous level instead of quitting. At the top level, Escape exits.

## Environment Variables

Sessions can carry environment variables. Plugins may set them (for example PR metadata), and you can manage them directly with `ez session env`:

```bash
# Set (or overwrite) a variable on the current session
ez session env set AWS_PROFILE staging

# Target a specific session / repo
ez session env set AWS_PROFILE staging --session feature-x
ez session env set AWS_PROFILE staging --session feature-x --repo my-repo

# List as KEY=VALUE lines (omit --session to use the current session)
ez session env list
ez session env list --session feature-x --json

# Remove a variable (succeeds silently if it was not set)
ez session env unset AWS_PROFILE --session feature-x
```

If no current session can be detected, pass `--session <name>`. Env vars set this way are stored on the session (same map plugins write to) and exported when you enter the session.

Keys prefixed with `ez_` are typically managed by ez itself (for example `ez_pr_number`). You can override them, but they may be rewritten later.

**Limitation:** zellij applies session env only when the zellij session is created. After changing env vars, recreate that zellij session to pick up the new values. For tmux, re-enter the session.
