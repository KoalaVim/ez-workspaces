//! Frame layout: list (+ preview) / status line / prompt line.

use ratatui::layout::{Constraint, Layout, Position};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::App;
use crate::browser::domain::Level as Severity;
use crate::browser::tui::list::display_line;

pub(super) fn draw(app: &mut App, frame: &mut Frame) {
    let [body, status, prompt] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let ratio = app.config.native.preview_ratio.min(90);
    let level = app.nav.top();
    let preview_req = level.preview_req();
    let (list_area, preview_area) = if level.has_preview() && ratio > 0 {
        let [l, p] = Layout::horizontal([
            Constraint::Percentage(100 - ratio),
            Constraint::Percentage(ratio),
        ])
        .areas(body);
        (l, Some(p))
    } else {
        (body, None)
    };
    if let Some(area) = preview_area {
        app.preview
            .render(frame, area, preview_req.as_ref(), app.data_gen);
    }

    let dim = Style::default().add_modifier(Modifier::DIM);
    let level = app.nav.top_mut();
    if level.list.total() == 0 && !level.loading {
        frame.render_widget(Paragraph::new(level.empty_hint()).style(dim), list_area);
    } else {
        level.list.render(frame, list_area);
    }

    let level = app.nav.top();
    if let Some((text, severity)) = &app.notice {
        let color = match severity {
            Severity::Info => Color::Green,
            Severity::Warn => Color::Yellow,
            Severity::Error => Color::Red,
        };
        let mut line = display_line(text, &[]);
        line.spans
            .insert(0, Span::styled(" ", Style::default().fg(color)));
        frame.render_widget(Paragraph::new(line), status);
    } else {
        let crumbs: Vec<&str> = app.nav.iter().map(|l| l.title.as_str()).collect();
        let kb = &app.config.keybinds;
        let mut spans = vec![
            Span::styled(
                format!(" view: {}", crumbs.join(" > ")),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}/{}", level.list.len(), level.list.total()),
                dim,
            ),
        ];
        if level.supports_sort() {
            spans.push(Span::styled(
                format!("  sort: {} ({})", level.sort.label(), kb.sort_toggle),
                dim,
            ));
        }
        if level.loading {
            spans.push(Span::styled("  loading...", dim));
        } else if level.stale {
            spans.push(Span::styled("  (cached, refreshing)", dim));
        }
        if let Some(err) = &level.error {
            spans.push(Span::raw(format!("  error: {err}")));
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), status);
    }

    let text = format!("> {}", level.list.query());
    let cursor_x = prompt.x + text.chars().count() as u16;
    frame.render_widget(Paragraph::new(text), prompt);
    frame.set_cursor_position(Position::new(cursor_x, prompt.y));
}
