//! Synchronous event loop (design D3): `crossterm::event::poll` with a short
//! timeout, results from worker threads over an `mpsc` channel, and redraws
//! only when something changed.

use std::sync::mpsc::Receiver;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use ratatui::Frame;

use super::terminal::SharedGuard;
use crate::error::Result;

const POLL: Duration = Duration::from_millis(16);
/// Redraw interval while [`Handler::animating`] is true.
const TICK: Duration = Duration::from_millis(80);

/// What the loop should do after an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    Continue,
    Quit,
    /// Leave and re-enter the alternate screen, then repaint everything.
    FullRedraw,
}

/// The application side of the loop.
pub trait Handler {
    /// Message type sent by worker threads.
    type Msg;

    fn on_key(&mut self, key: KeyEvent) -> Result<Flow>;
    fn on_msg(&mut self, msg: Self::Msg);
    fn draw(&mut self, frame: &mut Frame);

    /// True while something animates (the loading spinner): redraw on a tick.
    fn animating(&self) -> bool {
        false
    }

    /// False while the first frame is held back (waiting for fresh data).
    fn ready(&self) -> bool {
        true
    }
}

/// Run until the handler quits or errors (for example with `Interrupted`).
pub fn run<H: Handler>(
    guard: &SharedGuard,
    handler: &mut H,
    results: &Receiver<H::Msg>,
) -> Result<()> {
    let mut dirty = true;
    let mut last_draw = std::time::Instant::now();
    loop {
        if !dirty && handler.animating() && last_draw.elapsed() >= TICK {
            dirty = true;
        }
        if dirty && handler.ready() {
            guard.borrow_mut().terminal().draw(|f| handler.draw(f))?;
            dirty = false;
            last_draw = std::time::Instant::now();
        }
        if event::poll(POLL)? {
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    log::trace!("tui: key {key:?}");
                    match handler.on_key(key)? {
                        Flow::Continue => {}
                        Flow::Quit => return Ok(()),
                        Flow::FullRedraw => {
                            let mut g = guard.borrow_mut();
                            g.suspend()?;
                            g.resume()?;
                        }
                    }
                    dirty = true;
                }
                Event::Resize(..) => dirty = true,
                _ => {}
            }
        }
        while let Ok(msg) = results.try_recv() {
            handler.on_msg(msg);
            dirty = true;
        }
    }
}
