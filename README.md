<p align="center">
<img width="256" height="256" src="https://github.com/user-attachments/assets/b77cae31-5043-480d-9142-a3214803191d" />
</p>

# ez-workspaces

A fast, plugin-extensible workspace and session manager for git repos. Browse, create, and switch between worktree-based sessions with a single command.

<!-- TODO: Add demo GIF here -->
<!-- ![ez demo](assets/demo.gif) -->

## Why

Switching between tasks means juggling git branches, stashing changes, and losing editor context. **ez** turns git worktrees into managed "sessions" — each task gets its own directory, branch, and terminal, so you never lose your place. An interactive fzf browser ties it all together.

## Install

```bash
cargo install --locked --git https://github.com/user/ez-workspaces.git
```

Add shell integration:

```bash
echo 'eval "$(ez init-shell zsh)"' >> ~/.zshrc
# or for bash:
echo 'eval "$(ez init-shell bash)"' >> ~/.bashrc
```

For PowerShell, add to your `$PROFILE`:

```powershell
Invoke-Expression ((& ez init-shell pwsh) -join "`n")
```

## Quick Start

```bash
ez plugin enable git-worktree    # worktree-backed sessions
ez plugin enable tmux            # tmux integration (optional)
ez plugin enable zellij          # zellij integration (optional)
ez config add-root ~/workspace   # register a workspace directory
ez add ~/workspace/my-project    # register a repo
ez                               # launch the interactive browser
```

From the browser, select a repo to see its sessions. Press `Alt-n` to create a new session — ez creates a worktree and drops you in.

## How It Works

Sessions are tree-structured metadata. Plugins give them physical meaning:

- **git-worktree** — creates/deletes worktrees on session create/delete
- **tmux** — creates tmux sessions, auto-attach on enter, `Ctrl-a` to browse tmux sessions
- **zellij** — same for zellij: a session per workspace session, `Ctrl-z` to browse them
- **rex** — opens sessions as [Rex](https://www.superlogical.com/rex) tabs: one Rex session per repo, one tab per ez session

Worktrees live as siblings of the repo in a `.ez/` directory:

```
~/workspace/
  my-repo/                    # original repo
  .ez/my-repo/
    feature-auth/             # worktree session
    bugfix-crash/             # worktree session
```

Non-git directories can also be tracked — sessions become bookmarks without worktree management.

## Browser

Launch with `ez`. The preview pane shows all available keybinds.

**Views** — switch with keybinds anytime:

- `Ctrl-t` Tree · `Ctrl-w` Workspace · `Ctrl-e` Repo · `Ctrl-o` Owner · `Ctrl-g` Label

**Workspace drill-down** — while browsing directories:

- `Alt-a` clone a repo into the current directory

**Repo view actions:**

- `Alt-l` labels · `Alt-d` remove · `Ctrl-s` sort

**Session actions** — inside a repo's session picker:

- `Enter` enter · `Alt-n` new · `Alt-N` bare · `Alt-s` from dirty · `Alt-r` rename · `Alt-d` delete · `Alt-l` labels · `Ctrl-d` cd · `Ctrl-s` sort · `Alt-i` note · `Alt-I` note cd

`Esc` goes back one level; `Ctrl-C` quits from anywhere. All keybinds are [configurable](docs/user-guide.md#keybinds).

## Plugins

Bundled plugins — just enable:

```bash
ez plugin enable git-worktree        # worktree lifecycle
ez plugin enable tmux                # tmux session management
ez plugin enable zellij              # zellij session management
ez plugin enable rex                 # Rex tab per session
ez plugin enable cursor-mcp-auth     # share Cursor MCP OAuth tokens across worktrees
ez plugin enable cursor-trusted-workspace  # auto-trust worktree workspaces in Cursor
ez plugin enable cursor-mcp-approvals      # auto-approve MCP servers in Cursor
ez plugin enable kv                        # per-session KoalaVim environments
```

Custom plugins use a JSON-over-stdio protocol. See the [Plugin Guide](docs/plugin-guide.md).

## Session environment

Set, list, or unset per-session environment variables (exported when you enter a session):

```bash
ez session env set AWS_PROFILE staging          # current session
ez session env set AWS_PROFILE staging --session feature-x
ez session env list
ez session env list --json
ez session env unset AWS_PROFILE
```

## Suggested Aliases

```bash
alias del='ez session delete'
alias new='ez session new'
alias note='ez session note open'
alias Note='ez session note cd'
```

**tmux** — open the current session's note in a popup:

```bash
bind -T prefix n display-popup -E -w 65% -h 65% "ez session note open"
```

## Native browser (experimental)

ez also ships a built-in full-screen browser that does not need fzf. It is experimental; fzf stays the default.

Enable it (and switch back) with:

```bash
ez config set selector.backend native
ez config set selector.backend fzf
```

Options live in the `[native]` section of `config.toml`:

```toml
[selector]
backend = "native"

