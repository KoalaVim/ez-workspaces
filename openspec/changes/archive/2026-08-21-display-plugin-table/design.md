## Context

`list_plugins()` in `src/plugin/mod.rs` uses `println!("{:<20} {:<19} {}", ...)` to display plugins. The hardcoded widths break alignment when a name exceeds 20 characters. There is no header row.

## Goals / Non-Goals

**Goals:**
- Dynamic column widths that adapt to the longest value in each column.
- A header row with a separator line so columns are self-documenting.
- Preserve existing colored output (cyan names, green/dimmed status).

**Non-Goals:**
- Adding new columns (version, hooks, priority).
- Adding a table-rendering crate dependency.

## Decisions

**D1: No external table crate.** The output has 3 fixed columns and a small number of rows. A two-pass approach (measure max widths, then print) is straightforward and avoids a new dependency.

**D2: Header row with separator.** Print `Name`, `Status`, `Description` as a header, followed by a `─` separator line. This makes the output self-explanatory.

**D3: Two-pass rendering.** First pass collects all plugins into a vec and computes max widths for name and status columns. Second pass prints the header, separator, and rows. Description column remains unbounded (wraps naturally in the terminal).

## Risks / Trade-offs

**R1: Two passes over plugin entries.** Negligible cost given the expected plugin count (< 20). Collecting into a vec is simpler than streaming output with unknown widths.
