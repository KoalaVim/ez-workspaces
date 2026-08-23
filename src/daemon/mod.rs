use std::fs;
use std::os::unix::io::AsRawFd;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use colored::Colorize;

use crate::cli::DaemonCommand;
use crate::error::Result;
use crate::paths;
use crate::session::model::Session;

/// Dispatch daemon subcommands.
pub fn dispatch(cmd: DaemonCommand) -> Result<()> {
    match cmd {
        DaemonCommand::Start => {
            if is_daemon_alive() {
                println!("{}", "Daemon is already running.".yellow());
                Ok(())
            } else {
                spawn_daemon()
            }
        }
        DaemonCommand::Stop => stop_daemon(),
        DaemonCommand::Status => daemon_status(),
        DaemonCommand::Run => daemon_run(),
    }
}

/// Write the current process PID to the daemon PID file.
pub fn write_pid_file(pid: u32) -> Result<()> {
    let path = paths::daemon_pid_file()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, pid.to_string())?;
    Ok(())
}

/// Read the PID from the daemon PID file, if present and parseable.
pub fn read_pid_file() -> Option<u32> {
    let path = paths::daemon_pid_file().ok()?;
    let contents = fs::read_to_string(path).ok()?;
    contents.trim().parse::<u32>().ok()
}

/// Check whether the daemon process recorded in the PID file is alive.
/// Cleans up a stale PID file if the process is no longer running.
pub fn is_daemon_alive() -> bool {
    let Some(pid) = read_pid_file() else {
        return false;
    };

    // Signal 0 performs no-op error checking: it tells us whether the
    // process exists (and is signalable) without actually sending a signal.
    let alive = unsafe { libc::kill(pid as libc::pid_t, 0) } == 0;

    if !alive {
        log::debug!("daemon: stale PID file for pid {pid}, cleaning up");
        if let Ok(path) = paths::daemon_pid_file() {
            let _ = fs::remove_file(path);
        }
    }

    alive
}

