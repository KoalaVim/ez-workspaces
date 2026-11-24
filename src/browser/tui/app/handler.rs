//! Event-loop callbacks: key handling, worker results, drawing.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;

use super::keymap::RowCtx;
use super::{draw, find_level, App, Msg, FIRST_PAINT_WAIT};
use crate::browser::domain::Level as Severity;
use crate::browser::tui::event::{Flow, Handler};
use crate::browser::tui::keys::Key;
use crate::browser::tui::nav::Back;
use crate::error::{EzError, Result};

impl Handler for App {
    type Msg = Msg;

    fn on_key(&mut self, ev: KeyEvent) -> Result<Flow> {
        let key = Key::from_event(&ev);
        self.notice = None;
        if key == Key::new(KeyCode::Char('c'), KeyModifiers::CONTROL) {
            return Err(EzError::Interrupted);
        }
        if key.code == KeyCode::Esc && key.mods.is_empty() {
            return Ok(match self.nav.back() {
                Back::Popped => Flow::Continue,
                Back::Exit => Flow::Quit,
            });
        }
        if let Some(action) = self.keymap.get(&key).copied() {
            return Ok(self.act(action));
        }
        let enter = key == Key::new(KeyCode::Enter, KeyModifiers::NONE);
        let row_spec = RowCtx::of(self.nav.top())
            .and_then(|ctx| Some((ctx, self.row_keys.find(ctx, &key)?.to_string())));
        if (enter || row_spec.is_some()) && self.nav.top().stale {
            // Snapshot rows carry no payload to act on; fresh rows arrive in a moment.
            self.notice = Some((
                "Still loading fresh data, try again in a moment".into(),
                Severity::Warn,
            ));
            return Ok(Flow::Continue);
        }
        if enter {
            return self.enter();
        }
        if let Some((ctx, spec)) = row_spec {
            return self.row_action(ctx, &spec);
        }
        let list = &mut self.nav.top_mut().list;
        match (key.code, key.mods) {
            (KeyCode::Up, _) | (KeyCode::Char('p' | 'k'), KeyModifiers::CONTROL) => {
                list.move_cursor(-1)
            }
            (KeyCode::Down, _) | (KeyCode::Char('n' | 'j'), KeyModifiers::CONTROL) => {
                list.move_cursor(1)
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => list.set_query(""),
            (KeyCode::PageUp, _) => list.move_cursor(-10),
            (KeyCode::PageDown, _) => list.move_cursor(10),
            (KeyCode::Backspace, _) => list.pop_char(),
            (KeyCode::Char(c), m) if m.is_empty() => list.push_char(c),
            _ => log::debug!("tui: unmapped key {ev:?}"),
        }
        Ok(Flow::Continue)
    }

    fn on_msg(&mut self, msg: Msg) {
        match msg {
            Msg::Rows {
                level,
                gen,
                loaded,
                quiet,
                pending,
                partial,
            } => {
                let Some(l) =
                    find_level(&mut self.nav, &mut self.saved, level).filter(|l| l.gen == gen)
                else {
                    log::debug!("tui: dropped stale result for level {level}");
                    return;
                };
                log::debug!(
                    "tui: level {level} loaded, {} rows ({} branches pending, partial {partial})",
                    loaded.rows.len(),
                    pending.len()
                );
                l.list.set_rows(loaded.rows);
                l.loading = false;
                l.stale = false;
                if let Some(entry) = loaded.repo.as_ref().filter(|_| partial) {
                    l.title = entry.name.clone();
                    l.repo = Some(entry.clone());
                }
                if partial {
                    // Usable now; stays busy until the full result lands.
                    self.first_paint_at = None;
                    if !quiet {
                        self.data_gen += 1;
                    }
                    return;
                }
                l.busy = false;
                l.preview_cmd = loaded.preview_cmd;
                if let Some(entry) = loaded.repo {
                    l.title = entry.name.clone();
                    l.repo = Some(entry);
                }
                if std::mem::take(&mut l.reload_again) {
                    self.loader.request_quiet(l);
                }
                self.first_paint_at = None;
                if !quiet {
                    self.data_gen += 1;
                }
                if !pending.is_empty() {
                    self.branch_inflight += pending.len();
                    *self.branch_pending.entry(level).or_default() += pending.len();
                    self.loader.resolve_branches(level, pending);
                }
            }
            Msg::Failed { level, gen, error } => {
                let Some(l) =
                    find_level(&mut self.nav, &mut self.saved, level).filter(|l| l.gen == gen)
                else {
                    return;
                };
                log::debug!("tui: level {level} failed: {error}");
                l.error = Some(error);
                l.loading = false;
                l.busy = false;
                self.first_paint_at = None;
            }
            Msg::Branch { level } => self.branch_resolved(level),
            Msg::Preview {
                key,
                data_gen,
                index,
                text,
            } => self.preview.fill(key, data_gen, index, text),
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        draw::draw(self, frame);
    }

    fn animating(&self) -> bool {
        draw::animating(self)
    }

    fn ready(&self) -> bool {
        self.first_paint_at
            .is_none_or(|since| since.elapsed() >= FIRST_PAINT_WAIT)
    }
}
