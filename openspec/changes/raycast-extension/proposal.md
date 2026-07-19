## Why

The current Raycast integration consists of two bash script commands (`ez-repos.sh`, `ez-sessions.sh`) that use `fullOutput` mode — they can only display text. Raycast script commands cannot provide interactive lists, drill-down navigation, or action panels. Users have to manually type repo/session names. A proper Raycast Extension (TypeScript/React) would let users browse repos, drill into sessions, and perform actions (enter, open in editor, copy path, delete) all from the Raycast launcher with full keyboard-driven UX.

## What Changes

- Replace display-only script commands with a full interactive Raycast Extension
- Add a "Browse Repos" command with searchable list, view mode switcher (repo/owner/label/workspace), and actions (push to sessions, open in Finder/Cursor/Terminal, copy path)
- Add a drill-down "Session List" view pushed from repo selection, showing session tree hierarchy, LRU timestamps, PR status, with actions (enter session via terminal, open in editor, delete, open PR)
- Add a "Search Sessions" command for flat cross-repo session search
- Terminal integration via AppleScript to open Terminal.app or iTerm2 and run `ez session enter`
- Keep old script commands as lightweight fallback

## Capabilities

### New Capabilities
- `raycast-extension`: Full interactive Raycast Extension with browse repos, session drill-down, cross-repo search, view mode switching, and terminal/editor integration actions

### Modified Capabilities

## Impact

- New TypeScript project at `raycast/ez-extension/` with `@raycast/api` and `@raycast/utils` dependencies
- Depends on existing `ez repo list --json` and `ez session list --json` CLI output
- Depends on `ez session enter` and `ez session delete` CLI commands
- Old `raycast/ez-repos.sh` and `raycast/ez-sessions.sh` kept as fallback
- `raycast/README.md` updated to point to extension as primary integration
