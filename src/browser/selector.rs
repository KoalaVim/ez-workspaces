use std::io::Write;
use std::process::{Command, Stdio};

use crate::error::{EzError, Result};

/// An item presented in the interactive selector.
pub struct SelectItem {
    /// What the user sees in the list
    pub display: String,
    /// Internal identifier
    pub value: String,
}

/// Result of a selection with action keybinds.
pub enum ActionResult {
    /// User pressed Enter — enter/select the item at index.
    Select(usize),
    /// User pressed a keybind — action key + selected index.
    Action(String, usize),
    /// User cancelled (Escape/Ctrl-C).
    Cancel,
}

/// Outcome of a back-aware selection prompt used by multi-stage flows.
/// Selection-only — free-text input is a separate `input` call.
pub enum StageOutcome {
    /// User picked an item; the string is the SelectItem.value. Sentinel
    /// values like "(custom)" or "(none)" are matched by the caller.
    Picked(String),
    /// User asked to go back to the previous stage.
    Back,
    /// User cancelled the whole flow (Esc / Ctrl-C).
    Cancel,
}

/// Trait for interactive selection UIs. Default impl uses fzf.
/// Implementors can swap in skim, dialoguer, or a TUI framework.
pub trait InteractiveSelector {
    /// Present a list of items, return the selected item's index.
    fn select_one(
        &self,
        items: &[SelectItem],
        prompt: &str,
        preview_cmd: Option<&str>,
    ) -> Result<Option<usize>>;

    /// Present items and allow multiple selection (Tab to toggle in fzf).
    fn select_many(
        &self,
        items: &[SelectItem],
        prompt: &str,
    ) -> Result<Vec<usize>>;

    /// Prompt for free-text input with optional default.
    fn input(&self, prompt: &str, default: Option<&str>) -> Result<String>;

    /// Confirm yes/no.
    fn confirm(&self, prompt: &str, default: bool) -> Result<bool>;

    /// Selection-only prompt with optional Ctrl-P back keybind. Free-text
    /// input is intentionally not supported here — call `input` separately
    /// after the user picks a "(custom)"-style sentinel row.
    fn select_with_back(
        &self,
        prompt: &str,
        items: &[SelectItem],
        allow_back: bool,
    ) -> Result<StageOutcome>;

    /// Free-text input with optional Ctrl-P back keybind. Returns
    /// `StageOutcome::Picked(s)` (where `s` may be empty), `Back`, or
    /// `Cancel`. Used by free-text stages that still want back navigation.
    fn input_with_back(
        &self,
        prompt: &str,
        default: Option<&str>,
        allow_back: bool,
    ) -> Result<StageOutcome>;

    /// Present items with keybind actions. Returns which key was pressed + selected index.
    fn select_with_actions(
        &self,
        items: &[SelectItem],
        prompt: &str,
        preview_cmd: Option<&str>,
        expect_keys: &[&str],
        header: Option<&str>,
    ) -> Result<ActionResult>;
}

/// fzf-based interactive selector.
pub struct FzfSelector {
    pub height: String,
    pub extra_opts: Option<String>,
}

impl FzfSelector {
    pub fn new(fzf_config: &crate::config::model::FzfConfig) -> Result<Self> {
        // Verify fzf is available
        which::which("fzf").map_err(|_| {
            EzError::SelectorUnavailable(
                "fzf not found in PATH. Install fzf: https://github.com/junegunn/fzf".into(),
            )
        })?;
        Ok(Self {
            height: fzf_config.height.clone(),
            extra_opts: fzf_config.extra_opts.clone(),
        })
    }
}

impl InteractiveSelector for FzfSelector {
    fn select_one(
        &self,
        items: &[SelectItem],
        prompt: &str,
        preview_cmd: Option<&str>,
    ) -> Result<Option<usize>> {
        if items.is_empty() {
            return Ok(None);
        }

        // When preview is used, send "value\tdisplay" so fzf's {} gives
        // the value (path) to the preview command, while --with-nth shows
        // only the display portion to the user.
        let use_value_prefix = preview_cmd.is_some();

        let mut args = vec![
            "--prompt".to_string(),
            format!("{prompt}> "),
            "--height".to_string(),
            self.height.clone(),
            "--layout".to_string(),
            "reverse".to_string(),
            "--ansi".to_string(),
        ];

        if use_value_prefix {
            args.push("--delimiter".to_string());
            args.push("\t".to_string());
            args.push("--with-nth".to_string());
            args.push("2..".to_string());
        }

        if let Some(preview) = preview_cmd {
            // {1} extracts the first tab-delimited field (the value/path)
            let cmd = preview.replace("{}", "{1}");
            args.push("--preview".to_string());
            args.push(cmd);
            args.push("--preview-window".to_string());
            args.push("right:50%".to_string());
        }

        if let Some(opts) = &self.extra_opts {
            args.extend(opts.split_whitespace().map(String::from));
        }

        let mut child = Command::new("fzf")
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| EzError::SelectorUnavailable(e.to_string()))?;

