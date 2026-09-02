use std::fs;

use crate::error::Result;
use crate::paths;

use super::model::SessionTree;

/// Load sessions for a repo, returning an empty tree if none exist.
pub fn load_sessions(repo_id: &str) -> Result<SessionTree> {
    let path = paths::sessions_file(repo_id)?;
    if !path.exists() {
        return Ok(SessionTree::default());
    }
    let contents = fs::read_to_string(&path)?;
    let mut tree: SessionTree = toml::from_str(&contents)?;
    for session in &mut tree.sessions {
        session.path = session.path.as_ref().map(|p| paths::strip_unc_prefix(p));
    }
    Ok(tree)
}

/// Save sessions for a repo.
pub fn save_sessions(repo_id: &str, tree: &SessionTree) -> Result<()> {
    let path = paths::sessions_file(repo_id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = toml::to_string_pretty(tree)?;
    fs::write(&path, contents)?;
    Ok(())
}
