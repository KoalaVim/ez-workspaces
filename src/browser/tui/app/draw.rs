//! Frame layout, top to bottom:
//!
//! ```text
//! ╭─ ◆ ez ─────────────────────────────── tree › alpha-api ─╮
//! │ ❯ query                                    ⠋  12 / 40   │  search bar
//! ╰──────────────────────────────────────────────────────────╯
//!   Tree ^t   Workspace ^w   Repo ^e   Owner ^o   Label ^g      view tabs
//! ╭─ Sessions ──────────── sort LRU ─╮╭─ Preview ────────────╮
//! │▌ feat-one (main)                 ││ ■ feat-one           │  list + preview
//! ╰──────────────────────────────────╯╰──────────────────────╯
//!  ⏎ open   ⌥n new   ⌥r rename   ⌥d delete   esc back            key hints
//! ```

use ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use super::keymap::RowCtx;
use super::App;
use crate::browser::tui::level::Level;
use crate::browser::tui::list::display_line;
use crate::browser::tui::theme::{self, ACCENT, ACCENT2};

/// Below this width the preview pane is hidden.
const MIN_PREVIEW_WIDTH: u16 = 80;

pub(super) fn draw(app: &mut App, frame: &mut Frame) {
    let area = frame.area();
    let show_tabs = area.height >= 12;
    let [search, tabs, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(u16::from(show_tabs)),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(area);

    draw_search(app, frame, search);
    if show_tabs {
        draw_tabs(app, frame, tabs);
    }
    draw_body(app, frame, body);
    draw_footer(app, frame, footer);
}

/// Whether something is loading (keeps the spinner animating).
pub(super) fn animating(app: &App) -> bool {
    let level = app.nav.top();
    level.loading || level.stale || app.branch_inflight > 0
}

fn draw_search(app: &App, frame: &mut Frame, area: Rect) {
    let level = app.nav.top();
    let crumbs: Vec<&str> = app.nav.iter().map(|l| l.title.as_str()).collect();
    let mut trail: Vec<Span> = Vec::new();
    for (i, crumb) in crumbs.iter().enumerate() {
        if i > 0 {
            trail.push(Span::styled(theme::SEP, theme::muted()));
        }
        let style = if i + 1 == crumbs.len() {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            theme::dim()
        };
        trail.push(Span::styled(crumb.to_string(), style));
    }
    trail.push(Span::raw(" "));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .padding(ratatui::widgets::Padding::horizontal(1))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("◆ ez", theme::accent_bold()),
            Span::raw(" "),
        ]))
        .title(theme::right_title(trail));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut status: Vec<Span> = Vec::new();
    if animating(app) {
        let label = if level.stale {
            " refreshing"
        } else {
            " loading"
        };
        status.push(Span::styled(theme::spinner(), Style::default().fg(ACCENT)));
        status.push(Span::styled(label, theme::dim()));
        status.push(Span::raw("   "));
    }
    status.push(Span::styled(
        level.list.len().to_string(),
        Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD),
    ));
    status.push(Span::styled(
        format!(" / {} ", level.list.total()),
        theme::dim(),
    ));
    let status_w = status.iter().map(|s| s.width()).sum::<usize>() as u16;
    let [input, counter] =
        Layout::horizontal([Constraint::Min(1), Constraint::Length(status_w)]).areas(inner);

    let query = level.list.query();
    let mut line = vec![Span::styled(theme::PROMPT, theme::accent_bold())];
    if query.is_empty() {
        line.push(Span::styled("Type to filter…", theme::dim()));
    } else {
        line.push(Span::raw(query.to_string()));
    }
    frame.render_widget(Paragraph::new(Line::from(line)), input);
    frame.render_widget(
        Paragraph::new(Line::from(status)).alignment(Alignment::Right),
        counter,
    );
    let cursor_x = input.x + (theme::PROMPT.chars().count() + query.chars().count()) as u16;
    frame.set_cursor_position(Position::new(
        cursor_x.min(input.right().saturating_sub(1)),
        input.y,
    ));
}