        // Write items to fzf's stdin
        if let Some(mut stdin) = child.stdin.take() {
            for item in items {
                if use_value_prefix {
                    let _ = writeln!(stdin, "{}\t{}", item.value, item.display);
                } else {
                    let _ = writeln!(stdin, "{}", item.display);
                }
            }
        }

        let output = child.wait_with_output()?;

        log::debug!("select_one: fzf exit={}, stdout={:?}", output.status, String::from_utf8_lossy(&output.stdout));

        if !output.status.success() {
            return Ok(None);
        }

        let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // When using value prefix, match on value (first field) since fzf
        // strips ANSI codes from the display portion in its output.
        let (match_field, match_on_value) = if use_value_prefix {
            let value = raw.splitn(2, '\t')
                .next()
                .unwrap_or(&raw)
                .to_string();
            (value, true)
        } else {
            (raw, false)
        };
        log::debug!("select_one: match_field={:?} on_value={}", match_field, match_on_value);
        let index = if match_on_value {
            items.iter().position(|item| item.value == match_field)
        } else {
            items.iter().position(|item| item.display == match_field)
        };
        log::debug!("select_one: matched index={:?}", index);
        Ok(index)
    }

    fn select_many(
        &self,
        items: &[SelectItem],
        prompt: &str,
    ) -> Result<Vec<usize>> {
        if items.is_empty() {
            return Ok(Vec::new());
        }

        let mut args = vec![
            "--prompt".to_string(),
            format!("{prompt}> "),
            "--multi".to_string(),
            "--height".to_string(),
            self.height.clone(),
            "--layout".to_string(),
            "reverse".to_string(),
            "--ansi".to_string(),
        ];

        if let Some(opts) = &self.extra_opts {
            args.extend(opts.split_whitespace().map(String::from));
        }

        let mut child = Command::new("fzf")
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| EzError::SelectorUnavailable(e.to_string()))?;

        if let Some(mut stdin) = child.stdin.take() {
            for item in items {
                let _ = writeln!(stdin, "{}", item.display);
            }
        }

        let output = child.wait_with_output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let selected_lines: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        let indices: Vec<usize> = selected_lines
            .iter()
            .filter_map(|sel| items.iter().position(|item| item.display == *sel))
            .collect();

        Ok(indices)
    }

    fn input(&self, prompt: &str, default: Option<&str>) -> Result<String> {
        let mut args = vec![
            "--prompt".to_string(),
            format!("{prompt}: "),
            "--print-query".to_string(),
            "--height".to_string(),
            "~10%".to_string(),
            "--layout".to_string(),
            "reverse".to_string(),
        ];

        if let Some(def) = default {
            args.push("--query".to_string());
            args.push(def.to_string());
        }

        let mut child = Command::new("fzf")
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| EzError::SelectorUnavailable(e.to_string()))?;

        // Close stdin immediately (no items to select from)
        drop(child.stdin.take());

        let output = child.wait_with_output()?;
        let result = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();

        if result.is_empty() {
            if let Some(def) = default {
                return Ok(def.to_string());
            }
        }
        Ok(result)
    }

    fn confirm(&self, prompt: &str, default: bool) -> Result<bool> {
        let default_hint = if default { "Y/n" } else { "y/N" };
        eprint!("{prompt} [{default_hint}]: ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input.is_empty() {
            return Ok(default);
        }
        Ok(input == "y" || input == "yes")
    }

    fn select_with_back(
        &self,
        prompt: &str,
        items: &[SelectItem],
        allow_back: bool,
    ) -> Result<StageOutcome> {
        if items.is_empty() {
            return Ok(StageOutcome::Cancel);
        }

        // Tab-prefix items: line is "<value>\t<display>". --with-nth 2..
        // shows only the display portion, but stdout always carries the raw
        // line so we can match on value reliably.
        // --print-query lets the user type a custom value and Enter to use it
        // (when the typed query doesn't match any item). With both
        // --print-query and --expect set, fzf's stdout order is: query, key,
        // selection — which is what we parse below.
        let mut args = vec![
            "--prompt".to_string(),
            format!("{prompt}> "),
            "--height".to_string(),
            self.height.clone(),
            "--layout".to_string(),
            "reverse".to_string(),
            "--ansi".to_string(),
            "--delimiter".to_string(),
            "\t".to_string(),
            "--with-nth".to_string(),
            "2..".to_string(),
            "--print-query".to_string(),
        ];

        let header_hint = if allow_back {
            "Enter: pick or use typed value · Ctrl-P: back · Esc: cancel"
        } else {
            "Enter: pick or use typed value · Esc: cancel"
        };
        args.push("--header".to_string());
        args.push(header_hint.to_string());
        args.push("--header-first".to_string());

        if allow_back {
            args.push("--expect".to_string());
            args.push("ctrl-p".to_string());
        }

        if let Some(opts) = &self.extra_opts {
            args.extend(opts.split_whitespace().map(String::from));
        }

        let mut child = Command::new("fzf")
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| EzError::SelectorUnavailable(e.to_string()))?;

        if let Some(mut stdin) = child.stdin.take() {
            for item in items {
                let _ = writeln!(stdin, "{}\t{}", item.value, item.display);
            }
        }

        let output = child.wait_with_output()?;
        let code = output.status.code().unwrap_or(-1);

        // Exit codes with --print-query:
        //   0  → Enter on a matching list item
        //   1  → Enter with no matching item (typed-query path)
        //   130 → Esc / Ctrl-C
        if code == 130 {
            log::debug!("select_with_back: cancelled (exit 130)");
            return Ok(StageOutcome::Cancel);
        }

        let raw = String::from_utf8_lossy(&output.stdout);
        let mut lines = raw.lines();

        // Order: query, [key], [selection]. The key line only appears when
        // --expect is set; the selection line only appears when an item
        // actually matched (exit 0).
        let query_line = lines.next().unwrap_or("").to_string();
        let key_line = if allow_back {
            lines.next().unwrap_or("").to_string()
        } else {
            String::new()
        };
        let selection_line = lines.next().unwrap_or("").to_string();

        log::debug!(
            "select_with_back: query={query_line:?} key={key_line:?} selection={selection_line:?} exit={code}"
        );

        if key_line == "ctrl-p" {
            return Ok(StageOutcome::Back);
        }

        let selection_value = selection_line
            .splitn(2, '\t')
            .next()
            .unwrap_or("")
            .to_string();
        let query = query_line.trim().to_string();

        // Picked an item from the list.
        if !selection_value.is_empty() {
            return Ok(StageOutcome::Picked(selection_value));
        }
        // No item matched → use the typed query if it's non-empty.
        if !query.is_empty() {
            return Ok(StageOutcome::Picked(query));
        }
        // Empty query, no selection → user pressed Enter on nothing.
        Ok(StageOutcome::Cancel)
    }

    fn input_with_back(
        &self,
        prompt: &str,
        default: Option<&str>,
        allow_back: bool,
    ) -> Result<StageOutcome> {
        // fzf with no items, --print-query, and (optionally) --expect=ctrl-p.
        // With both flags, fzf's output ordering is: query, key. (Selection
        // is absent because there are no items.)
        let mut args = vec![
            "--prompt".to_string(),
            format!("{prompt}: "),
            "--print-query".to_string(),
            "--height".to_string(),
            "~10%".to_string(),
            "--layout".to_string(),
            "reverse".to_string(),
        ];

        let header_hint = if allow_back {
            "Enter: confirm · Ctrl-P: back · Esc: cancel"
        } else {
            "Enter: confirm · Esc: cancel"
        };
        args.push("--header".to_string());
        args.push(header_hint.to_string());
        args.push("--header-first".to_string());

        if allow_back {
            args.push("--expect".to_string());
            args.push("ctrl-p".to_string());
        }

        if let Some(def) = default {
            args.push("--query".to_string());
            args.push(def.to_string());
        }

        let mut child = Command::new("fzf")
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| EzError::SelectorUnavailable(e.to_string()))?;

        // No items.
        drop(child.stdin.take());

        let output = child.wait_with_output()?;
        let code = output.status.code().unwrap_or(-1);

        if code == 130 {
            return Ok(StageOutcome::Cancel);
        }

        let raw = String::from_utf8_lossy(&output.stdout);
        let mut lines = raw.lines();
        // With --print-query, line 1 is the query. With --expect, the next
        // line is the pressed key (empty for Enter). When --expect is not
        // set, there's no key line at all.
        let query = lines.next().unwrap_or("").to_string();
        let key = if allow_back {
            lines.next().unwrap_or("").to_string()
        } else {
            String::new()
        };

        log::debug!(
            "input_with_back: query={query:?} key={key:?} exit={code} allow_back={allow_back}"
        );

        if key == "ctrl-p" {
            return Ok(StageOutcome::Back);
        }

        // Empty query on Enter is allowed — caller decides what to do.
        Ok(StageOutcome::Picked(query))
    }

    fn select_with_actions(
        &self,
        items: &[SelectItem],
        prompt: &str,
        preview_cmd: Option<&str>,
        expect_keys: &[&str],
        header: Option<&str>,
    ) -> Result<ActionResult> {
        if items.is_empty() {
            return Ok(ActionResult::Cancel);
        }

        // Always tab-prefix items with their value so matching works whether or
        // not fzf strips ANSI codes from stdout. The visible portion is set via
        // --with-nth 2.. below.
        let use_value_prefix = true;

        let mut args = vec![
            "--prompt".to_string(),
            format!("{prompt}> "),
            "--height".to_string(),
            self.height.clone(),
            "--layout".to_string(),
            "reverse".to_string(),
            "--ansi".to_string(),
        ];

        // --expect captures keybinds; fzf outputs the pressed key on line 1
        if !expect_keys.is_empty() {
            args.push("--expect".to_string());
            args.push(expect_keys.join(","));
        }

        if let Some(hdr) = header {
            args.push("--header".to_string());
            args.push(hdr.to_string());
            args.push("--header-first".to_string());
        }

        if use_value_prefix {
            args.push("--delimiter".to_string());
            args.push("\t".to_string());
            args.push("--with-nth".to_string());
            args.push("2..".to_string());
        }

        if let Some(preview) = preview_cmd {
            let cmd = preview.replace("{}", "{1}");
            args.push("--preview".to_string());
            args.push(cmd);
            args.push("--preview-window".to_string());
            args.push("right:50%".to_string());
        }

        if let Some(opts) = &self.extra_opts {
            args.extend(opts.split_whitespace().map(String::from));
        }

        let mut child = Command::new("fzf")
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| EzError::SelectorUnavailable(e.to_string()))?;

        if let Some(mut stdin) = child.stdin.take() {
            for item in items {
                if use_value_prefix {
                    let _ = writeln!(stdin, "{}\t{}", item.value, item.display);
                } else {
                    let _ = writeln!(stdin, "{}", item.display);
                }
            }
        }

        let output = child.wait_with_output()?;

        log::debug!("fzf exit status: {}", output.status);
        log::debug!("fzf stdout raw: {:?}", String::from_utf8_lossy(&output.stdout));

        if !output.status.success() {
            log::debug!("fzf exited with non-zero status, returning Cancel");
            return Ok(ActionResult::Cancel);
        }

        let raw = String::from_utf8_lossy(&output.stdout);
        let mut lines = raw.lines();

        // With --expect, first line is the key pressed (empty string = Enter)
        let key_line = lines.next().unwrap_or("").trim().to_string();
        let selection_line = lines.next().unwrap_or("").trim().to_string();

        log::debug!("fzf key_line: {:?}", key_line);
        log::debug!("fzf selection_line: {:?}", selection_line);

        // When using value prefix (tab-delimited), match on value (first field)
        // because fzf strips ANSI codes from the display portion in its output.
        let (match_field, match_on_value) = if use_value_prefix {
            let value = selection_line
                .splitn(2, '\t')
                .next()
                .unwrap_or(&selection_line)
                .to_string();
            (value, true)
        } else {
            (selection_line, false)
        };

        log::debug!("fzf match_field: {:?} (on_value={})", match_field, match_on_value);

        let index = match if match_on_value {
            items.iter().position(|item| item.value == match_field)
        } else {
            items.iter().position(|item| item.display == match_field)
        } {
            Some(idx) => idx,
            None => {
                log::debug!("fzf match failed, items:");
                for (i, item) in items.iter().enumerate() {
                    log::debug!("  [{}] display={:?} value={:?}", i, item.display, item.value);
                }
                return Ok(ActionResult::Cancel);
            }
        };

        log::debug!("fzf matched index: {}, key: {:?}", index, key_line);

        if key_line.is_empty() {
            Ok(ActionResult::Select(index))
        } else {
            Ok(ActionResult::Action(key_line, index))
        }
    }
}
