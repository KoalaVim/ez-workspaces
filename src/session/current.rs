use std::path::{Path, PathBuf};
use std::process::Command;

use colored::Colorize;

use super::model::Session;
use super::store;
use crate::browser::selector::confirm_prompt;
use crate::error::{EzError, Result};
use crate::repo::{self, model::RepoEntry};

pub(crate) struct CurrentSessionTarget {
    pub repo_entry: RepoEntry,
    pub session: Session,
    source: CurrentSessionSource,
}

enum CurrentSessionSource {
    Tmux(PathBuf),
    Zellij(PathBuf),
    Worktree(PathBuf),
}

impl CurrentSessionSource {
    fn label(&self) -> &'static str {
        match self {
            Self::Tmux(_) => "tmux @ez_session_path",
            Self::Zellij(_) => "zellij session name",
            Self::Worktree(_) => "current directory",
        }
    }

    fn path(&self) -> &Path {
        match self {
            Self::Tmux(path) | Self::Zellij(path) | Self::Worktree(path) => path,
        }
    }
}

/// Encode a repo/session pair into a multiplexer session name.
///
/// Every byte outside `[A-Za-z0-9_-]` becomes `_`, and the two parts are joined
/// with `__`. The encoding is deterministic so the zellij plugin (which has no
/// per-session metadata store, unlike tmux user options) and this module can
/// agree on a session's identity without any persisted state. The bash side in
/// `plugins/zellij/zellij-plugin` mirrors this with `LC_ALL=C tr`, which is why
/// this substitutes per byte rather than per char: a multi-byte character maps
/// to one `_` per byte on both sides.
pub(crate) fn encode_mux_name(repo_basename: &str, session_name: &str) -> String {
    format!(
        "{}__{}",
        encode_mux_part(repo_basename),
        encode_mux_part(session_name)
    )
}

fn encode_mux_part(part: &str) -> String {
    part.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' {
                b as char
            } else {
                '_'
            }
        })
        .collect()
}

/// Number of hex digits of the digest the zellij plugin appends to a shortened
/// session name. Must match `md5_hex4` in `plugins/zellij/zellij-plugin`.
const MUX_DIGEST_LEN: usize = 4;

/// Does `mux_name` name the zellij session belonging to this repo/session pair?
///
/// zellij binds one socket per session named after the session, so a name that
/// does not fit the 103-byte socket path cannot be used at all. The plugin
/// shortens those to `<encoded-session-prefix>_<digest>`, where the digest is the
/// first [`MUX_DIGEST_LEN`] hex digits of the md5 of the *full* encoded name (see
/// `fit_name` in `plugins/zellij/zellij-plugin`).
///
/// The byte budget that decided how much of the name survived is deliberately not
/// re-derived here: it depends on `$TMPDIR`, `$ZELLIJ_SOCKET_DIR` and zellij's
/// contract-version directory, none of which this process can know were the same
/// when the session was created. Accepting *any* prefix of the encoded session
/// name that carries the right digest matches every budget without tracking one.
fn mux_name_matches(repo_basename: &str, session_name: &str, mux_name: &str) -> bool {
    let full = encode_mux_name(repo_basename, session_name);
    if mux_name == full {
        return true;
    }

    let Some((visible, digest)) = mux_name.rsplit_once('_') else {
        return false;
    };
    if digest.len() != MUX_DIGEST_LEN {
        return false;
    }
    // The visible part is what is left of the session name; the repo is only
    // represented in the digest, which is why both are checked.
    if !encode_mux_part(session_name).starts_with(visible) {
        return false;
    }
    let expected = format!("{:x}", md5::compute(full.as_bytes()));
    digest == &expected[..MUX_DIGEST_LEN]
}

