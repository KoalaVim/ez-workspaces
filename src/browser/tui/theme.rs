//! Visual language of the native browser: palette, glyphs and the small
//! shared widgets (rounded panels, key chips, spinner, search bar).
//!
//! Colors use the terminal's named ANSI palette so the browser follows the
//! user's terminal theme out of the box (no truecolor assumptions).

use std::sync::OnceLock;
use std::time::Instant;

use ratatui::layout::Alignment;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Padding};

/// Primary accent: focus, selection gutter, active tab, prompt.
pub const ACCENT: Color = Color::Magenta;
/// Secondary accent: key chips and counters.
pub const ACCENT2: Color = Color::Cyan;
/// Unfocused borders and separators (ANSI bright black).
pub const LINE: Color = Color::DarkGray;
/// Background of the selected row.
pub const SELECTED_BG: Color = Color::Indexed(236);

pub const PROMPT: &str = "❯ ";
pub const GUTTER: &str = "▌ ";
pub const SEP: &str = " › ";
const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn muted() -> Style {
    Style::default().fg(LINE)
}

pub fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

pub fn accent_bold() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

/// Style for fuzzy-matched characters.
pub fn matched() -> Style {
    Style::default()
        .fg(ACCENT)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
}

/// Style for the selected row (the gutter glyph carries the accent).
pub fn selected() -> Style {
    Style::default()
        .bg(SELECTED_BG)
        .add_modifier(Modifier::BOLD)
}

/// A rounded panel; `focused` panels get the accent border and title.
pub fn panel(title: &str, focused: bool) -> Block<'static> {
    let (border_style, title_style) = if focused {
        (Style::default().fg(ACCENT), accent_bold())
    } else {
        (muted(), Style::default().add_modifier(Modifier::BOLD))
    };
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .padding(Padding::horizontal(1));
    if !title.is_empty() {
        block = block.title(Line::from(Span::styled(format!(" {title} "), title_style)));
    }
    block
}

/// Right-aligned secondary title for a panel.
pub fn right_title(spans: Vec<Span<'static>>) -> Line<'static> {
    Line::from(spans).alignment(Alignment::Right)
}

/// Current braille spinner frame (time based, so every redraw advances it).
pub fn spinner() -> &'static str {
    static START: OnceLock<Instant> = OnceLock::new();
    let ms = START.get_or_init(Instant::now).elapsed().as_millis();
    SPINNER[(ms / 80) as usize % SPINNER.len()]
}

/// Compact form of an fzf key spec: `ctrl-t` → `^t`, `alt-n` → `⌥n`,
/// `alt-shift-n` → `⌥N`.
pub fn short_key(spec: &str) -> String {
    let lower = spec.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("alt-shift-") {
        return format!("⌥{}", rest.to_ascii_uppercase());
    }
    if let Some(rest) = spec
        .strip_prefix("alt-")
        .or_else(|| spec.strip_prefix("Alt-"))
    {
        return format!("⌥{rest}");
    }
    if let Some(rest) = lower.strip_prefix("ctrl-") {
        return format!("^{rest}");
    }
    match lower.as_str() {
        "enter" => "⏎".into(),
        "esc" => "esc".into(),
        _ => spec.to_string(),
    }
}

/// A key hint: the key in the secondary accent, the label muted.
pub fn chip(key: &str, label: &str) -> Vec<Span<'static>> {
    vec![
        Span::styled(
            short_key(key),
            Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {label}"), dim()),
        Span::raw("   "),
    ]
}

/// Replace the shared preview's boxed section headers (`┌──` / `│ Title` /
/// `└──`, kept for fzf) with a single accent heading line.
pub fn section_headers(text: Text<'static>) -> Text<'static> {
    let plain = |l: &Line| {
        l.spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<String>()
    };
    let mut out: Vec<Line<'static>> = Vec::with_capacity(text.lines.len());
    let lines = text.lines;
    let mut i = 0;
    while i < lines.len() {
        if i + 2 < lines.len() {
            let (a, b, c) = (plain(&lines[i]), plain(&lines[i + 1]), plain(&lines[i + 2]));
            if a.starts_with("┌─") && c.starts_with("└─") {
                if let Some(title) = b.strip_prefix("│ ") {
                    out.push(Line::from(vec![
                        Span::styled("▍", Style::default().fg(ACCENT)),
                        Span::styled(
                            format!(" {}", title.trim()),
                            Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD),
                        ),
                    ]));
                    i += 3;
                    continue;
                }
            }
        }
        out.push(lines[i].clone());
        i += 1;
    }
    Text::from(out)
}

/// Severity glyph and color for notices.
pub fn notice_style(level: crate::browser::domain::Level) -> (&'static str, Color) {
    use crate::browser::domain::Level;
    match level {
        Level::Info => ("●", Color::Green),
        Level::Warn => ("▲", Color::Yellow),
        Level::Error => ("✖", Color::Red),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_keys() {
        assert_eq!(short_key("ctrl-t"), "^t");
        assert_eq!(short_key("alt-n"), "⌥n");
        assert_eq!(short_key("alt-shift-n"), "⌥N");
        assert_eq!(short_key("alt-I"), "⌥I");
        assert_eq!(short_key("enter"), "⏎");
        assert_eq!(short_key("f2"), "f2");
    }

    #[test]
    fn boxed_section_headers_become_one_line() {
        let text = Text::from(vec![
            Line::from("■ repo"),
            Line::from("┌────"),
            Line::from("│ Sessions"),
            Line::from("└────"),
            Line::from("  main"),
        ]);
        let out = section_headers(text);
        let plain: Vec<String> = out
            .lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect();
        assert_eq!(plain, vec!["■ repo", "▍ Sessions", "  main"]);
    }

    #[test]
    fn spinner_frame_is_a_braille_glyph() {
        assert!(SPINNER.contains(&spinner()));
    }
}
