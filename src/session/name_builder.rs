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
use crate::config::model::{EzConfig, NameBuilderMode, StageKind};
use crate::error::{EzError, Result};
use colored::Colorize;

const NONE_VALUE: &str = "__none__";
const NONE_DISPLAY: &str = "(none)";

/// Result of running the staged prompt.
pub enum NamePromptResult {
    /// User completed all stages with a non-empty name.
    Done(String),
    /// User cancelled (Esc/Ctrl-C) — caller should abort the operation.
    Cancelled,
}

/// Present the configured name builder modes and let the user pick one.
/// Returns `None` if the user cancelled.
fn select_mode(
    selector: &dyn InteractiveSelector,
    modes: &[NameBuilderMode],
) -> Result<Option<NameBuilderMode>> {
    let items: Vec<SelectItem> = modes
        .iter()
        .map(|m| {
            let (display, value) = match m {
                NameBuilderMode::FullName => ("Full name (type the whole name)", "full_name"),
                NameBuilderMode::BuildFromParts => (
                    "Build from parts (prefix → ticket → name)",
                    "build_from_parts",
                ),
                NameBuilderMode::GitHubPr => ("From GitHub PR (paste PR URL)", "github_pr"),
                NameBuilderMode::JiraUrl => ("From Jira URL (paste Jira link)", "jira_url"),
            };
            SelectItem {
                display: display.to_string(),
                value: value.to_string(),
            }
        })
        .collect();

    let picked = selector.select_one(&items, "naming mode", None)?;
    let Some(idx) = picked else {
        return Ok(None);
    };

    Ok(Some(modes[idx].clone()))
}

/// Run the configured mode selection, then dispatch to the appropriate name
/// builder. The default ("main") session bypasses this entirely — callers
/// handle that separately.
pub fn prompt_session_name(
    selector: &dyn InteractiveSelector,
    config: &EzConfig,
) -> Result<NamePromptResult> {
    let modes = &config.name_builder_modes;

    let mode = if modes.len() == 1 {
        modes[0].clone()
    } else if modes.is_empty() {
        return Ok(NamePromptResult::Cancelled);
    } else {
        match select_mode(selector, modes)? {
            Some(m) => m,
            None => return Ok(NamePromptResult::Cancelled),
        }
    };

    match mode {
        NameBuilderMode::FullName => prompt_full_name(selector),
        NameBuilderMode::BuildFromParts => prompt_staged(selector, config),
        NameBuilderMode::GitHubPr => prompt_github_pr(selector, config),
        NameBuilderMode::JiraUrl => prompt_jira_url(selector),
    }
}

/// `FullName` mode: single free-text prompt, reject empty input.
fn prompt_full_name(selector: &dyn InteractiveSelector) -> Result<NamePromptResult> {
    match selector.input_with_back("session name", None, false, None)? {
        StageOutcome::Picked(name) if !name.trim().is_empty() => {
            Ok(NamePromptResult::Done(name.trim().to_string()))
        }
        StageOutcome::Picked(_) => {
            eprintln!("Session name cannot be empty.");
            prompt_full_name(selector)
        }
        StageOutcome::Cancel | StageOutcome::Back => Ok(NamePromptResult::Cancelled),
    }
}

/// `BuildFromParts` mode: the existing multi-stage builder.
fn prompt_staged(
    selector: &dyn InteractiveSelector,
    config: &EzConfig,
) -> Result<NamePromptResult> {
    let stages = &config.session_name_stages;
    let mut parts: Vec<Option<String>> = vec![None; stages.len() + 1];
    let mut idx: usize = 0;

    loop {
        let is_final = idx == stages.len();
        let (prompt, kind, choices): (String, StageKind, &[String]) = if is_final {
            ("name".into(), StageKind::Text, &[])
        } else {
            let s = &stages[idx];
            (s.name.clone(), s.kind.clone(), s.choices.as_slice())
        };
        let allow_back = idx > 0;

        let so_far = join_parts(&parts[..idx]);
        let context = if so_far.is_empty() {
            None
        } else {
            Some(format!("{so_far}-"))
        };

        let outcome = match kind {
            StageKind::Choice => {
                let items = build_items(choices, !is_final);
                selector.select_with_back(&prompt, &items, allow_back, context.as_deref())?
            }
            StageKind::Text => {
                selector.input_with_back(&prompt, None, allow_back, context.as_deref())?
            }
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
                idx = idx.saturating_sub(1);
            }
            StageOutcome::Cancel => return Ok(NamePromptResult::Cancelled),
        }
    }
}