pub(crate) fn resolve_current_session(repo_arg: Option<&str>) -> Result<CurrentSessionTarget> {
    let repos = candidate_repos(repo_arg)?;

    // Try tmux @ez_session_name first (set by the tmux plugin) — most precise.
    if let (Some(tmux_repo_id), Some(tmux_session_name)) = (
        tmux_user_option("@ez_repo_id"),
        tmux_user_option("@ez_session_name"),
    ) {
        log::debug!(
            "resolving current session from tmux @ez_session_name: repo={} session={}",
            tmux_repo_id,
            tmux_session_name
        );
        if let Some((repo_entry, session)) =
            find_session_by_name(&repos, &tmux_repo_id, &tmux_session_name)?
        {
            let path = session
                .path
                .clone()
                .unwrap_or_else(|| repo_entry.path.clone());
            return Ok(CurrentSessionTarget {
                repo_entry,
                session,
                source: CurrentSessionSource::Tmux(path),
            });
        }
        log::debug!("tmux @ez_session_name did not match any registered session");
    }

    // Fall back to @ez_session_path matching.
    if let Some(path) = tmux_user_option("@ez_session_path").map(std::path::PathBuf::from) {
        log::debug!(
            "resolving current session from tmux @ez_session_path: {}",
            path.display()
        );
        if let Some((repo_entry, session)) = find_session_by_path(&repos, &path)? {
            return Ok(CurrentSessionTarget {
                repo_entry,
                session,
                source: CurrentSessionSource::Tmux(path),
            });
        }
        log::debug!(
            "tmux @ez_session_path did not match any registered session: {}",
            path.display()
        );
    }

    // Zellij has no per-session option store, so identity is derived from the
    // session name the plugin encoded when it created the zellij session.
    if let Some(zellij_name) = zellij_session_name() {
        log::debug!("resolving current session from zellij session name: {zellij_name}");
        if let Some((repo_entry, session)) = find_session_by_mux_name(&repos, &zellij_name)? {
            let path = session
                .path
                .clone()
                .unwrap_or_else(|| repo_entry.path.clone());
            return Ok(CurrentSessionTarget {
                repo_entry,
                session,
                source: CurrentSessionSource::Zellij(path),
            });
        }
        log::debug!("zellij session name did not match any registered session: {zellij_name}");
    }

    let cwd = std::env::current_dir()?;
    log::debug!(
        "resolving current session from current directory: {}",
        cwd.display()
    );
    if let Some((repo_entry, session)) = find_session_by_path(&repos, &cwd)? {
        return Ok(CurrentSessionTarget {
            repo_entry,
            session,
            source: CurrentSessionSource::Worktree(cwd),
        });
    }

    Err(EzError::SessionNotFound(
        "current session (tmux user options, zellij session name, and current directory did not match any registered session)".into(),
    ))
}

pub(crate) fn confirm_delete_current_session(target: &CurrentSessionTarget) -> Result<()> {
    let session_path = target
        .session
        .path
        .as_deref()
        .unwrap_or(target.repo_entry.path.as_path());
    let prompt = format!(
        "{} {}
{} {}
{} {}
{} {}
{}",
        "Delete current session?".yellow().bold(),
        target.session.name.bold(),
        "Repo:".cyan(),
        target.repo_entry.name.bold(),
        "Detected by:".cyan(),
        target.source.label(),
        "Matched path:".cyan(),
        target.source.path().display(),
        format!("Session path: {}", session_path.display()).dimmed()
    );

    if confirm_prompt(&prompt, false)? {
        Ok(())
    } else {
        Err(EzError::Cancelled)
    }
}

fn candidate_repos(repo_arg: Option<&str>) -> Result<Vec<RepoEntry>> {
    match repo_arg {
        Some(arg) => Ok(vec![repo::resolve_repo(Some(arg))?]),
        None => Ok(repo::store::load_index()?.repos),
    }
}

