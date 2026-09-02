## Context

On Windows, `std::fs::canonicalize` returns paths with a `\\?\` prefix — the "extended-length path" form that bypasses the 260-character `MAX_PATH` limit. While technically correct, this prefix breaks string-based comparisons (e.g., `collapse_tilde` matching against `dirs::home_dir()`) and produces ugly UI output. The codebase has a `paths::normalize` wrapper around `canonicalize`, but also ~18 direct `canonicalize()` calls scattered across modules.

## Goals / Non-Goals

**Goals:**
- Paths displayed in the browser UI show `~/...` on Windows, matching Unix behavior
- Single fix point: strip the `\\?\` prefix inside `paths::normalize` so all callers benefit
- Replace direct `canonicalize()` calls with `paths::normalize()` where the result flows to display or is compared against registered repo paths

**Non-Goals:**
- Preserving `\\?\` for paths that genuinely exceed `MAX_PATH` (not a real scenario for this tool)
- Fixing `collapse_tilde` to handle `\\?\` — the prefix should never reach it

## Decisions

### Strip in `normalize`, not in `collapse_tilde`

Fix the problem at the source (`normalize`) rather than working around it downstream (`collapse_tilde`). The `\\?\` prefix is a `canonicalize` artifact, not meaningful data — stripping it early keeps every downstream consumer clean.

**Alternative considered:** Making `collapse_tilde` strip the prefix before matching — rejected because it only fixes display, not path comparisons elsewhere that also break on the prefix.

### Replace direct `canonicalize()` with `paths::normalize()`

Many call sites use `path.canonicalize()` directly instead of `paths::normalize(path)`. Each of these produces `\\?\`-prefixed paths on Windows. Replacing them centralizes the fix.

**Scope:** Only replace calls whose results flow to display, comparison against registered paths, or storage. A call used purely for symlink resolution in a test fixture can stay as-is.

## Risks / Trade-offs

- **Behavioral risk is minimal** — stripping `\\?\` only changes the string representation, not the filesystem semantics. The standard `C:\...` form works for all path operations in this codebase.
- **Incomplete replacement** — if new `canonicalize()` calls are added in the future, they could reintroduce the prefix. Mitigation: a clippy lint or code review convention to prefer `paths::normalize`.