/// Spawn the daemon as a detached background process running `ez daemon run`.
pub fn spawn_daemon() -> Result<()> {
    let exe = std::env::current_exe()?;
    log::debug!("daemon: spawning {} daemon run", exe.display());

    let child = std::process::Command::new(exe)
        .args(["daemon", "run"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    println!(
        "{} (pid {})",
        "Daemon started.".green(),
        child.id().to_string().bold()
    );
    Ok(())
}

/// Ensure the background daemon is running, starting it silently if not.
///
/// This is fire-and-forget: it never prints anything and never returns an
/// error. A failure to auto-start the daemon should never affect the user's
/// command.
pub fn ensure_daemon_running() {
    if is_daemon_alive() {
        return;
    }

    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    log::debug!("daemon: auto-starting {} daemon run", exe.display());

    let _ = std::process::Command::new(exe)
        .args(["daemon", "run"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

/// Stop the running daemon by sending SIGTERM, then remove the PID file.
pub fn stop_daemon() -> Result<()> {
    let Some(pid) = read_pid_file() else {
        println!("{}", "Daemon is not running.".yellow());
        return Ok(());
    };

    log::debug!("daemon: sending SIGTERM to pid {pid}");
    unsafe {
        libc::kill(pid as libc::pid_t, libc::SIGTERM);
    }

    if let Ok(path) = paths::daemon_pid_file() {
        let _ = fs::remove_file(path);
    }

    println!("{} (pid {})", "Daemon stopped.".green(), pid);
    Ok(())
}

/// Print the current daemon status.
pub fn daemon_status() -> Result<()> {
    let log_path = paths::daemon_log_file()?;

    if is_daemon_alive() {
        let pid = read_pid_file().unwrap_or(0);
        println!("{} (pid {})", "Daemon is running.".green(), pid);
    } else {
        println!("{}", "Daemon is not running.".yellow());
    }
    println!("{} {}", "Log file:".dimmed(), log_path.display());

    Ok(())
}

/// How often the daemon refreshes PR statuses, in seconds.
const POLL_INTERVAL_SECS: u64 = 300;

/// Maximum number of sessions refreshed per cycle.
const MAX_CANDIDATES: usize = 20;

/// Set by the SIGTERM handler; polled by the main loop between sleeps.
static SHOULD_STOP: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_sigterm(_signum: libc::c_int) {
    SHOULD_STOP.store(true, Ordering::SeqCst);
}

/// The actual daemon loop entry point (run via `ez daemon run`, not called
/// directly by users). Writes the PID file, sets up logging to the daemon
/// log file, installs a SIGTERM handler, and then polls for PR status
/// updates every `POLL_INTERVAL_SECS` seconds until asked to stop.
fn daemon_run() -> Result<()> {
    let pid = std::process::id();
    write_pid_file(pid)?;
    setup_daemon_logging()?;

    unsafe {
        libc::signal(
            libc::SIGTERM,
            handle_sigterm as *const () as libc::sighandler_t,
        );
    }

    log::info!("daemon: started (pid {pid})");

    loop {
        let (refreshed, skipped) = refresh_all_sessions();
        log::info!("daemon: cycle complete: refreshed {refreshed}, skipped {skipped}");

        if wait_or_stop(POLL_INTERVAL_SECS) {
            break;
        }
    }

    if let Ok(path) = paths::daemon_pid_file() {
        let _ = fs::remove_file(path);
    }
    log::info!("daemon: stopped (pid {pid})");
    Ok(())
}

/// Configure logging for the daemon process: truncate/create the daemon log
/// file and route all log output there at Info level.
fn setup_daemon_logging() -> Result<()> {
    let log_path = paths::daemon_log_file()?;
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = fs::File::create(&log_path)?;
    // The daemon is a fresh process (main.rs skips its own logger init for
    // `daemon run`), so this should always succeed; try_init guards against
    // any future double-init rather than panicking the daemon.
    let _ = env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .target(env_logger::Target::Pipe(Box::new(file)))
        .try_init();
    Ok(())
}

/// Sleep for `secs` seconds in 1-second increments, checking the stop flag
/// between each. Returns `true` if a stop was requested during the sleep.
fn wait_or_stop(secs: u64) -> bool {
    for _ in 0..secs {
        if SHOULD_STOP.load(Ordering::SeqCst) {
            return true;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    SHOULD_STOP.load(Ordering::SeqCst)
}

/// A session with `ez_pr_number` set, eligible for a PR status refresh.
struct Candidate {
    repo_id: String,
    session_id: String,
    session_name: String,
    last_accessed: Option<String>,
}

/// Scan all registered repos for sessions with PR metadata, and refresh the
/// PR status of the most-recently-accessed ones (capped at `MAX_CANDIDATES`).
/// Returns (refreshed, skipped) counts.
fn refresh_all_sessions() -> (usize, usize) {
    let current_gh_user = match crate::session::get_current_gh_user() {
        Some(user) => user,
        None => {
            log::warn!("daemon: gh user not authenticated, skipping all sessions this cycle");
            return (0, 0);
        }
    };

    let index = match crate::repo::store::load_index() {
        Ok(idx) => idx,
        Err(e) => {
            log::warn!("daemon: failed to load repo index: {e}");
            return (0, 0);
        }
    };

    let mut candidates: Vec<Candidate> = Vec::new();
    for repo in &index.repos {
        if !repo.is_git {
            continue;
        }
        let tree = match crate::session::store::load_sessions(&repo.id) {
            Ok(t) => t,
            Err(e) => {
                log::warn!(
                    "daemon: failed to load sessions for repo '{}': {e}",
                    repo.id
                );
                continue;
            }
        };
        for session in &tree.sessions {
            if session.env.contains_key("ez_pr_number") {
                candidates.push(Candidate {
                    repo_id: repo.id.clone(),
                    session_id: session.id.clone(),
                    session_name: session.name.clone(),
                    last_accessed: session.last_accessed.clone(),
                });
            }
        }
    }

    // Most-recently-accessed first; sessions with no last_accessed sort last.
    candidates.sort_by(|a, b| match (&a.last_accessed, &b.last_accessed) {
        (Some(x), Some(y)) => y.cmp(x),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    candidates.truncate(MAX_CANDIDATES);

    let mut refreshed = 0usize;
    let mut skipped = 0usize;
    for candidate in candidates {
        match process_candidate(&candidate, &current_gh_user) {
            ProcessOutcome::Refreshed => refreshed += 1,
            ProcessOutcome::Skipped => skipped += 1,
        }
    }

    (refreshed, skipped)
}

enum ProcessOutcome {
    Refreshed,
    Skipped,
}

/// True if the session's PR status was updated within the freshness window
/// `refresh_pr_status` uses internally. Used only to pick an accurate log
/// message; the actual staleness gating happens inside `refresh_pr_status`.
fn pr_status_is_fresh(session: &Session) -> bool {
    match session.env.get("ez_pr_status_updated") {
        Some(updated) => match chrono::DateTime::parse_from_rfc3339(updated) {
            Ok(dt) => chrono::Utc::now().signed_duration_since(dt).num_seconds() < 300,
            Err(_) => false,
        },
        None => false,
    }
}

/// Refresh (or skip) a single candidate session's PR status, holding an
/// exclusive `flock` on the repo's sessions file for the whole
/// read-modify-write cycle so an interactive `ez` process can't race it.
fn process_candidate(candidate: &Candidate, current_gh_user: &str) -> ProcessOutcome {
    let repo_id = &candidate.repo_id;
    let session_id = &candidate.session_id;
    let session_name = &candidate.session_name;

    let result = with_session_lock(repo_id, || {
        let mut tree = crate::session::store::load_sessions(repo_id)?;

        let session = match tree.sessions.iter().find(|s| s.id == *session_id) {
            Some(s) => s,
            None => return Ok(ProcessOutcome::Skipped),
        };

        let session_gh_user = session.env.get("ez_pr_gh_user").cloned();
        if let Some(gh_user) = &session_gh_user {
            if gh_user != current_gh_user {
                log::debug!(
                    "daemon: skipped session '{session_name}' in repo '{repo_id}': gh user mismatch"
                );
                return Ok(ProcessOutcome::Skipped);
            }
        }

        let was_fresh = pr_status_is_fresh(session);

        crate::session::refresh_pr_status(&mut tree, session_id);

        // Legacy sessions with no ez_pr_gh_user recorded: backfill it now
        // that we know the status was (re)fetched as the current gh user.
        if session_gh_user.is_none() {
            if let Some(s) = tree.sessions.iter_mut().find(|s| s.id == *session_id) {
                s.env
                    .insert("ez_pr_gh_user".into(), current_gh_user.to_string());
            }
        }

        crate::session::store::save_sessions(repo_id, &tree)?;

        if was_fresh {
            log::debug!("daemon: skipped session '{session_name}': status is fresh");
            Ok(ProcessOutcome::Skipped)
        } else {
            let pr_number = tree
                .sessions
                .iter()
                .find(|s| s.id == *session_id)
                .and_then(|s| s.env.get("ez_pr_number").cloned())
                .unwrap_or_default();
            log::info!(
                "daemon: refreshed PR #{pr_number} for session '{session_name}' in repo '{repo_id}'"
            );
            Ok(ProcessOutcome::Refreshed)
        }
    });

    match result {
        Ok(outcome) => outcome,
        Err(e) => {
            log::warn!(
                "daemon: error processing session '{session_name}' in repo '{repo_id}': {e}"
            );
            ProcessOutcome::Skipped
        }
    }
}

/// Run `f` while holding an exclusive `flock` on the repo's sessions file,
/// blocking until the lock is available. Guards against interactive `ez`
/// processes writing the same file concurrently.
fn with_session_lock<F, T>(repo_id: &str, f: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    let path = paths::sessions_file(repo_id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    // Open (creating if missing) without truncating, purely to hold a lock.
    #[allow(clippy::suspicious_open_options)]
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)?;

    unsafe {
        libc::flock(file.as_raw_fd(), libc::LOCK_EX);
    }
    let result = f();
    unsafe {
        libc::flock(file.as_raw_fd(), libc::LOCK_UN);
    }
    result
}