[native]
preview_ratio = 50   # preview pane width in percent, 10-90 (default 50)
snapshot = true      # draw the last-seen rows immediately on start (default true)
```

```bash
ez config set native.preview_ratio 40
ez config set native.snapshot false
```

Behavior:

- One full-screen session for the whole flow: view switching, drill-down, session lists and actions all happen in one process, without restarting a selector between levels.
- View switching is instant, and each top-level view keeps its own query and cursor while you are in another view.
- Branches, PR status, previews and plugin views load in background threads, so typing and cursor movement never wait on git, disk or plugins.
- Snapshot first paint: on exit the last shown rows are saved to `<data_dir>/ez/browser-snapshot.toml` (macOS: `~/Library/Application Support/ez/`, Linux: `~/.local/share/ez/`). The next start draws them on the first frame and swaps in fresh rows, keeping the cursor on the same item. A missing or corrupt snapshot is ignored. Disable with `native.snapshot = false`.
- Prompts (name builder, confirm, input, config wizard) appear as modals over the browser.
- Keybinds use the same fzf key syntax as the config (`ctrl-t`, `alt-d`, `enter`, ...), so your existing `[keybinds]` work unchanged.

Differences and limits compared to fzf:

- `fzf.extra_opts` and `selector.fzf_opts` are ignored.
- A modal text input cancelled with Esc returns its default, like fzf.
- Previews are not shown inside modals.
- Validation messages from wizards are shown after the wizard finishes, not inline.

Set `EZ_NO_DAEMON=1` to stop ez from auto-starting its background daemon (useful for tests and sandboxes).

## Testing with a dev config

To try ez (for example the native browser) without touching your real setup, point it at throwaway directories:

```bash
EZ_CONFIG_DIR=~/ez-dev/config EZ_DATA_DIR=~/ez-dev/data ez
```

`EZ_CONFIG_DIR` replaces the config directory (config file, repo index, sessions, plugins, daemon pid/log, so a dev daemon never collides with the default one) and `EZ_DATA_DIR` replaces the data directory (notes, browser snapshot). Unset or empty means the platform defaults. Set `EZ_NO_DAEMON=1` to avoid auto-starting the daemon.

`make dev` does this for a debug build, using `.dev/` in the repo (override with `DEV_DIR=...`):

```bash
make dev                                  # launch the browser against .dev/
make dev ARGS="config get selector.backend"
make dev ARGS="add ~/code/some-repo"      # the dev index starts empty
```

On first run it copies your real `config.toml` (repos and sessions are not copied) and sets `selector.backend` to `native`. Repos under `workspace_roots` still show up through scanning. `make dev` is for testing the UI: without the shell wrapper, cd effects need `--cd-file <path>`.

## Docs

- [User Guide](docs/user-guide.md) — full command reference, config options, name builder modes
- [Plugin Guide](docs/plugin-guide.md) — writing custom plugins
- [Architecture](docs/architecture.md)
- [Design](docs/design.md)

## Requirements

- Rust 1.70+
- [fzf](https://github.com/junegunn/fzf)
- git
- jq (for bundled plugins)
- tmux, zellij 0.40+ or Rex (optional)

## Windows Prerequisites

Plugins are bash scripts and require the following on Windows:

1. **Git for Windows** — provides `bash.exe` used to run plugins
   - Install: `winget install Git.Git` or download from https://gitforwindows.org
2. **jq** — used by plugins for JSON parsing
   - Install: `winget install stedolan.jq` or download from https://jqlang.github.io/jq/

Or run `make setup` to install both automatically.

## License

AGPL-3.0