fn tmux_user_option(option: &str) -> Option<String> {
    std::env::var_os("TMUX")?;

    let output = match Command::new("tmux")
        .args(["show-options", "-v", "-q", option])
        .output()
    {
        Ok(output) => output,
        Err(err) => {
            log::debug!("failed to run tmux while reading {option}: {err}");
            return None;
        }
    };

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn zellij_session_name() -> Option<String> {
    let name = std::env::var("ZELLIJ_SESSION_NAME").ok()?;
    let name = name.trim().to_string();
    if name.is_empty() { None } else { Some(name) }
}

/// Find the session whose multiplexer name is `mux_name`, in either the full or
/// the shortened encoding (see `mux_name_matches`).
///
/// Encoding is lossy (see `encode_mux_name`), so two repo/session pairs can in
/// principle collide; the first match in registry order wins.
fn find_session_by_mux_name(
    repos: &[RepoEntry],
    mux_name: &str,
) -> Result<Option<(RepoEntry, Session)>> {
    for repo_entry in repos {
        let repo_basename = repo_basename(&repo_entry.path);
        let tree = store::load_sessions(&repo_entry.id)?;
        for session in tree.sessions {
            if mux_name_matches(&repo_basename, &session.name, mux_name) {
                return Ok(Some((repo_entry.clone(), session)));
            }
        }
    }
    Ok(None)
}

fn repo_basename(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn find_session_by_name(
    repos: &[RepoEntry],
    repo_id: &str,
    session_name: &str,
) -> Result<Option<(RepoEntry, Session)>> {
    for repo_entry in repos {
        if repo_entry.id != repo_id {
            continue;
        }
        let tree = store::load_sessions(&repo_entry.id)?;
        if let Some(session) = tree.find_by_name(session_name) {
            return Ok(Some((repo_entry.clone(), session.clone())));
        }
    }
    Ok(None)
}

fn find_session_by_path(repos: &[RepoEntry], path: &Path) -> Result<Option<(RepoEntry, Session)>> {
    let current_path = normalize_path(path);
    let mut best: Option<(RepoEntry, Session, usize)> = None;

    for repo_entry in repos {
        let tree = store::load_sessions(&repo_entry.id)?;
        for session in tree.sessions {
            let Some(session_path) = session_path(&session, repo_entry) else {
                continue;
            };
            let normalized_session_path = normalize_path(session_path);
            if path_matches_current(&current_path, &normalized_session_path) {
                let depth = normalized_session_path.components().count();
                if best
                    .as_ref()
                    .map(|(_, _, best_depth)| depth > *best_depth)
                    .unwrap_or(true)
                {
                    best = Some((repo_entry.clone(), session, depth));
                }
            }
        }
    }

    Ok(best.map(|(repo_entry, session, _)| (repo_entry, session)))
}

fn session_path<'a>(session: &'a Session, repo_entry: &'a RepoEntry) -> Option<&'a Path> {
    session
        .path
        .as_deref()
        .or_else(|| session.is_default.then_some(repo_entry.path.as_path()))
}

fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn path_matches_current(current_path: &Path, session_path: &Path) -> bool {
    current_path == session_path || current_path.starts_with(session_path)
}

#[cfg(test)]
mod tests {
    use super::{encode_mux_name, mux_name_matches, path_matches_current};
    use std::path::Path;

    // The shortened names below were produced by `fit_name` in
    // plugins/zellij/zellij-plugin under a 24-byte budget (macOS default
    // $TMPDIR). They are golden values: if the two implementations of the
    // digest ever drift, these fail rather than "current session not found".

    #[test]
    fn mux_name_matches_full_encoding() {
        assert!(mux_name_matches("my-repo", "main", "my-repo__main"));
        assert!(!mux_name_matches("my-repo", "main", "other__main"));
    }

    #[test]
    fn mux_name_matches_shortened_encoding() {
        // Session name intact, repo replaced by the digest of the full name.
        assert!(mux_name_matches(
            "acme-widgets",
            "refactor-auth-flow",
            "refactor-auth-flow_7239"
        ));
    }

    #[test]
    fn mux_name_matches_shortened_and_truncated_encoding() {
        // Too long even without the repo prefix, so the session name is cut too.
        assert!(mux_name_matches(
            "acme-widgets",
            "feat-ABC-123-add-dark-mode-toggle",
            "feat-ABC-123-add-da_eb18"
        ));
    }

    #[test]
    fn shortened_encoding_distinguishes_repos() {
        // Same session name in two repos: only the digest tells them apart.
        assert!(mux_name_matches(
            "shared-component-library",
            "refactor-auth-flow",
            "refactor-auth-flow_6f3e"
        ));
        assert!(!mux_name_matches(
            "shared-component-library",
            "refactor-auth-flow",
            "refactor-auth-flow_7239"
        ));
    }

    #[test]
    fn shortened_encoding_distinguishes_names_truncated_alike() {
        // Both truncate to "feat-ABC-123-add-da"; the digest covers the full name.
        assert!(mux_name_matches(
            "acme-widgets",
            "feat-ABC-123-add-dark-mode",
            "feat-ABC-123-add-da_9fe7"
        ));
        assert!(!mux_name_matches(
            "acme-widgets",
            "feat-ABC-123-add-dark-theme",
            "feat-ABC-123-add-da_9fe7"
        ));
    }

    #[test]
    fn shortened_encoding_agrees_on_non_ascii() {
        // 'é' is two bytes, so it becomes two underscores on both sides before
        // the digest is taken — the case where a char-wise bash `tr` or a
        // char-wise Rust map would diverge.
        assert!(mux_name_matches(
            "acme-widgets",
            "feat/ABC-1 café",
            "feat_ABC-1_caf___fda6"
        ));
    }

    #[test]
    fn mux_name_rejects_mismatched_digest_and_prefix() {
        // Right prefix, wrong digest.
        assert!(!mux_name_matches(
            "acme-widgets",
            "refactor-auth-flow",
            "refactor-auth-flow_0000"
        ));
        // Right digest length, prefix not from this session's name.
        assert!(!mux_name_matches(
            "acme-widgets",
            "refactor-auth-flow",
            "some-other-name_7239"
        ));
        // Digest of the wrong length is not a shortened name at all.
        assert!(!mux_name_matches(
            "acme-widgets",
            "refactor-auth-flow",
            "refactor-auth-flow_723"
        ));
    }

    #[test]
    fn encode_joins_parts_with_double_underscore() {
        assert_eq!(encode_mux_name("my-repo", "main"), "my-repo__main");
    }

    #[test]
    fn encode_replaces_separators_and_punctuation() {
        assert_eq!(
            encode_mux_name("my.repo", "feat/ABC-1"),
            "my_repo__feat_ABC-1"
        );
        assert_eq!(encode_mux_name("a:b", "c d"), "a_b__c_d");
        assert_eq!(encode_mux_name(".dotfiles", "wip"), "_dotfiles__wip");
    }

    #[test]
    fn encode_never_emits_a_slash() {
        // zellij rejects session names containing '/'
        assert!(!encode_mux_name("owner/repo", "a/b/c").contains('/'));
    }

    #[test]
    fn encode_replaces_non_ascii_per_byte() {
        // Must match `LC_ALL=C tr -c 'A-Za-z0-9_-' '_'` in the zellij plugin:
        // 'é' is two UTF-8 bytes, so it becomes two underscores.
        assert_eq!(encode_mux_name("repo", "café"), "repo__caf__");
    }

    #[test]
    fn encode_is_lossy_and_can_collide() {
        // Documents the first-match-wins rule in find_session_by_mux_name.
        assert_eq!(encode_mux_name("a.b", "c"), encode_mux_name("a:b", "c"));
    }

    #[test]
    fn path_match_accepts_session_root() {
        assert!(path_matches_current(
            Path::new("/tmp/repo-worktree"),
            Path::new("/tmp/repo-worktree")
        ));
    }

    #[test]
    fn path_match_accepts_descendant_of_session_root() {
        assert!(path_matches_current(
            Path::new("/tmp/repo-worktree/src/module"),
            Path::new("/tmp/repo-worktree")
        ));
    }

    #[test]
    fn path_match_rejects_common_prefix_that_is_not_parent() {
        assert!(!path_matches_current(
            Path::new("/tmp/repo-worktree-extra"),
            Path::new("/tmp/repo-worktree")
        ));
    }
}
