# Shell Integration (Delta)

## ADDED Requirements

### Requirement: Return to ez after tmux detach

The shell wrapper SHALL support a loop mode where, after the user detaches from a tmux session (causing ez to exit with a detach indicator), the browser is automatically re-entered. This allows users to detach from one session and immediately pick another without manually re-running `ez`.

#### Scenario: Re-enter browser after detach

- **WHEN** user detaches from a tmux session (Ctrl-b d) while in an ez-managed session
- **THEN** the shell wrapper detects the detach condition and re-runs the ez browser loop
- **THEN** user sees the browser again and can select another session

#### Scenario: Normal exit does not loop

- **WHEN** user exits ez normally (Escape at top level, or session entered with cd)
- **THEN** the shell wrapper does NOT re-enter the browser; the command completes normally

#### Scenario: Ctrl-C exits fully

- **WHEN** user presses Ctrl-C during the browser
- **THEN** the shell wrapper exits completely without re-entering the loop

#### Scenario: Loop disabled by flag

- **WHEN** user runs `ez --no-loop` or the config has `browser_loop = false`
- **THEN** the return-to-ez loop is disabled and ez behaves as a single-shot invocation
