//! Captures everything written to stdout/stderr while an action handler runs.
//!
//! Domain handlers and the code they call (`session::*`, `repo::*`, plugin
//! runner, git helpers) print progress with `println!`/`eprintln!`. The
//! output is collected so it can become a notice instead of drawing over the
//! TUI: file descriptors 1 and 2 are pointed at an unlinked temp file for the
//! duration of the handler, then put back to what they were (the session
//! sink of [`stdio`](super::stdio), not the terminal).
//!
//! Descriptor redirection is unix only; elsewhere [`Capture`] is a no-op with
//! the same API (handler output is not collected).

#[cfg(unix)]
pub use unix_impl::Capture;

#[cfg(not(unix))]
pub use fallback::Capture;

#[cfg(unix)]
mod unix_impl {
    use std::cell::RefCell;
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};
    use std::os::fd::{AsRawFd, OwnedFd};

    use crate::browser::tui::stdio::fd::{dup, flush_std, redirect};
    use crate::error::Result;

    struct Active {
        saved_out: OwnedFd,
        saved_err: OwnedFd,
        file: File,
        muted: bool,
    }

    #[derive(Default)]
    pub struct Capture {
        state: RefCell<Option<Active>>,
    }

    impl Capture {
        /// Start capturing. A nested `begin` is ignored.
        pub fn begin(&self) -> Result<()> {
            let mut state = self.state.borrow_mut();
            if state.is_some() {
                return Ok(());
            }
            let path = std::env::temp_dir().join(format!("ez-capture-{}", std::process::id()));
            let file = std::fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .read(true)
                .write(true)
                .open(&path)?;
            // Unlink right away: the data lives as long as the descriptors do.
            let _ = std::fs::remove_file(&path);
            flush_std();
            let saved_out = dup(1)?;
            let saved_err = dup(2)?;
            redirect(file.as_raw_fd(), 1);
            redirect(file.as_raw_fd(), 2);
            log::debug!("capture: begin");
            *state = Some(Active {
                saved_out,
                saved_err,
                file,
                muted: true,
            });
            Ok(())
        }

        /// Give back the descriptors captured over (modals draw while paused).
        pub fn pause(&self) {
            if let Some(a) = self.state.borrow_mut().as_mut() {
                if a.muted {
                    flush_std();
                    redirect(a.saved_out.as_raw_fd(), 1);
                    redirect(a.saved_err.as_raw_fd(), 2);
                    a.muted = false;
                }
            }
        }

        /// Capture again after [`pause`](Self::pause).
        pub fn resume(&self) {
            if let Some(a) = self.state.borrow_mut().as_mut() {
                if !a.muted {
                    flush_std();
                    redirect(a.file.as_raw_fd(), 1);
                    redirect(a.file.as_raw_fd(), 2);
                    a.muted = true;
                }
            }
        }

        /// Stop capturing, restore the descriptors, return what was written.
        pub fn end(&self) -> String {
            self.pause();
            let Some(mut a) = self.state.borrow_mut().take() else {
                return String::new();
            };
            let mut text = String::new();
            if a.file.seek(SeekFrom::Start(0)).is_ok() {
                let mut bytes = Vec::new();
                let _ = a.file.read_to_end(&mut bytes);
                text = String::from_utf8_lossy(&bytes).into_owned();
            }
            log::debug!("capture: end ({} bytes)", text.len());
            text
        }
    }

    impl Drop for Capture {
        fn drop(&mut self) {
            // Never leave the process with redirected descriptors.
            self.end();
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn captured_output_is_returned_and_descriptors_restored() {
            let _serial = crate::browser::tui::stdio::FD_TESTS
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            // Only the descriptor plumbing is checked: the test harness captures
            // `println!` itself, so write through the raw descriptor.
            let cap = Capture::default();
            cap.begin().unwrap();
            let msg = b"hello from fd 1\n";
            unsafe {
                libc::write(1, msg.as_ptr().cast(), msg.len());
            }
            cap.pause();
            let hidden = b"visible\n";
            unsafe {
                libc::write(1, hidden.as_ptr().cast(), hidden.len());
            }
            cap.resume();
            let msg2 = b"second\n";
            unsafe {
                libc::write(2, msg2.as_ptr().cast(), msg2.len());
            }
            let text = cap.end();
            assert!(text.contains("hello from fd 1\n"), "{text:?}");
            assert!(text.contains("second\n"), "{text:?}");
            assert!(!text.contains("visible"), "{text:?}");
            // Ending twice is harmless.
            assert_eq!(cap.end(), "");
        }
    }
}

#[cfg(not(unix))]
mod fallback {
    use crate::error::Result;

    /// No descriptor redirection on this platform.
    #[derive(Default)]
    pub struct Capture;

    impl Capture {
        pub fn begin(&self) -> Result<()> {
            Ok(())
        }

        pub fn pause(&self) {}

        pub fn resume(&self) {}

        pub fn end(&self) -> String {
            String::new()
        }
    }
}

/// Runs with the capture paused, resuming afterwards.
pub struct Unmuted<'a>(&'a Capture);

impl<'a> Unmuted<'a> {
    pub fn new(capture: &'a Capture) -> Self {
        capture.pause();
        Self(capture)
    }
}

impl Drop for Unmuted<'_> {
    fn drop(&mut self) {
        self.0.resume();
    }
}
