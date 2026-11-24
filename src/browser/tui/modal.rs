//! `NativeSelector`: implements every `InteractiveSelector` method as a modal
//! drawn over the browser on the same terminal (design D1). Each call runs a
//! nested blocking loop; the outer event loop redraws once it returns.
//! Key semantics live in `modal_state.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use ansi_to_tui::IntoText;
use console::strip_ansi_codes;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap};
use ratatui::Frame;

use super::capture::{Capture, Unmuted};
use super::keys::Key;
use super::modal_state::{Done, Kind, Modal, Step};
use super::terminal::SharedGuard;
use super::theme;
use crate::browser::selector::{ActionResult, InteractiveSelector, SelectItem, StageOutcome};
use crate::error::Result;

/// What the selector and the app share: the terminal, the stdout capture
/// (paused while a modal is open) and the frozen frame modals are drawn over.
pub struct Core {
    pub guard: SharedGuard,
    pub capture: Capture,
    backdrop: RefCell<Option<Buffer>>,
}

impl Core {
    pub fn new(guard: SharedGuard) -> Self {
        Self {
            guard,
            capture: Capture::default(),
            backdrop: RefCell::new(None),
        }
    }

    pub fn set_backdrop(&self, buffer: Buffer) {
        *self.backdrop.borrow_mut() = Some(buffer);
    }
}

pub struct NativeSelector {
    core: Rc<Core>,
}

impl NativeSelector {
    pub fn new(core: Rc<Core>) -> Self {
        Self { core }
    }

    /// Run `modal` to completion on the shared terminal.
    fn run(&self, mut modal: Modal) -> Result<Done> {
        let _real_stdio = Unmuted::new(&self.core.capture);
        loop {
            {
                let backdrop = self.core.backdrop.borrow();
                self.core
                    .guard
                    .borrow_mut()
                    .terminal()
                    .draw(|f| draw(f, &mut modal, backdrop.as_ref()))?;
            }
            match event::read()? {
                Event::Key(k) if k.kind != KeyEventKind::Release => {
                    if let Step::Done(done) = modal.on_key(Key::from_event(&k))? {
                        log::debug!("modal '{}': {done:?}", strip_ansi_codes(&modal.title));
                        return Ok(done);
                    }
                }
                _ => {}
            }
        }
    }
}

fn hint(allow_back: bool, text_input: bool) -> String {
    let enter = if text_input {
        "Enter: confirm"
    } else {
        "Enter: pick or use typed value"
    };
    if allow_back {
        format!("{enter} · Ctrl-P: back · Esc: cancel")
    } else {
        format!("{enter} · Esc: cancel")
    }
}

fn stage_header(context: Option<&str>, hint: String) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(c) = context.filter(|c| !c.is_empty()) {
        lines.push(format!("Building: {c}"));
    }
    lines.push(hint);
    lines
}

impl InteractiveSelector for NativeSelector {
    fn select_one(
        &self,
        items: &[SelectItem],
        prompt: &str,
        _preview_cmd: Option<&str>,
    ) -> Result<Option<usize>> {
        if items.is_empty() {
            return Ok(None);
        }
        Ok(match self.run(Modal::pick(prompt, items, false))? {
            Done::Index(i) => Some(i),
            _ => None,
        })
    }

    fn select_many(&self, items: &[SelectItem], prompt: &str) -> Result<Vec<usize>> {
        if items.is_empty() {
            return Ok(Vec::new());
        }
        let modal = Modal::pick(prompt, items, true)
            .with_header(vec!["Tab: toggle · Enter: accept · Esc: cancel".into()]);
        Ok(match self.run(modal)? {
            Done::Marked(v) => v,
            _ => Vec::new(),
        })
    }

    fn input(&self, prompt: &str, default: Option<&str>) -> Result<String> {
        let fallback = default.unwrap_or_default().to_string();
        let modal = Modal::text(prompt, default).with_header(vec![hint(false, true)]);
        Ok(match self.run(modal)? {
            Done::Text(t) if !t.is_empty() => t,
            _ => fallback,
        })
    }

    fn confirm(&self, prompt: &str, default: bool) -> Result<bool> {
        Ok(matches!(
            self.run(Modal::confirm(prompt, default))?,
            Done::Answer(true)
        ))
    }

    fn select_with_back(
        &self,
        prompt: &str,
        items: &[SelectItem],
        allow_back: bool,
        context: Option<&str>,
    ) -> Result<StageOutcome> {
        if items.is_empty() {
            return Ok(StageOutcome::Cancel);
        }
        let modal = Modal::pick(prompt, items, false)
            .with_back(allow_back)
            .with_header(stage_header(context, hint(allow_back, false)));
        Ok(match self.run(modal)? {
            Done::Index(i) => StageOutcome::Picked(items[i].value.clone()),
            Done::Query(q) if !q.trim().is_empty() => StageOutcome::Picked(q.trim().to_string()),
            Done::Back => StageOutcome::Back,
            _ => StageOutcome::Cancel,
        })
    }

    fn input_with_back(
        &self,
        prompt: &str,
        default: Option<&str>,
        allow_back: bool,
        context: Option<&str>,
    ) -> Result<StageOutcome> {
        let modal = Modal::text(prompt, default)
            .with_back(allow_back)
            .with_header(stage_header(context, hint(allow_back, true)));
        Ok(match self.run(modal)? {
            Done::Text(t) => StageOutcome::Picked(t),
            Done::Back => StageOutcome::Back,
            _ => StageOutcome::Cancel,
        })
    }

