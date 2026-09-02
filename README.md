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

All keybinds are [configurable](docs/user-guide.md#keybinds).

## Plugins

Bundled plugins — just enable:

```bash
ez plugin enable git-worktree        # worktree lifecycle
ez plugin enable tmux                # tmux session management
ez plugin enable zellij              # zellij session management
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
- tmux or zellij 0.40+ (optional)

## Windows Prerequisites

Plugins are bash scripts and require the following on Windows:

1. **Git for Windows** — provides `bash.exe` used to run plugins
   - Install: `winget install Git.Git` or download from https://gitforwindows.org
2. **jq** — used by plugins for JSON parsing
   - Install: `winget install stedolan.jq` or download from https://jqlang.github.io/jq/

Or run `make setup` to install both automatically.

## License

AGPL-3.0