/// `GitHubPr` mode: paste a GitHub PR URL, extract PR number, optionally
/// resolve branch name via OnNameResolve plugin hook.
fn prompt_github_pr(
    selector: &dyn InteractiveSelector,
    config: &EzConfig,
) -> Result<NamePromptResult> {
    let re = regex::Regex::new(r"github\.com/[^/]+/[^/]+/pull/(\d+)").unwrap();

    loop {
        match selector.input_with_back("GitHub PR URL", None, false, None)? {
            StageOutcome::Picked(url) => {
                let url = url.trim();
                if let Some(caps) = re.captures(url) {
                    let pr_number = &caps[1];
                    let candidate = format!("pr{pr_number}");

                    eprint!("{}", "Resolving PR branch...".dimmed());
                    let resolved = crate::plugin::run_name_resolve_hook(url, &candidate, config);
                    eprintln!("\r{}", " ".repeat(30));

                    let name = match resolved {
                        Some(resolved_name) => resolved_name,
                        None => candidate,
                    };

                    return Ok(NamePromptResult::Done(name));
                } else {
                    eprintln!(
                        "{}",
                        "Could not extract PR number. Expected: https://github.com/<owner>/<repo>/pull/<number>"
                            .yellow()
                    );
                    continue;
                }
            }
            StageOutcome::Cancel | StageOutcome::Back => return Ok(NamePromptResult::Cancelled),
        }
    }
}

/// `JiraUrl` mode: paste a Jira URL, extract the ticket key, then
/// optionally append a descriptive suffix.
fn prompt_jira_url(selector: &dyn InteractiveSelector) -> Result<NamePromptResult> {
    let re = regex::Regex::new(r"/browse/([A-Z][A-Z0-9]+-\d+)").unwrap();
    loop {
        match selector.input_with_back("Jira URL", None, false, None)? {
            StageOutcome::Picked(url) => {
                let url = url.trim();
                if url.is_empty() {
                    return Ok(NamePromptResult::Cancelled);
                }
                if let Some(caps) = re.captures(url) {
                    let ticket = caps[1].to_string();
                    let context = Some(format!("{ticket}-"));
                    match prompt_final_suffix(selector, context.as_deref())? {
                        Some(suffix) if !suffix.is_empty() => {
                            return Ok(NamePromptResult::Done(format!("{ticket}-{suffix}")));
                        }
                        _ => {
                            return Ok(NamePromptResult::Done(ticket));
                        }
                    }
                } else {
                    eprintln!(
                        "Could not extract Jira ticket from URL. Expected format: .../browse/PROJ-123"
                    );
                    continue;
                }
            }
            StageOutcome::Cancel | StageOutcome::Back => return Ok(NamePromptResult::Cancelled),
        }
    }
}

/// Prompt for a final descriptive suffix. Used by the Jira mode (and
/// potentially others) after the structured part of the name is known.
/// `context` is shown as a header hint (e.g. `"PROJ-123-"`).
///
/// Returns `Some(trimmed_text)` on success, `None` on cancel/back/empty.
pub fn prompt_final_suffix(
    selector: &dyn InteractiveSelector,
    context: Option<&str>,
) -> Result<Option<String>> {
    match selector.input_with_back("name", None, false, context)? {
        StageOutcome::Picked(text) if !text.trim().is_empty() => Ok(Some(text.trim().to_string())),
        StageOutcome::Picked(_) => Ok(None),
        StageOutcome::Cancel | StageOutcome::Back => Ok(None),
    }
}

enum PickResolution {
    Use(String),
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
