## Why

The preview pane in the session picker redundantly shows the full sessions tree — the same data already visible in the fzf list on the left. This wastes vertical space and adds visual noise. Additionally, PR status metadata (when available) is not shown in the Git Info section, missing an opportunity to surface useful context.

## What Changes

1. **Hide sessions section in session picker preview**: When `show_session_actions` is true (session action loop), skip rendering the "Sessions" section entirely. The repo labels sub-section moves up.
2. **Add PR status to Git Info**: When the repo has sessions with PR metadata, display PR status (number, state, URL) in the Git Info section of the preview.

## Capabilities

### New Capabilities

### Modified Capabilities
- `interactive-browser`: Preview pane hides redundant sessions section in session picker; Git Info section shows PR status when available

## Impact

- `src/browser/preview.rs`: Modify `preview_repo` and `preview_non_git_repo` to skip sessions section when `show_actions` is true. Add PR status display to Git Info section.
