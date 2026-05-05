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
use crate::config::model::EzConfig;
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
        let prompt: String = if is_final {
            "name".into()
        } else {
            stages[idx].name.clone()
        };
        let choices: &[String] = if is_final { &[] } else { &stages[idx].choices };
        let items = build_items(choices, /*include_none=*/ !is_final);

        match selector.select_with_back(&prompt, &items, /*allow_back=*/ idx > 0)? {
            StageOutcome::Picked(value) => {
                if value == NONE_VALUE {
                    parts[idx] = None;
                    idx += 1;
                } else if value == CUSTOM_VALUE {
                    let typed = selector.input(&prompt, None)?;
                    let trimmed = typed.trim();
                    if trimmed.is_empty() {
                        // Empty/Esc on the input prompt — fall back to the
                        // selection screen rather than treating as cancel.
                        continue;
                    }
                    parts[idx] = Some(trimmed.to_string());
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
                } else {
                    parts[idx] = Some(value);
                    if is_final {
                        // The final stage has no preset choices, so this
                        // branch is unreachable in practice. Defensive:
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
