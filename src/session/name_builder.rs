//! Interactive multi-stage session name builder.
//!
//! Each `choice`-kind stage shows an fzf list of choices plus a `(none)`
//! sentinel row. The user can either pick an item, type a custom value (the
//! typed query becomes the part on Enter when no item matches), or pick
//! `(none)` to skip the part. `text`-kind stages skip fzf entirely and use
//! a free-text prompt. Ctrl-P goes back to the previous stage.
//!
//! Parts are joined with "-"; `(none)` parts contribute nothing. The final
//! joined name must be non-empty.

use crate::browser::selector::{InteractiveSelector, SelectItem, StageOutcome};
use crate::config::model::{EzConfig, StageKind};
use crate::error::{EzError, Result};

const NONE_VALUE: &str = "__none__";
const NONE_DISPLAY: &str = "(none)";

/// Result of running the staged prompt.
pub enum NamePromptResult {
    /// User completed all stages with a non-empty name.
    Done(String),
    /// User cancelled (Esc/Ctrl-C) — caller should abort the operation.
    Cancelled,
}

/// Run the configured stages plus the final descriptive-name prompt and
/// return the joined session name. The default ("main") session bypasses
/// this entirely — callers handle that separately.
pub fn prompt_session_name(
    selector: &dyn InteractiveSelector,
    config: &EzConfig,
) -> Result<NamePromptResult> {
    let stages = &config.session_name_stages;
    // Parts collected per stage; entries may be `None` when the user picked
    // "(none)" or skipped. Length matches `stages.len() + 1` (the +1 is the
    // final descriptive part).
    let mut parts: Vec<Option<String>> = vec![None; stages.len() + 1];
    let mut idx: usize = 0;

    loop {
        let is_final = idx == stages.len();
        let (prompt, kind, choices): (String, StageKind, &[String]) = if is_final {
            // Implicit final descriptive-name stage: always free text, no
            // (none) (the joined name must be non-empty), but back works.
            ("name".into(), StageKind::Text, &[])
        } else {
            let s = &stages[idx];
            (s.name.clone(), s.kind.clone(), s.choices.as_slice())
        };
        let allow_back = idx > 0;

        let outcome = match kind {
            StageKind::Choice => {
                let items = build_items(choices, /*include_none=*/ !is_final);
                selector.select_with_back(&prompt, &items, allow_back)?
            }
            StageKind::Text => selector.input_with_back(&prompt, None, allow_back)?,
        };

        match outcome {
            StageOutcome::Picked(raw) => {
                let value = handle_pick(&kind, &raw)?;
                match value {
                    PickResolution::Use(s) => {
                        parts[idx] = Some(s);
                        if is_final {
                            let joined = join_parts(&parts);
                            if joined.is_empty() {
                                eprintln!("Session name cannot be empty.");
                                parts[idx] = None;
                                continue;
                            }
                            return Ok(NamePromptResult::Done(joined));
                        }
                        idx += 1;
                    }
                    PickResolution::None => {
                        if is_final {
                            eprintln!("Session name cannot be empty.");
                            continue;
                        }
                        parts[idx] = None;
                        idx += 1;
                    }
                }
            }
            StageOutcome::Back => {
                if idx > 0 {
                    idx -= 1;
                }
            }
            StageOutcome::Cancel => return Ok(NamePromptResult::Cancelled),
        }
    }
}

enum PickResolution {
    /// Use this string for the part.
    Use(String),
    /// Skip this part (treated as `(none)`).
    None,
}

fn handle_pick(kind: &StageKind, raw: &str) -> Result<PickResolution> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(PickResolution::None);
    }
    if matches!(kind, StageKind::Choice) && trimmed == NONE_VALUE {
        return Ok(PickResolution::None);
    }
    Ok(PickResolution::Use(trimmed.to_string()))
}

fn build_items(choices: &[String], include_none: bool) -> Vec<SelectItem> {
    let mut items: Vec<SelectItem> = choices
        .iter()
        .map(|v| SelectItem {
            display: v.clone(),
            value: v.clone(),
        })
        .collect();
    if include_none {
        items.push(SelectItem {
            display: NONE_DISPLAY.into(),
            value: NONE_VALUE.into(),
        });
    }
    items
}

fn join_parts(parts: &[Option<String>]) -> String {
    parts
        .iter()
        .filter_map(|p| p.as_ref())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Convenience: build a default fzf selector and run the prompt. Returns
/// `Err(EzError::Cancelled)` on cancel so callers can propagate quietly.
pub fn prompt_session_name_default(config: &EzConfig) -> Result<String> {
    use crate::browser::selector::FzfSelector;
    let selector = FzfSelector::new(&config.fzf)?;
    match prompt_session_name(&selector, config)? {
        NamePromptResult::Done(name) => Ok(name),
        NamePromptResult::Cancelled => Err(EzError::Cancelled),
    }
}
