//! Keeps stray stdout/stderr output off the alternate screen.
//!
//! Background threads, plugin warnings (`eprintln!`) and `RUST_LOG` output
//! must never reach the terminal while the TUI owns it. For the whole native
//! session file descriptors 1 and 2 point at an unlinked temp file (the
//! "sink") and the TUI draws through a private duplicate of the real
//! terminal ([`Redirect::tty`]). [`Redirect::release`] puts the real
//! terminal back for external programs (suspend, `Effect::Exec`, notes,
//! clone); dropping the redirect restores it for good and replays whatever
//! landed in the sink to stderr, so nothing is lost.
//!
//! Unix only: on other platforms there is no redirection and
//! [`Redirect::begin`] returns `None`.

use std::fs::File;

#[cfg(unix)]
pub(super) mod fd {
    use std::io::Write;
    use std::os::fd::{FromRawFd, OwnedFd};

    pub fn dup(fd: i32) -> std::io::Result<OwnedFd> {
        // SAFETY: `dup` returns a fresh descriptor that we exclusively own.
        let new = unsafe { libc::dup(fd) };
        if new < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(unsafe { OwnedFd::from_raw_fd(new) })
    }

    pub fn redirect(from: i32, onto: i32) {
        // SAFETY: both descriptors are valid for the duration of the call.
        unsafe {
            libc::dup2(from, onto);
        }
    }

    pub fn flush_std() {
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
    }
}

#[cfg(unix)]
mod imp {
    use super::fd::{dup, flush_std, redirect};
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};
    use std::os::fd::{AsRawFd, OwnedFd};
    use std::sync::{Mutex, PoisonError};

    /// Descriptor numbers of the real terminal, for the panic hook.
    static REAL: Mutex<Option<(i32, i32)>> = Mutex::new(None);

    pub fn emergency_release() {
        if let Some((out, err)) = *REAL.lock().unwrap_or_else(PoisonError::into_inner) {
            redirect(out, 1);
            redirect(err, 2);
        }
    }

    pub struct Redirect {
        real_out: OwnedFd,
        real_err: OwnedFd,
        sink: File,
    }

    impl Redirect {
        pub fn begin() -> Option<Self> {
            let path = std::env::temp_dir().join(format!("ez-stdio-{}", std::process::id()));
            let sink = std::fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .read(true)
                .write(true)
                .open(&path)
                .ok()?;
            // Unlink right away: the data lives as long as the descriptors do.
            let _ = std::fs::remove_file(&path);
            flush_std();
            let (real_out, real_err) = (dup(1).ok()?, dup(2).ok()?);
            let this = Self {
                real_out,
                real_err,
                sink,
            };
            *REAL.lock().unwrap_or_else(PoisonError::into_inner) =
                Some((this.real_out.as_raw_fd(), this.real_err.as_raw_fd()));
            this.capture();
            log::debug!("stdio: fds 1 and 2 redirected to the sink");
            Some(this)
        }

        /// The real terminal output, for the TUI to draw on.
        pub fn tty(&self) -> Option<File> {
            self.real_out.try_clone().ok().map(File::from)
        }

        /// External programs get the real terminal.
        pub fn release(&self) {
            flush_std();
            redirect(self.real_out.as_raw_fd(), 1);
            redirect(self.real_err.as_raw_fd(), 2);
        }

        /// Send stdout/stderr to the sink again.
        pub fn capture(&self) {
            flush_std();
            redirect(self.sink.as_raw_fd(), 1);
            redirect(self.sink.as_raw_fd(), 2);
        }

        /// Stray output collected so far.
        pub fn collected(&self) -> String {
            let mut file = &self.sink;
            let mut bytes = Vec::new();
            if file.seek(SeekFrom::Start(0)).is_ok() {
                let _ = file.read_to_end(&mut bytes);
            }
            String::from_utf8_lossy(&bytes).into_owned()
        }
    }

    impl Drop for Redirect {
        fn drop(&mut self) {
            self.release();
            *REAL.lock().unwrap_or_else(PoisonError::into_inner) = None;
            let text = self.collected();
            if !text.trim().is_empty() {
                log::debug!("stdio: {} bytes of stray output replayed", text.len());
                eprint!("{text}");
            }
        }
    }
}

#[cfg(not(unix))]
mod imp {
    use std::fs::File;

    pub struct Redirect;

    pub fn emergency_release() {}

    impl Redirect {
        pub fn begin() -> Option<Self> {
            None
        }

        pub fn tty(&self) -> Option<File> {
            None
        }

        pub fn release(&self) {}

        pub fn capture(&self) {}
    }
}

pub use imp::{emergency_release, Redirect};

/// The terminal duplicate to draw on, if the redirect has one.
pub fn tty_of(redirect: Option<&Redirect>) -> Option<File> {
    redirect.and_then(Redirect::tty)
}

/// Tests that redirect the process-wide descriptors must not run concurrently.
#[cfg(all(test, unix))]
pub(super) static FD_TESTS: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::fd::AsRawFd;

    #[test]
    fn stray_output_goes_to_the_sink_until_released() {
        let _serial = FD_TESTS.lock().unwrap_or_else(|e| e.into_inner());
        // Raw writes: the test harness captures `println!` by itself.
        fn write_fd(fd: i32, text: &str) {
            unsafe {
                libc::write(fd, text.as_ptr().cast(), text.len());
            }
        }
        let redirect = Redirect::begin().expect("redirect");
        write_fd(1, "stray stdout\n");
        write_fd(2, "stray stderr\n");
        // The duplicate still points at the original stdout, not the sink.
        let tty = redirect.tty().expect("tty duplicate");
        assert_ne!(tty.as_raw_fd(), 1);
        redirect.release();
        redirect.capture();
        write_fd(1, "again\n");
        let text = redirect.collected();
        assert!(text.contains("stray stdout\n"), "{text:?}");
        assert!(text.contains("stray stderr\n"), "{text:?}");
        assert!(text.contains("again\n"), "{text:?}");
    }
}