    fn select_with_actions(
        &self,
        items: &[SelectItem],
        prompt: &str,
        _preview_cmd: Option<&str>,
        expect_keys: &[&str],
        header: Option<&str>,
    ) -> Result<ActionResult> {
        if items.is_empty() {
            return Ok(ActionResult::Cancel);
        }
        let modal = Modal::pick(prompt, items, false)
            .with_actions(expect_keys)
            .with_header(header.map(|h| vec![h.to_string()]).unwrap_or_default());
        Ok(match self.run(modal)? {
            Done::Index(i) => ActionResult::Select(i),
            Done::Action(key, i) => ActionResult::Action(key, i),
            _ => ActionResult::Cancel,
        })
    }
}

fn lines_of(text: &str) -> Vec<Line<'static>> {
    match text.into_text() {
        Ok(t) => t.lines,
        Err(_) => strip_ansi_codes(text)
            .lines()
            .map(|l| Line::from(l.to_string()))
            .collect(),
    }
}

/// Rows `lines` take when wrapped at `width`.
fn wrapped_height(lines: &[Line<'_>], width: u16) -> u16 {
    let w = width.max(1) as usize;
    lines
        .iter()
        .map(|l| l.width().div_ceil(w).max(1) as u16)
        .sum()
}

fn popup_area(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}

/// Draw the frozen app frame, then the modal centered over it.
pub fn draw(frame: &mut Frame, modal: &mut Modal, backdrop: Option<&Buffer>) {
    let area = frame.area();
    if let Some(b) = backdrop.filter(|b| b.area == area) {
        *frame.buffer_mut() = b.clone();
        // Fade the frozen frame so the modal stands out.
        for cell in frame.buffer_mut().content.iter_mut() {
            cell.modifier.insert(ratatui::style::Modifier::DIM);
        }
    }
    let dim = theme::dim();
    let width = area.width.saturating_sub(4).clamp(24, 90);
    let inner_w = width.saturating_sub(4);
    let header: Vec<Line> = modal.header.iter().flat_map(|h| lines_of(h)).collect();
    let header_h = wrapped_height(&header, inner_w);

    let (title, body_h) = match modal.kind {
        Kind::Confirm => {
            let msg = lines_of(&modal.title);
            ("Confirm".to_string(), wrapped_height(&msg, inner_w) + 1)
        }
        Kind::Text => (strip_ansi_codes(&modal.title).to_string(), header_h + 1),
        Kind::Pick { .. } => {
            let rows = modal.list.len().clamp(1, 10) as u16;
            (
                strip_ansi_codes(&modal.title).to_string(),
                header_h + 1 + rows,
            )
        }
    };
    let popup = popup_area(area, width, body_h + 3);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(title, theme::accent_bold()),
            Span::raw(" "),
        ]));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if modal.kind == Kind::Confirm {
        let [msg_area, hint_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(inner);
        let text = Text::from(lines_of(&modal.title));
        frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }), msg_area);
        let hint = if modal.confirm_default {
            "[Y/n]  y: yes · n: no · Enter: yes · Esc: no"
        } else {
            "[y/N]  y: yes · n: no · Enter: no · Esc: no"
        };
        frame.render_widget(Paragraph::new(hint).style(dim), hint_area);
        return;
    }

    let [header_area, input_area, rule_area, list_area] = Layout::vertical([
        Constraint::Length(header_h.min(inner.height)),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(inner);
    frame.render_widget(
        Paragraph::new("─".repeat(rule_area.width as usize)).style(theme::muted()),
        rule_area,
    );
    frame.render_widget(
        Paragraph::new(Text::from(header))
            .style(dim)
            .wrap(Wrap { trim: false }),
        header_area,
    );
    let input = modal.input().to_string();
    let cursor_x = input_area.x + (theme::PROMPT.chars().count() + input.chars().count()) as u16;
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(theme::PROMPT, theme::accent_bold()),
            Span::raw(input),
        ])),
        input_area,
    );
    frame.set_cursor_position(Position::new(
        cursor_x.min(input_area.right().saturating_sub(1)),
        input_area.y,
    ));
    if matches!(modal.kind, Kind::Pick { .. }) {
        modal.list.render(frame, list_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn screen(term: &Terminal<TestBackend>) -> String {
        let buf = term.backend().buffer();
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn modal_is_drawn_over_the_backdrop() {
        let mut term = Terminal::new(TestBackend::new(60, 16)).unwrap();
        let mut backdrop = Buffer::empty(Rect::new(0, 0, 60, 16));
        backdrop.set_string(0, 0, "SESSION LIST BEHIND", Style::default());
        let items = vec![SelectItem {
            display: "alpha".into(),
            value: "a".into(),
        }];
        let mut modal = Modal::pick("naming mode", &items, false)
            .with_header(vec!["Building: feat-".into(), "Enter: pick".into()]);
        term.draw(|f| draw(f, &mut modal, Some(&backdrop))).unwrap();
        let s = screen(&term);
        assert!(s.contains("SESSION LIST BEHIND"), "{s}");
        assert!(s.contains("naming mode"), "{s}");
        assert!(s.contains("Building: feat-"), "{s}");
        assert!(s.contains("alpha"), "{s}");
    }

    #[test]
    fn confirm_modal_shows_question_and_choices() {
        let mut term = Terminal::new(TestBackend::new(60, 16)).unwrap();
        let mut modal = Modal::confirm("Delete session 'x'?", false);
        term.draw(|f| draw(f, &mut modal, None)).unwrap();
        let s = screen(&term);
        assert!(s.contains("Delete session 'x'?"), "{s}");
        assert!(s.contains("[y/N]"), "{s}");
    }
}
