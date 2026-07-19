# Interactive Browser (Delta)

## MODIFIED Requirements

### Requirement: Preview pane
The browser SHALL show a preview pane (right 50%) in fzf. The preview calls back into the ez binary (`ez preview <path>`) to render repo info, git status, or keybind help depending on context. When in the session action loop (`--session-actions`), the preview SHALL NOT render the Sessions section (it is already visible in the fzf list). The Git Info section SHALL display PR status when the repo has sessions with PR metadata.

#### Scenario: Session picker preview hides sessions
- **WHEN** user is in the session action loop and highlights a session
- **THEN** the preview pane does NOT show the "Sessions" section or repo labels
- **AND** shows Git Info, Recent Commits, and Keybinds sections

#### Scenario: PR status in Git Info
- **WHEN** user previews a repo that has sessions with `ez_pr_number` and `ez_pr_status` set
- **THEN** the Git Info section includes a PR status line showing the PR number, state, and URL
