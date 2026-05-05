//! Interactive multi-stage session name builder.
//!
//! Each stage shows an fzf list of choices plus two sentinel rows:
//!   - "(custom)" — drops to a separate text-input prompt for free-text
//!   - "(none)"   — skip this part (omitted on the final stage)
//! Picking "(custom)" never happens through fzf typing — fzf is selection-only
//! to avoid the picked-vs-typed ambiguity. Free-text comes from a follow-up
//! `selector.input` call. Ctrl-P goes back to the previous stage.
//!
//! Parts are joined with "-"; "(none)" parts contribute nothing. The final
//! joined name must be non-empty.

use crate::browser::selector::{InteractiveSelector, SelectItem, StageOutcome};
use crate::config::model::{EzConfig, StageKind};
use crate::error::{EzError, Result};

const NONE_VALUE: &str = "__none__";
const NONE_DISPLAY: &str = "(none)";
const CUSTOM_VALUE: &str = "__custom__";
const CUSTOM_DISPLAY: &str = "(custom)";

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
                let value = handle_pick(&kind, &raw, selector, &prompt)?;
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
                            // Final stage cannot be empty; re-prompt.
                            eprintln!("Session name cannot be empty.");
                            continue;
                        }
                        parts[idx] = None;
                        idx += 1;
                    }
                    PickResolution::Reprompt => continue,
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
    /// Stay on this stage (e.g. user cancelled the (custom) sub-prompt).
    Reprompt,
}

fn handle_pick(
    kind: &StageKind,
    raw: &str,
    selector: &dyn InteractiveSelector,
    prompt: &str,
) -> Result<PickResolution> {
    match kind {
        StageKind::Text => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                Ok(PickResolution::None)
            } else {
                Ok(PickResolution::Use(trimmed.to_string()))
            }
        }
        StageKind::Choice => {
            if raw == NONE_VALUE {
                Ok(PickResolution::None)
            } else if raw == CUSTOM_VALUE {
                let typed = selector.input(prompt, None)?;
                let trimmed = typed.trim();
                if trimmed.is_empty() {
                    // User Esc'd the input prompt — go back to the choice list.
                    Ok(PickResolution::Reprompt)
                } else {
                    Ok(PickResolution::Use(trimmed.to_string()))
                }
            } else {
                Ok(PickResolution::Use(raw.to_string()))
            }
        }
    }
}

fn build_items(choices: &[String], include_none: bool) -> Vec<SelectItem> {
    let mut items: Vec<SelectItem> = choices
        .iter()
        .map(|v| SelectItem {
            display: v.clone(),
            value: v.clone(),
        })
        .collect();
    items.push(SelectItem {
        display: CUSTOM_DISPLAY.into(),
        value: CUSTOM_VALUE.into(),
    });
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
