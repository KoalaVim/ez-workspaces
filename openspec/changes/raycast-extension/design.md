## Context

ez-workspaces has a Raycast integration via two bash script commands that display repo/session lists as text output. Raycast script commands are inherently non-interactive — limited to static arguments and text rendering. For proper UX (searchable lists, drill-down navigation, action panels), a Raycast Extension built with TypeScript/React is needed.

The extension consumes the existing `ez` CLI JSON output (`ez repo list --json`, `ez session list --json`) and uses AppleScript for terminal integration when entering sessions.

## Goals / Non-Goals

**Goals:**
- Interactive browsable repo list with search, view mode switching, and drill-down to sessions
- Session list with tree hierarchy rendering, LRU timestamps, PR status, and rich actions
- Cross-repo session search for finding sessions without knowing which repo they belong to
- Terminal integration to enter sessions (tmux attach) from Raycast
- Editor integration to open session paths in Cursor/VS Code from Raycast

**Non-Goals:**
- Publishing to Raycast Store (local extension only)
- Session creation from Raycast (too interactive — name builder, mode picker — better done in terminal with fzf)
- Plugin views in Raycast (deferred — core views only for now)
- iTerm2-specific AppleScript (start with Terminal.app, add iTerm2 later if needed)
- Replacing the terminal-based fzf browser (Raycast is a complementary launcher, not a replacement)

## Decisions

### 1. Separate TypeScript project under `raycast/ez-extension/`

The extension lives alongside the Rust crate as a separate TypeScript project with its own `package.json`. It has no build-time coupling to the Rust code — it only calls the `ez` binary at runtime via `child_process`.

**Why not a Raycast Store extension?** The extension depends on `ez` being installed locally. It's a personal/project tool, not a general-purpose extension.

### 2. Data fetching via `ez` CLI with `--json` flags

All data comes from executing `ez repo list --json` and `ez session list --json --repo <name>` via `child_process.execSync`. This reuses existing CLI output with no new API surface.

**Alternative considered:** Reading TOML files directly from `~/.config/ez/`. Rejected — duplicates parsing logic and breaks if storage format changes.

### 3. Two entry-point commands

- `browse-repos`: Main command. Lists repos, user selects one, pushes to session list.
- `search-sessions`: Secondary command. Flat list of all sessions across all repos, grouped by repo sections.

**Why two commands instead of one?** Raycast commands are the top-level entry points. Having a dedicated session search avoids forcing users through repo selection when they know the session name.

### 4. View modes via `List.Dropdown` (searchBarAccessory)

The browse-repos command supports switching between Repo (flat), Owner (grouped by path parent), Label (grouped by label), and Workspace (grouped by workspace root) views using Raycast's built-in dropdown in the search bar.

### 5. Session tree hierarchy via accessories

Sessions are rendered flat in the Raycast `List` but convey hierarchy through:
- Indentation prefix in the title (e.g., `  ├── feature-branch`)
- Parent name shown as subtitle
- Default session marked with a tag accessory

This mimics the fzf tree glyphs within Raycast's UI constraints.

### 6. Terminal integration via AppleScript

Entering a session requires opening a terminal and running `ez session enter <name> --repo <repo>`. This is done via AppleScript targeting Terminal.app:

```
tell application "Terminal"
  do script "ez session enter <name> --repo <repo>"
  activate
end tell
```

AppleScript is the standard macOS mechanism for programmatic terminal interaction from GUI apps.

### 7. Keep old script commands as fallback

The bash scripts (`ez-repos.sh`, `ez-sessions.sh`) remain for users who prefer lightweight text output without installing the npm-based extension.

## Risks / Trade-offs

- **`ez` binary must be in PATH for Raycast** → The extension checks for `ez` availability and shows a clear error if missing. Users may need to configure Raycast's shell environment.
- **AppleScript terminal launch is macOS-only** → Raycast itself is macOS-only, so this is not a practical limitation.
- **No real-time updates** → Session/repo data is fetched on command open. If user creates a session in terminal, they need to re-open the Raycast command. Using `useCachedPromise` with short TTL mitigates staleness.
- **Cross-repo search fetches N+1 commands** → One `ez repo list --json` then one `ez session list --json` per repo. For large repo counts this could be slow. Acceptable for typical counts (5-20 repos).
