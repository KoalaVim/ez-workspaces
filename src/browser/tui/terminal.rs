//! Terminal lifecycle: raw mode + alternate screen, restored on drop and on panic.
//!
//! The TUI draws on a private duplicate of the terminal output while file
//! descriptors 1 and 2 point at the stdio sink (see [`stdio`](super::stdio)),
//! so stray output of background threads cannot corrupt the screen.

use std::cell::RefCell;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::rc::Rc;
use std::sync::{Mutex, PoisonError};

use crossterm::cursor::Show;
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use super::stdio::{emergency_release, tty_of, Redirect};
use crate::error::Result;

/// The guard is shared between the event loop and the modal selector.
pub type SharedGuard = Rc<RefCell<TerminalGuard>>;

pub type Term = Terminal<CrosstermBackend<Box<dyn Write>>>;

/// The real terminal output while stdout is redirected (also read by the panic hook).
static TTY: Mutex<Option<File>> = Mutex::new(None);

fn set_tty(file: Option<File>) {
    *TTY.lock().unwrap_or_else(PoisonError::into_inner) = file;
}

/// Where the TUI writes: the terminal duplicate, or stdout when not redirected.
fn writer() -> Box<dyn Write> {
    let tty = TTY
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .and_then(|f| f.try_clone().ok());
    match tty {
        Some(file) => Box::new(BufWriter::new(file)),
        None => Box::new(io::stdout()),
    }
}

/// Owns the terminal. Dropping it restores the user's terminal state.
pub struct TerminalGuard {
    term: Term,
    active: bool,
    /// Declared last: dropped after the screen was left.
    stdio: Option<Redirect>,
}

fn enter_screen() -> io::Result<()> {
    enable_raw_mode()?;
    execute!(writer(), EnterAlternateScreen)
}

fn leave_screen() -> io::Result<()> {
    let raw = disable_raw_mode();
    execute!(writer(), LeaveAlternateScreen, Show)?;
    raw
}

impl TerminalGuard {
    /// Enter raw mode and the alternate screen.
    pub fn enter() -> Result<Self> {
        let stdio = Redirect::begin();
        set_tty(tty_of(stdio.as_ref()));
        if let Err(e) = enter_screen() {
            set_tty(None);
            return Err(e.into());
        }
        let term = match Terminal::new(CrosstermBackend::new(writer())) {
            Ok(t) => t,
            Err(e) => {
                let _ = leave_screen();
                set_tty(None);
                return Err(e.into());
            }
        };
        log::debug!("terminal: entered alternate screen");
        Ok(Self {
            term,
            active: true,
            stdio,
        })
    }

    pub fn terminal(&mut self) -> &mut Term {
        &mut self.term
    }

    /// Give the terminal back (for running an external program): leaves the
    /// alternate screen and points stdout/stderr at the real terminal.
    pub fn suspend(&mut self) -> Result<()> {
        if self.active {
            leave_screen()?;
            if let Some(stdio) = &self.stdio {
                stdio.release();
            }
            self.active = false;
            log::debug!("terminal: suspended");
        }
        Ok(())
    }

    /// Take the terminal again after [`suspend`](Self::suspend) and force a full redraw.
    pub fn resume(&mut self) -> Result<()> {
        if !self.active {
            if let Some(stdio) = &self.stdio {
                stdio.capture();
            }
            enter_screen()?;
            self.active = true;
            log::debug!("terminal: resumed");
        }
        execute!(writer(), Clear(ClearType::All))?;
        // A new terminal has empty buffers, so the next draw repaints everything.
        // (`Terminal::clear` asks the terminal for the cursor position through
        // stdout, which is redirected to the stdio sink.)
        self.term = Terminal::new(CrosstermBackend::new(writer()))?;
        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.active {
            if let Err(e) = leave_screen() {
                log::debug!("terminal: restore failed: {e}");
            }
        }
        // `stdio` restores the descriptors (and replays stray output) right
        // after this; the duplicate is not needed any more.
        set_tty(None);
    }
}

/// Restore the terminal before the default panic message is printed, so the
/// message is readable and the shell is left usable. Only a panic on the main
/// thread ends the session; a worker panic is contained by the pool.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    let main = std::thread::current().id();
    std::panic::set_hook(Box::new(move |info| {
        if std::thread::current().id() == main {
            let _ = leave_screen();
            emergency_release();
        }
        previous(info);
    }));
}
