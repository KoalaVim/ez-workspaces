## ADDED Requirements

### Requirement: Selector backend configuration
The config SHALL support `[selector] backend` with values `"fzf"` (default) and `"native"`. It SHALL be settable with `ez config set selector.backend <value>` and in interactive setup. Values other than these two SHALL be rejected by `config set` with an error naming the accepted values.

#### Scenario: Enable native backend
- **WHEN** the user runs `ez config set selector.backend native`
- **THEN** the config file stores `backend = "native"` under `[selector]` and the next `ez` launch uses the native browser

#### Scenario: Reject unknown backend
- **WHEN** the user runs `ez config set selector.backend skim`
- **THEN** the command fails with an error listing `fzf` and `native`, and the config is not changed

### Requirement: Native browser configuration
The config SHALL support a `[native]` section for options that only apply to the native browser: `preview_ratio` (percentage of width for the preview pane, default `50`) and `snapshot` (boolean, default `true`, enables snapshot first paint). The `[fzf]` section SHALL NOT affect the native browser.

#### Scenario: Custom preview width
- **WHEN** config has `[native] preview_ratio = 40`
- **THEN** the native browser's preview pane uses 40% of the terminal width

#### Scenario: Disable snapshot
- **WHEN** config has `[native] snapshot = false`
- **THEN** the native browser neither reads nor writes the snapshot file and waits for fresh data before the first paint

#### Scenario: Fzf options ignored in native mode
- **WHEN** config has `[fzf] extra_opts = "--border"` and `backend = "native"`
- **THEN** the native browser starts normally and ignores the fzf options

### Requirement: Config and data directory override
The system SHALL honor `EZ_CONFIG_DIR` and `EZ_DATA_DIR` environment variables. When set to a non-empty value, `EZ_CONFIG_DIR` SHALL replace the ez config directory (config file, repo index, sessions, plugins, daemon pid/log) and `EZ_DATA_DIR` SHALL replace the ez data directory (notes, browser snapshot). When unset or empty, the platform defaults SHALL be used. Plugins already receive `EZ_CONFIG_DIR`, so nested `ez` invocations from plugins SHALL resolve the same directories.

#### Scenario: Dev config directory
- **WHEN** the user runs `EZ_CONFIG_DIR=~/ez-dev/config EZ_DATA_DIR=~/ez-dev/data ez`
- **THEN** ez reads `~/ez-dev/config/config.toml` and stores repos, sessions, and plugins under `~/ez-dev/config/`, and notes and the snapshot under `~/ez-dev/data/`
- **AND** the default config and data directories are not read or written

#### Scenario: Separate daemon per config dir
- **WHEN** ez runs with `EZ_CONFIG_DIR` set
- **THEN** the daemon pid and log files live in that directory, so a dev daemon does not conflict with the default one

#### Scenario: Unset falls back to defaults
- **WHEN** `EZ_CONFIG_DIR` is unset or empty
- **THEN** ez uses the platform config directory as before
