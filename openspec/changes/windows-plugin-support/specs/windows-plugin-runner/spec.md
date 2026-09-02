# Windows Plugin Runner

## Purpose

Enable the plugin runner to execute bash-based plugin scripts on Windows by locating and invoking `bash.exe` from Git for Windows, and using it for all shell command execution.

## ADDED Requirements

### Requirement: Bash discovery on Windows
On Windows, the system SHALL locate `bash.exe` using the following fallback chain:
1. `bash` on the system PATH
2. `C:\Program Files\Git\usr\bin\bash.exe`
3. `C:\Program Files (x86)\Git\usr\bin\bash.exe`

The resolved path SHALL be cached for the lifetime of the process.

#### Scenario: Bash found on PATH
- **WHEN** `bash.exe` exists on the system PATH on Windows
- **THEN** the system uses that path for all plugin and shell command execution

#### Scenario: Bash found at Git for Windows default location
- **WHEN** `bash.exe` is not on PATH but exists at `C:\Program Files\Git\usr\bin\bash.exe`
- **THEN** the system uses that path for all plugin and shell command execution

#### Scenario: Bash not found
- **WHEN** `bash.exe` is not found at any known location on Windows
- **THEN** the system returns an error directing the user to install Git for Windows

#### Scenario: Result is cached
- **WHEN** `find_bash()` is called multiple times within one process
- **THEN** the lookup runs only once and subsequent calls return the cached result

### Requirement: Plugin execution on Windows
On Windows, the system SHALL execute plugin scripts by invoking `bash.exe <script_path>` instead of relying on OS shebang resolution. On Unix, plugins SHALL continue to be executed directly.

#### Scenario: Plugin script executed on Windows
- **WHEN** a plugin hook is triggered on Windows
- **THEN** the system spawns `bash.exe` with the plugin script path as argument, writes the JSON request to stdin, and reads the JSON response from stdout

#### Scenario: Plugin script executed on Unix
- **WHEN** a plugin hook is triggered on Unix
- **THEN** the system spawns the plugin executable directly (unchanged behavior)

### Requirement: Shell commands on Windows
On Windows, `run_shell_commands` SHALL use `bash.exe -c <cmd>` instead of `sh -c <cmd>`. On Unix, it SHALL continue to use `sh -c`.

#### Scenario: Shell command on Windows
- **WHEN** a plugin returns `shell_commands` or `post_shell_commands` on Windows
- **THEN** the system executes each command via `bash.exe -c <cmd>`

#### Scenario: Shell command on Unix
- **WHEN** a plugin returns `shell_commands` on Unix
- **THEN** the system executes each command via `sh -c <cmd>` (unchanged behavior)

### Requirement: Windows prerequisites documentation
The README SHALL document that Windows users need Git for Windows (provides bash) and jq installed and available to bash.

#### Scenario: README lists Windows prerequisites
- **WHEN** a user reads the README on Windows
- **THEN** they find instructions for installing Git for Windows and jq

### Requirement: UNC path prefix stripping
On Windows, `std::fs::canonicalize()` produces paths with a `\\?\` prefix that bash cannot handle. The system SHALL strip this prefix from repo paths and session paths when loading them from storage.

#### Scenario: Repo path with UNC prefix loaded
- **WHEN** a repo path stored as `\\?\C:\Users\...` is loaded from the index on Windows
- **THEN** the system strips the `\\?\` prefix and downstream code sees `C:\Users\...`

#### Scenario: Session path with UNC prefix loaded
- **WHEN** a session path stored with `\\?\` prefix is loaded from the sessions file
- **THEN** the system strips the prefix before passing it to plugins or displaying it

#### Scenario: Path without prefix unchanged
- **WHEN** a path without the `\\?\` prefix is loaded
- **THEN** the system returns it unchanged

### Requirement: Setup target in Makefile
The Makefile SHALL include a `setup` target that installs platform-appropriate prerequisites.

#### Scenario: Setup on Windows
- **WHEN** user runs `make setup` on Windows
- **THEN** `winget` installs Git for Windows and jq

#### Scenario: Setup on macOS
- **WHEN** user runs `make setup` on macOS
- **THEN** `brew` installs jq

#### Scenario: Setup on Linux
- **WHEN** user runs `make setup` on Linux
- **THEN** `apt-get` installs jq
