## MODIFIED Requirements

### Requirement: From GitHub PR mode
The "From GitHub PR" mode SHALL prompt the user for a GitHub PR reference and accept either a full PR URL (`https://github.com/<owner>/<repo>/pull/<number>`) or a bare PR number (`<number>` or `#<number>`). When a bare number is given, the system SHALL resolve the GitHub repository from the current repo's remote by running the `gh` lookup with the repo root as its working directory; the mode SHALL therefore receive the repo root path from its caller. The mode SHALL extract the PR number as `pr<number>`, and invoke a plugin hook to optionally resolve the PR's branch name for the worktree. If the plugin hook is available and returns a branch name, the system SHALL use that branch for the worktree instead of creating a new one.

#### Scenario: Paste PR URL and extract number
- **WHEN** user selects "From GitHub PR" mode and pastes `https://github.com/org/repo/pull/42`
- **THEN** system extracts the PR number and sets the session name to `pr42`

#### Scenario: Enter bare PR number inside a repo
- **WHEN** user selects "From GitHub PR" mode and enters `42` (or `#42`)
- **THEN** system resolves the PR against the current repo's GitHub remote
- **THEN** the session is named and populated exactly as if the equivalent PR URL had been pasted

#### Scenario: Bare number with no repo context
- **WHEN** the mode is invoked without a repo root (no repo context available) and the user enters a bare number
- **THEN** system shows an error explaining that a full PR URL is required here and re-prompts

#### Scenario: Bare number in a repo with no GitHub remote
- **WHEN** user enters a bare number in a repo whose remote is not on GitHub, so the `gh` lookup fails
- **THEN** system warns with the `gh` error and falls back to `pr<number>` as the session name

#### Scenario: Plugin resolves branch
- **WHEN** the `OnPRResolve` plugin hook is available and the PR URL is provided
- **THEN** system invokes the hook with the PR URL
- **THEN** if the hook returns a branch name, the worktree is created on that branch instead of a new one

#### Scenario: Plugin hook unavailable
- **WHEN** no plugin handles `OnPRResolve`
- **THEN** system proceeds with `pr<number>` as the session name and creates a standard worktree

#### Scenario: Invalid PR reference
- **WHEN** user enters a string that is neither a valid GitHub PR URL nor a PR number
- **THEN** system shows an error naming both accepted forms and re-prompts

### Requirement: Mode selection step
The system SHALL present a mode selection prompt before entering the staged name builder when creating a session interactively. The available modes SHALL be configurable. The user selects a mode via fzf, and the system dispatches to the corresponding mode handler. The "From GitHub PR" entry SHALL advertise both accepted input forms (URL or number).

#### Scenario: Mode picker displayed
- **WHEN** user creates a session interactively (no name provided or `--interactive` flag)
- **THEN** system presents a mode selection list with all configured modes before any name building begins

#### Scenario: GitHub PR mode label
- **WHEN** the mode picker is displayed
- **THEN** the "From GitHub PR" row indicates that either a PR URL or a PR number is accepted

#### Scenario: Single mode configured
- **WHEN** only one mode is configured in `name_builder_modes`
- **THEN** system skips the mode picker and enters that mode directly

#### Scenario: Cancel mode selection
- **WHEN** user presses Escape at the mode selection step
- **THEN** session creation is cancelled and the system returns to the previous context
