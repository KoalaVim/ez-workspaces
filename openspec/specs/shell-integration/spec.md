# Shell Integration

## Purpose

Enable ez-workspaces to change the user's shell working directory and execute post-exit commands — operations that are impossible for a child process to perform directly. The system uses a shell wrapper function pattern (cd-file + post-cmd-file) and provides shell completions for a seamless CLI experience.

## Requirements

### Requirement: Shell wrapper function
The system SHALL provide a shell wrapper function via `ez init-shell <shell>` that the user evals in their shell config. The wrapper creates tempfiles, runs the `ez` binary with `--cd-file` and `--post-cmd-file` flags, then applies the results after `ez` exits.

#### Scenario: Zsh/Bash wrapper
- **WHEN** user runs `eval "$(ez init-shell zsh)"`
- **THEN** a shell function `ez` is defined that:
  1. Creates two tempfiles
  2. Runs `command ez "$@" --cd-file="$tmp" --post-cmd-file="$post_cmd"`
  3. If the cd-file is non-empty, runs `cd "$(cat "$tmp")"`
  4. If the post-cmd-file is non-empty, runs `source "$post_cmd"`
  5. Cleans up tempfiles and returns the exit code

#### Scenario: Fish wrapper
- **WHEN** user runs `eval (ez init-shell fish)`
- **THEN** an equivalent Fish function is defined with the same behavior

#### Scenario: Unsupported shell
- **WHEN** user runs `ez init-shell powershell`
- **THEN** system returns an error listing supported shells (bash, zsh, fish)

### Requirement: Cd-file pattern
The system SHALL write the target directory path to the cd-file (passed via `--cd-file`) whenever a navigation action occurs (session enter, browser selection, plugin response). If no cd-file is provided, the path is printed to stdout.

#### Scenario: Navigate via cd-file
- **WHEN** user selects a session in the browser (with shell wrapper active)
- **THEN** the session's worktree path is written to the cd-file
- **THEN** the shell wrapper reads the file and runs `cd` to that path

#### Scenario: No shell wrapper
- **WHEN** `--cd-file` is not provided
- **THEN** system prints the path to stdout

### Requirement: Post-command-file pattern
The system SHALL write post-exit shell commands to the post-cmd-file (passed via `--post-cmd-file`). These are commands that must run in the user's shell after ez exits, such as `tmux switch-client`. If no post-cmd-file is available, the system warns about an outdated shell wrapper and runs the commands inline as a fallback.

#### Scenario: Post-exit tmux command
- **WHEN** the tmux plugin returns `post_shell_commands: ["tmux switch-client -t my-session"]`
- **THEN** system writes the command to the post-cmd-file
- **THEN** the shell wrapper sources the file after ez exits

#### Scenario: Outdated wrapper fallback
- **WHEN** `--post-cmd-file` is not provided but post-shell commands exist
- **THEN** system prints a warning to re-run `eval "$(ez init-shell zsh)"` and runs the commands inline

### Requirement: Cd-to-session command
The system SHALL provide `ez cd-to-session` that reads the `@ez_session_path` tmux user option from the current tmux session and writes it to the cd-file. This allows navigating back to an ez-managed session's worktree from within tmux.

#### Scenario: Cd to tmux session path
- **WHEN** user runs `ez cd-to-session` inside a tmux session managed by ez
- **THEN** system reads `@ez_session_path` from tmux and writes it to the cd-file

#### Scenario: Not in tmux
- **WHEN** user runs `ez cd-to-session` outside of tmux
- **THEN** system returns a config error indicating tmux is required

#### Scenario: No ez session path set
- **WHEN** the tmux session has no `@ez_session_path` option
- **THEN** system returns an error indicating the session is not ez-managed

### Requirement: Shell completions
The system SHALL generate shell completions via `ez completions <shell>` for Zsh, Bash, and Fish using clap's completion generator.

#### Scenario: Zsh completions
- **WHEN** user runs `ez completions zsh > ~/.zfunc/_ez`
- **THEN** system generates Zsh completion script and writes it to stdout

### Requirement: Colored output with no-color flag
All CLI output SHALL use colored output via the `colored` crate. The `--no-color` global flag SHALL disable all colors. Convention: green for success, yellow for warnings, cyan for info/labels, bold for emphasis, dimmed for secondary info.

#### Scenario: No-color mode
- **WHEN** user runs `ez --no-color repo list`
- **THEN** all output is plain text without ANSI color codes

#### Scenario: Force colors in preview
- **WHEN** fzf pipes preview output (which strips TTY detection)
- **THEN** system forces colors on for preview commands unless `--no-color` is set

### Requirement: Debug logging
The system SHALL support `--debug` flag that writes debug logs to `/tmp/ez-debug-<pid>.log` and prints the log path on exit. Plugins read the `EZ_DEBUG` environment variable to decide whether to emit their own debug logs.

#### Scenario: Debug mode
- **WHEN** user runs `ez --debug`
- **THEN** system logs to a temp file, sets `EZ_DEBUG=1`, and prints the log path on exit

### Requirement: Cancellation exit code
The system SHALL exit with code 130 when the user cancels (Escape/Ctrl-C), matching the Unix convention for SIGINT termination.

#### Scenario: Cancel exits 130
- **WHEN** user presses Escape at the top level of the browser
- **THEN** ez exits with code 130
