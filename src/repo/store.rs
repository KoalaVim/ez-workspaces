use std::fs;

use crate::error::Result;
use crate::paths;

use super::model::{RepoIndex, RepoMeta};

/// Load the repo index from disk, returning an empty one if it doesn't exist.
pub fn load_index() -> Result<RepoIndex> {
    let path = paths::repos_index_file()?;
    if !path.exists() {
        return Ok(RepoIndex::default());
    }
    let contents = fs::read_to_string(&path)?;
    let index: RepoIndex = toml::from_str(&contents)?;
    Ok(index)
}

/// Save the repo index to disk.
pub fn save_index(index: &RepoIndex) -> Result<()> {
    let path = paths::repos_index_file()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = toml::to_string_pretty(index)?;
    fs::write(&path, contents)?;
    Ok(())
}

/// Load per-repo metadata.
pub fn load_repo_meta(repo_id: &str) -> Result<RepoMeta> {
    let path = paths::repo_meta_file(repo_id)?;
    if !path.exists() {
        return Ok(RepoMeta::default());
    }
    let contents = fs::read_to_string(&path)?;
    let meta: RepoMeta = toml::from_str(&contents)?;
    Ok(meta)
}

/// Save per-repo metadata.
pub fn save_repo_meta(repo_id: &str, meta: &RepoMeta) -> Result<()> {
    let path = paths::repo_meta_file(repo_id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = toml::to_string_pretty(meta)?;
    fs::write(&path, contents)?;
    Ok(())
}

/// Delete per-repo metadata directory.
pub fn delete_repo_meta(repo_id: &str) -> Result<()> {
    let dir = paths::repo_meta_dir(repo_id)?;
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    Ok(())
}
