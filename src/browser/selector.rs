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
        let selected = if use_value_prefix {
            raw.splitn(2, '\t')
                .nth(1)
                .unwrap_or(&raw)
                .to_string()
        } else {
            raw
        };
        log::debug!("select_one: selected={:?}", selected);
        let index = items.iter().position(|item| item.display == selected);
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

        let selected_display = if use_value_prefix {
            selection_line
                .splitn(2, '\t')
                .nth(1)
                .unwrap_or(&selection_line)
                .to_string()
        } else {
            selection_line
        };

        log::debug!("fzf selected_display: {:?}", selected_display);

        let index = match items.iter().position(|item| item.display == selected_display) {
            Some(idx) => idx,
            None => {
                log::debug!("fzf display match failed, items:");
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
