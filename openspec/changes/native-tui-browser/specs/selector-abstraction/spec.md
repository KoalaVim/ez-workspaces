## MODIFIED Requirements

### Requirement: Single injection point
The concrete selector SHALL be instantiated in exactly one place (`browse()` function) based on `selector.backend`. All other code receives `&dyn InteractiveSelector`. For `fzf` (default), a `FzfSelector` is created from `config.fzf`. For `native`, the native browser owns the terminal and provides a native selector bound to it for modal prompts.

#### Scenario: Backend selection
- **WHEN** the browser starts with `selector.backend` absent or `"fzf"`
- **THEN** `browse()` creates a `FzfSelector` from `config.fzf` settings and passes it as `&dyn InteractiveSelector` to all downstream functions

#### Scenario: Native backend selection
- **WHEN** the browser starts with `selector.backend = "native"`
- **THEN** `browse()` starts the native browser, and every prompt flow it calls (name builder, input, confirm) receives a native selector as `&dyn InteractiveSelector`

#### Scenario: Unknown backend
- **WHEN** `selector.backend` is set to an unrecognized value
- **THEN** `browse()` returns a config error naming the accepted values (`fzf`, `native`)

## ADDED Requirements

### Requirement: Native selector implementation
A native selector SHALL implement every `InteractiveSelector` method by drawing a modal on the native browser's terminal, without spawning external processes. It SHALL return the same result types and Esc/Ctrl-P/Ctrl-C outcomes as `FzfSelector`. `confirm` SHALL render as a modal yes/no prompt instead of reading stdin.

#### Scenario: Select with back
- **WHEN** `select_with_back` is called with `allow_back = true` and the user presses Ctrl-P
- **THEN** it returns `StageOutcome::Back`

#### Scenario: Typed query accepted
- **WHEN** the user types a value that matches no item in `select_with_back` and presses Enter
- **THEN** it returns `StageOutcome::Picked(typed_value)`

#### Scenario: Interrupt propagates
- **WHEN** the user presses Ctrl-C in any native modal
- **THEN** the method returns `Err(EzError::Interrupted)`

#### Scenario: Confirm modal
- **WHEN** `confirm` is called with `default: true` and the user presses Enter
- **THEN** it returns `true` without reading from stdin
