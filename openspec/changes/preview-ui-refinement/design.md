## Context

The preview pane (`ez preview <path>`) renders repo information in the right 50% of fzf. When called with `--session-actions`, it's in the session picker context where sessions are already listed in the fzf left pane.

## Goals / Non-Goals

**Goals:**
- Remove redundant sessions section from preview when in session picker context
- Show PR status in Git Info section when available

**Non-Goals:**
- Changing preview layout for non-session contexts (workspace/repo views still show sessions)
- Adding new PR data fetching (only display existing metadata)

## Decisions

### 1. Conditionally skip sessions section
Use the existing `show_actions` boolean to gate the sessions section. When `show_actions` is true, skip the sessions rendering and repo labels. The keybinds section already uses this flag.

### 2. PR status from session env
When rendering Git Info for a repo, load sessions and check for PR metadata (`ez_pr_number`, `ez_pr_status`, `ez_pr_url`) in the current branch's session. Display it as a line in Git Info: `pr: #42 open https://...`.

## Risks / Trade-offs

- Minor: the preview in session picker will be shorter (more whitespace). This is actually desirable — less clutter.