fn draw_tabs(app: &App, frame: &mut Frame, area: Rect) {
    let current = app.current_view();
    let switchable = app.can_switch_view();
    let mut spans = vec![Span::raw(" ")];
    for (view, key) in &app.tabs {
        let name = capitalize(view.title());
        if *view == current {
            spans.push(Span::styled(
                format!(" {name} "),
                Style::default()
                    .fg(ratatui::style::Color::Black)
                    .bg(ACCENT)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            let style = if switchable {
                Style::default()
            } else {
                theme::dim()
            };
            spans.push(Span::styled(format!(" {name}"), style));
            spans.push(Span::styled(
                format!(" {}", theme::short_key(key)),
                theme::muted(),
            ));
        }
        spans.push(Span::raw("  "));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_body(app: &mut App, frame: &mut Frame, body: Rect) {
    let ratio = app.config.native.preview_ratio.min(90);
    let level = app.nav.top();
    let preview_req = level.preview_req();
    let wants_preview = level.has_preview() && ratio > 0 && body.width >= MIN_PREVIEW_WIDTH;
    let (list_area, preview_area) = if wants_preview {
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

    let level = app.nav.top_mut();
    let mut block = theme::panel(&list_title(level), true);
    if level.supports_sort() {
        block = block.title(theme::right_title(vec![
            Span::styled("sort ", theme::dim()),
            Span::styled(level.sort.label().to_string(), Style::default().fg(ACCENT2)),
            Span::raw(" "),
        ]));
    }
    if let Some(err) = &level.error {
        block = block.title_bottom(Line::from(Span::styled(
            format!(" ✖ {err} "),
            Style::default().fg(ratatui::style::Color::Red),
        )));
    }
    let inner = block.inner(list_area);
    frame.render_widget(block, list_area);

    if level.list.total() == 0 && !level.loading {
        draw_empty(frame, inner, &level.empty_hint());
    } else if level.list.is_empty() && !level.list.query().is_empty() {
        let hint = format!("No matches for “{}”", level.list.query());
        draw_empty(frame, inner, &hint);
    } else {
        level.list.render(frame, inner);
    }
}

fn list_title(level: &Level) -> String {
    if level.is_sessions() {
        "Sessions".into()
    } else {
        capitalize(&level.title)
    }
}

fn draw_empty(frame: &mut Frame, area: Rect, text: &str) {
    let [_, mid, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(area);
    frame.render_widget(
        Paragraph::new(text.to_string())
            .style(theme::dim())
            .alignment(Alignment::Center),
        mid,
    );
}

fn draw_footer(app: &App, frame: &mut Frame, area: Rect) {
    if let Some((text, severity)) = &app.notice {
        let (glyph, color) = theme::notice_style(*severity);
        let mut line = display_line(text, &[]);
        line.spans.insert(
            0,
            Span::styled(format!(" {glyph} "), Style::default().fg(color)),
        );
        frame.render_widget(Paragraph::new(line), area);
        return;
    }
    let kb = &app.config.keybinds;
    let level = app.nav.top();
    let mut spans = vec![Span::raw(" ")];
    let open = if level.is_sessions() { "enter" } else { "open" };
    spans.extend(theme::chip("enter", open));
    match RowCtx::of(level) {
        Some(RowCtx::Sessions) => {
            spans.extend(theme::chip(&kb.new_session, "new"));
            spans.extend(theme::chip(&kb.rename_session, "rename"));
            spans.extend(theme::chip(&kb.delete_session, "delete"));
            spans.extend(theme::chip(&kb.edit_labels, "labels"));
        }
        Some(RowCtx::RepoView) => {
            spans.extend(theme::chip(&kb.edit_labels, "labels"));
            spans.extend(theme::chip(&kb.remove_repo, "remove"));
        }
        Some(RowCtx::Drill) => spans.extend(theme::chip(&kb.clone_repo, "clone")),
        None => {}
    }
    if level.supports_sort() {
        spans.extend(theme::chip(&kb.sort_toggle, "sort"));
    }
    let esc = if app.nav.is_root() { "quit" } else { "back" };
    spans.extend(theme::chip("esc", esc));
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}
