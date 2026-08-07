## MODIFIED Requirements

### Requirement: Cd-to-session command
The system SHALL provide `ez cd-to-session` that resolves the current session using the multiplexer-agnostic current-session detection (tmux user options, then `$ZELLIJ_SESSION_NAME`, then the working directory) and writes that session's path to the cd-file. This allows navigating back to an ez-managed session's worktree from anywhere inside the session's multiplexer session.

#### Scenario: Cd to tmux session path
- **WHEN** user runs `ez cd-to-session` inside a tmux session managed by ez
- **THEN** system resolves the session from the tmux user options and writes its path to the cd-file

#### Scenario: Cd to zellij session path
- **WHEN** user runs `ez cd-to-session` inside a zellij session whose name matches a registered session
- **THEN** system resolves the session from `$ZELLIJ_SESSION_NAME` and writes its path to the cd-file

#### Scenario: No multiplexer session
- **WHEN** user runs `ez cd-to-session` outside any multiplexer and the working directory is not inside a registered session
- **THEN** system returns an error indicating no current session could be resolved

#### Scenario: Unmanaged multiplexer session
- **WHEN** the current tmux session has no `@ez_session_path` option, or the current zellij session name matches no registered session
- **THEN** system falls back to working-directory matching and errors only if that also fails

### Requirement: Return to ez after tmux detach
The shell wrapper SHALL support a loop mode where, after the user detaches from a multiplexer session (tmux detach, zellij detach — each causing the attach command sourced from the post-cmd-file to return), the browser is automatically re-entered. This allows users to detach from one session and immediately pick another without manually re-running `ez`. The loop is multiplexer-agnostic: it is driven by the post-cmd-file returning control, not by which multiplexer was attached.

#### Scenario: Re-enter browser after detach
- **WHEN** user detaches from a tmux session (Ctrl-b d) while in an ez-managed session
- **THEN** the shell wrapper detects the detach condition and re-runs the ez browser loop
- **THEN** user sees the browser again and can select another session

#### Scenario: Re-enter browser after zellij detach
- **WHEN** user detaches from a zellij session that was attached via the zellij plugin's post-shell command
- **THEN** the shell wrapper re-runs the ez browser loop the same way it does for tmux

#### Scenario: Normal exit does not loop
- **WHEN** user exits ez normally (Escape at top level, or session entered with cd)
- **THEN** the shell wrapper does NOT re-enter the browser; the command completes normally

#### Scenario: Ctrl-C exits fully
- **WHEN** user presses Ctrl-C during the browser
- **THEN** the shell wrapper exits completely without re-entering the loop

#### Scenario: Loop disabled by flag
- **WHEN** user runs `ez --no-loop` or the config has `browser_loop = false`
- **THEN** the return-to-ez loop is disabled and ez behaves as a single-shot invocation
