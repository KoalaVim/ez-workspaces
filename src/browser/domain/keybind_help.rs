//! Keybind help tables and section headers used by the preview panes.

use std::fmt::Write;

use colored::Colorize;

use crate::config;
use crate::plugin;

fn fmt_key(k: &str) -> String {
    match k {
        "tab" => return "Tab".into(),
        "btab" | "shift-tab" => return "Shift-Tab".into(),
        _ => {}
    }
    k.replace("alt-", "Alt-")
        .replace("ctrl-", "Ctrl-")
        .replace("shift-", "Shift-")
}

fn ansi_pad(s: &str, width: usize) -> String {
    let visible = console::measure_text_width(s);
    if visible >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - visible))
    }
}

const KEYBIND_KEY_W: usize = 8;

fn center_in(text: &str, width: usize) -> String {
    let visible = console::measure_text_width(text);
    if visible >= width {
        return text.to_string();
    }
    let pad_left = (width - visible) / 2;
    format!("{}{}", " ".repeat(pad_left), text)
}

fn keybind_col_width(entries: &[(String, String)]) -> usize {
    let max_entry = entries
        .iter()
        .map(|(_k, v)| {
            // "  " + key padded to KEYBIND_KEY_W + " " + description
            2 + KEYBIND_KEY_W + 1 + console::measure_text_width(v)
        })
        .max()
        .unwrap_or(20);
    max_entry + 2 // breathing room
}

fn print_keybind_table(
    out: &mut impl Write,
    left_title: &str,
    right_title: &str,
    left: &[(String, String)],
    right: &[(String, String)],
) {
    let col_w = keybind_col_width(left);
    let sep = "│".dimmed();

    // Header
    let left_h = center_in(&left_title.bold().cyan().to_string(), col_w);
    let right_h = center_in(&right_title.bold().cyan().to_string(), col_w);
    outln!(out, "{}{sep}{}", ansi_pad(&left_h, col_w), right_h);
    outln!(
        out,
        "{}",
        format!("{}┼{}", "─".repeat(col_w), "─".repeat(col_w)).dimmed()
    );

    // Rows
    let rows = left.len().max(right.len());
    for i in 0..rows {
        let left_cell = if i < left.len() {
            format!("  {} {}", ansi_pad(&left[i].0, KEYBIND_KEY_W), left[i].1)
        } else {
            String::new()
        };
        let right_cell = if i < right.len() {
            format!(" {} {}", ansi_pad(&right[i].0, KEYBIND_KEY_W), right[i].1)
        } else {
            String::new()
        };
        outln!(out, "{}{sep}{}", ansi_pad(&left_cell, col_w), right_cell);
    }
}

pub(super) fn preview_keybind_help(out: &mut impl Write) {
    let config = config::load().unwrap_or_default();
    let keybinds = &config.keybinds;

    preview_section(out, "Keybinds");

    let mut left: Vec<(String, String)> = vec![
        ("Enter".bold().green().to_string(), "Enter session".into()),
        (
            fmt_key(&keybinds.new_session).bold().yellow().to_string(),
            "New child session".into(),
        ),
        (
            fmt_key(&keybinds.new_bare_session)
                .bold()
                .yellow()
                .to_string(),
            "New bare session".into(),
        ),
        (
            fmt_key(&keybinds.session_from_dirty)
                .bold()
                .yellow()
                .to_string(),
            "From dirty".into(),
        ),
        (
            fmt_key(&keybinds.rename_session)
                .bold()
                .yellow()
                .to_string(),
            "Rename".into(),
        ),
        (
            fmt_key(&keybinds.delete_session).bold().red().to_string(),
            "Delete".into(),
        ),
        (
            fmt_key(&keybinds.edit_labels).bold().magenta().to_string(),
            "Edit labels".into(),
        ),
        (
            fmt_key(&keybinds.cd_session).bold().yellow().to_string(),
            "Cd into worktree".into(),
        ),
        (
            fmt_key(&keybinds.note_open).bold().yellow().to_string(),
            "Open note".into(),
        ),
        (
            fmt_key(&keybinds.note_cd).bold().yellow().to_string(),
            "Cd to notes".into(),
        ),
    ];
    for pb in plugin::collect_plugin_binds("session", &config).unwrap_or_default() {
        let desc = pb.description.as_deref().unwrap_or(&pb.label);
        left.push((fmt_key(&pb.key).bold().cyan().to_string(), desc.into()));
    }

    let mut right: Vec<(String, String)> = vec![
        (
            fmt_key(&keybinds.sort_toggle).bold().yellow().to_string(),
            "Toggle sort".into(),
        ),
        ("Esc".bold().dimmed().to_string(), "Go back".into()),
        ("Ctrl-C".bold().dimmed().to_string(), "Quit".into()),
    ];
    for pv in plugin::collect_plugin_views("session", &config).unwrap_or_default() {
        right.push((
            fmt_key(&pv.key).bold().cyan().to_string(),
            format!("{} view", pv.label),
        ));
    }

    print_keybind_table(out, "Session", "Menu", &left, &right);
}

pub(super) fn preview_main_keybind_help(out: &mut impl Write) {
    let config = config::load().unwrap_or_default();
    let keybinds = &config.keybinds;

    preview_section(out, "Keybinds");

    let left: Vec<(String, String)> = vec![
        ("Enter".bold().green().to_string(), "Open sessions".into()),
        (
            fmt_key(&keybinds.clone_repo).bold().yellow().to_string(),
            "Clone repo".into(),
        ),
        (
            fmt_key(&keybinds.edit_labels).bold().magenta().to_string(),
            "Edit labels".into(),
        ),
        (
            fmt_key(&keybinds.remove_repo).bold().red().to_string(),
            "Remove repo".into(),
        ),
        (
            fmt_key(&keybinds.sort_toggle).bold().yellow().to_string(),
            "Toggle sort".into(),
        ),
        ("Esc".bold().dimmed().to_string(), "Quit".into()),
    ];

    let mut right: Vec<(String, String)> = vec![
        (
            fmt_key(&keybinds.view_tree).bold().yellow().to_string(),
            "Tree".into(),
        ),
        (
            fmt_key(&keybinds.view_workspace)
                .bold()
                .yellow()
                .to_string(),
            "Workspace".into(),
        ),
        (
            fmt_key(&keybinds.view_repo).bold().yellow().to_string(),
            "Repo".into(),
        ),
        (
            fmt_key(&keybinds.view_owner).bold().yellow().to_string(),
            "Owner".into(),
        ),
        (
            fmt_key(&keybinds.view_label).bold().yellow().to_string(),
            "Label".into(),
        ),
        (
            format!(
                "{}/{}",
                fmt_key(&keybinds.view_next),
                fmt_key(&keybinds.view_prev)
            )
            .bold()
            .yellow()
            .to_string(),
            "Next/prev view".into(),
        ),
    ];
    for pv in plugin::collect_plugin_views("repo", &config).unwrap_or_default() {
        right.push((fmt_key(&pv.key).bold().cyan().to_string(), pv.label.clone()));
    }

    print_keybind_table(out, "Repo", "Views", &left, &right);
}

pub(super) fn preview_section(out: &mut impl Write, title: &str) {
    let bar = "─".repeat(40);
    outln!(out, "{}", format!("┌{bar}").dimmed());
    outln!(out, "{} {}", "│".dimmed(), title.bold().cyan());
    outln!(out, "{}", format!("└{bar}").dimmed());
}
