## MODIFIED Requirements

### Requirement: Session labels
Sessions SHALL support arbitrary string labels for grouping and filtering. Labels can be added, removed, and listed via CLI commands.

#### Scenario: Add labels
- **WHEN** user runs `ez session label add feature-x wip urgent`
- **THEN** system adds the labels `wip` and `urgent` to session `feature-x`

#### Scenario: List labels grouped
- **WHEN** user runs `ez session label list` without a session name
- **THEN** system lists all sessions grouped by their labels

### Requirement: Session tree hierarchy
Sessions SHALL be organized in a tree using `parent_id` pointers. The system SHALL support operations: list roots, find children, find ancestors, find descendants, and render as an indented tree.

#### Scenario: Render tree
- **WHEN** user runs `ez session list`
- **THEN** system renders sessions as an indented tree with root sessions at the top level and children indented below their parents

#### Scenario: Flat list
- **WHEN** user runs `ez session list --flat`
- **THEN** system renders sessions as a flat list without tree structure

#### Scenario: JSON session list
- **WHEN** user runs `ez session list --json --repo my-repo`
- **THEN** system outputs a JSON array of session objects with fields: `id`, `name`, `parent_id`, `path`, `bare`, `labels`, `last_accessed`, `env`, `is_default`
