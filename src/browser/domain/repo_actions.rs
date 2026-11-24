//! Repo-level actions of the global views (labels, remove, clone naming).

use std::path::{Path, PathBuf};

use colored::Colorize;

use super::super::format::parse_label_input;
use super::super::selector::InteractiveSelector;
use super::{Effect, Level, RefreshScope};
use crate::error::Result;
use crate::repo;
use crate::repo::model::RepoEntry;

/// Edit a repo's labels through an input prompt (comma-separated, `-` prefix removes).
pub(crate) fn edit_labels(
    selector: &dyn InteractiveSelector,
    entry: &RepoEntry,
) -> Result<Vec<Effect>> {
    let meta = repo::store::load_repo_meta(&entry.id).unwrap_or_default();
    let current = meta.labels.join(",");
    let input = selector.input("Labels (comma-sep; prefix - to remove)", Some(&current))?;
    let (add, remove) = parse_label_input(&input);
    let result = repo::set_repo_labels(&entry.id, &add, &remove)?;
    let labels = if result.is_empty() {
        "(none)".dimmed().to_string()
    } else {
        result.join(", ").magenta().to_string()
    };
    Ok(vec![
        Effect::Notify(
            format!("{} {} → {}", "Labels on".green(), entry.name.bold(), labels),
            Level::Info,
        ),
        Effect::Refresh(RefreshScope::Repos),
    ])
}

/// Unregister a repo after confirmation (files stay on disk).
pub(crate) fn remove(selector: &dyn InteractiveSelector, entry: &RepoEntry) -> Result<Vec<Effect>> {
    if !selector.confirm(&format!("Remove repo '{}'?", entry.name), false)? {
        return Ok(Vec::new());
    }
    let mut index = repo::store::load_index()?;
    index.remove_by_id(&entry.id);
    repo::store::save_index(&index)?;
    Ok(vec![
        Effect::Notify(
            format!("{} {}", "Removed:".green(), entry.name.bold()),
            Level::Info,
        ),
        Effect::Refresh(RefreshScope::Repos),
    ])
}

/// Where `git clone <url>` should land inside `dir` (the last URL segment, without `.git`).
pub(crate) fn clone_target(dir: &Path, url: &str) -> PathBuf {
    let name = url
        .trim()
        .rsplit('/')
        .next()
        .unwrap_or("repo")
        .trim_end_matches(".git");
    dir.join(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_target_uses_last_segment_without_git_suffix() {
        let dir = Path::new("/ws");
        assert_eq!(
            clone_target(dir, " https://github.com/o/repo.git "),
            PathBuf::from("/ws/repo")
        );
        assert_eq!(
            clone_target(dir, "git@github.com:o/other"),
            PathBuf::from("/ws/other")
        );
    }
}
