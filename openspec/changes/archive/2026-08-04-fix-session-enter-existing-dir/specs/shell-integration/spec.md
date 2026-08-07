## MODIFIED Requirements

### Requirement: Shell wrapper function
The system SHALL provide a shell wrapper function via `ez init-shell <shell>` that the user evals in their shell config. The wrapper creates tempfiles, runs the `ez` binary with `--cd-file` and `--post-cmd-file` flags, then applies the results after `ez` exits.

The wrapper SHALL always source the post-cmd-file if it is non-empty (to apply env exports, tmux commands, etc.). The wrapper SHALL only re-invoke `ez` (loop) when the post-cmd-file contains the `#EZ_RELAUNCH` marker line, indicating the binary explicitly requests re-invocation. Without the marker, the wrapper SHALL break out of the loop and apply the cd-file normally.

#### Scenario: Zsh/Bash wrapper
- **WHEN** user runs `eval "$(ez init-shell zsh)"`
- **THEN** a shell function `ez` is defined that:
  1. Creates two tempfiles
  2. Runs `command ez "$@" --cd-file="$tmp" --post-cmd-file="$post_cmd"`
  3. If the post-cmd-file is non-empty, sources it
  4. If the post-cmd-file contained `#EZ_RELAUNCH`, sets `--repo` from cd-file and loops back to step 2
  5. Otherwise breaks the loop, and if the cd-file is non-empty, runs `cd "$(cat "$tmp")"`
  6. Cleans up tempfiles and returns the exit code

#### Scenario: Fish wrapper
- **WHEN** user runs `eval (ez init-shell fish)`
- **THEN** an equivalent Fish function is defined with the same marker-based re-invocation behavior

#### Scenario: PowerShell wrapper
- **WHEN** user runs `ez init-shell pwsh`
- **THEN** an equivalent PowerShell function is defined with the same marker-based re-invocation behavior

#### Scenario: Session enter with env vars does not re-invoke
- **WHEN** user runs `ez session enter <name>` for a session with PR metadata env vars
- **THEN** the shell wrapper sources the env exports from post-cmd-file
- **THEN** the wrapper does NOT re-invoke `ez` (no `#EZ_RELAUNCH` marker present)
- **THEN** the wrapper cds to the session's worktree path from cd-file

#### Scenario: Browser plugin bind action re-invokes
- **WHEN** user triggers a plugin bind action in the browser (e.g. kill a tmux session)
- **THEN** the binary writes the action's post-shell-commands AND the `#EZ_RELAUNCH` marker to post-cmd-file
- **THEN** the shell wrapper sources the commands and re-invokes `ez` to re-open the browser

#### Scenario: Unsupported shell
- **WHEN** user runs `ez init-shell powershell`
- **THEN** system returns an error listing supported shells (bash, zsh, fish, pwsh)

### Requirement: Post-command-file pattern
The system SHALL write post-exit shell commands to the post-cmd-file (passed via `--post-cmd-file`). These are commands that must run in the user's shell after ez exits, such as `tmux switch-client`, `tmux kill-session`, or `export` statements for session env vars. If no post-cmd-file is available, the system warns about an outdated shell wrapper and runs the commands inline as a fallback.

When the binary intends for the shell wrapper to re-invoke `ez` after sourcing post commands, it SHALL append a `#EZ_RELAUNCH` marker line to the post-cmd-file. This marker is a shell comment (no-op when sourced) that signals the wrapper to loop.

#### Scenario: Post-exit tmux command
- **WHEN** the tmux plugin returns `post_shell_commands: ["tmux switch-client -t my-session"]`
- **THEN** system writes the command to the post-cmd-file
- **THEN** the shell wrapper sources the file after ez exits

#### Scenario: Session env exports
- **WHEN** a session with env vars (e.g. PR metadata) is entered
- **THEN** system writes `export` statements to the post-cmd-file
- **THEN** the shell wrapper sources them, setting the env vars in the parent shell
- **THEN** the wrapper does NOT re-invoke `ez` (no `#EZ_RELAUNCH` marker)

#### Scenario: Post-exit tmux kill on session delete
- **WHEN** user deletes a session that has an associated tmux session
- **THEN** system writes `tmux kill-session -t "=<tmux-session-name>"` to the post-cmd-file
- **THEN** the shell wrapper sources the file after ez exits, killing the tmux session

#### Scenario: Outdated wrapper fallback
- **WHEN** `--post-cmd-file` is not provided but post-shell commands exist
- **THEN** system prints a warning to re-run `eval "$(ez init-shell zsh)"` and runs the commands inline

### Requirement: Return to ez after tmux detach
The shell wrapper SHALL support re-invocation when the `ez` binary writes the `#EZ_RELAUNCH` marker to the post-cmd-file. This enables flows where the binary exits but the user should re-enter the browser (e.g. after a plugin bind action kills a tmux session, or after tmux detach).

#### Scenario: Re-enter browser after plugin bind action
- **WHEN** a plugin bind action in the browser writes post-shell-commands with `#EZ_RELAUNCH`
- **THEN** the shell wrapper sources the commands and re-invokes `ez --repo <cd-target>`
- **THEN** user sees the browser again

#### Scenario: Normal exit does not loop
- **WHEN** user exits ez normally (Escape at top level, or session entered with cd)
- **THEN** the shell wrapper does NOT re-enter the browser; the command completes normally
