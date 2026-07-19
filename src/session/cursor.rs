use std::fs;
use std::path::Path;

/// Compute the Cursor workspace slug for a given path.
/// Mirrors Cursor's algorithm: replace non-alphanumeric chars with dashes,
/// collapse consecutive dashes, strip leading/trailing dashes.
pub fn cursor_slug(path: &Path) -> String {
    let path_str = path.display().to_string();
    let mut slug = String::with_capacity(path_str.len());
    for ch in path_str.chars() {
        if ch.is_alphanumeric() {
            slug.push(ch);
        } else {
            slug.push('-');
        }
    }
    // Collapse consecutive dashes
    let mut collapsed = String::with_capacity(slug.len());
    let mut prev_dash = false;
    for ch in slug.chars() {
        if ch == '-' {
            if !prev_dash {
                collapsed.push('-');
            }
            prev_dash = true;
        } else {
            collapsed.push(ch);
            prev_dash = false;
        }
    }
    collapsed.trim_matches('-').to_string()
}

/// Compute the Cursor chat hash for a workspace path.
/// Uses md5(realpath(workspace_path)) and returns the hex digest.
pub fn cursor_chat_hash(workspace_path: &Path) -> String {
    let real_path = workspace_path
        .canonicalize()
        .unwrap_or_else(|_| workspace_path.to_path_buf());
    let digest = md5::compute(real_path.display().to_string().as_bytes());
    format!("{:x}", digest)
}

/// Copy Cursor conversations from old workspace path to new workspace path.
/// Best-effort: logs operations, swallows all errors.
pub fn copy_cursor_conversations(old_path: &Path, new_path: &Path) {
    let cursor_dir = match dirs::home_dir() {
        Some(home) => home.join(".cursor"),
        None => {
            log::debug!("copy_cursor_conversations: cannot determine home directory");
            return;
        }
    };

    // Copy agent-transcripts
    let old_slug = cursor_slug(old_path);
    let new_slug = cursor_slug(new_path);
    log::debug!(
        "copy_cursor_conversations: old_slug='{}' new_slug='{}'",
        old_slug,
        new_slug
    );

    let old_transcripts = cursor_dir
        .join("projects")
        .join(&old_slug)
        .join("agent-transcripts");
    let new_transcripts = cursor_dir
        .join("projects")
        .join(&new_slug)
        .join("agent-transcripts");

    if old_transcripts.is_dir() {
        if let Err(e) = copy_dir_contents(&old_transcripts, &new_transcripts) {
            log::debug!(
                "copy_cursor_conversations: failed to copy agent-transcripts: {}",
                e
            );
        } else {
            log::debug!(
                "copy_cursor_conversations: copied agent-transcripts from '{}' to '{}'",
                old_transcripts.display(),
                new_transcripts.display()
            );
        }
    } else {
        log::debug!(
            "copy_cursor_conversations: no agent-transcripts at '{}'",
            old_transcripts.display()
        );
    }

    // Copy chats
    let old_hash = cursor_chat_hash(old_path);
    let new_hash = cursor_chat_hash(new_path);
    log::debug!(
        "copy_cursor_conversations: old_hash='{}' new_hash='{}'",
        old_hash,
        new_hash
    );

    let old_chats = cursor_dir.join("chats").join(&old_hash);
    let new_chats = cursor_dir.join("chats").join(&new_hash);

    if old_chats.is_dir() {
        if let Err(e) = copy_dir_contents(&old_chats, &new_chats) {
            log::debug!("copy_cursor_conversations: failed to copy chats: {}", e);
        } else {
            log::debug!(
                "copy_cursor_conversations: copied chats from '{}' to '{}'",
                old_chats.display(),
                new_chats.display()
            );
        }
    } else {
        log::debug!(
            "copy_cursor_conversations: no chats at '{}'",
            old_chats.display()
        );
    }
}

/// Recursively copy contents of one directory to another, creating the target if needed.
fn copy_dir_contents(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_contents(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_cursor_slug_basic() {
        let path = PathBuf::from("/Users/ofirg/workspace/my-project");
        let slug = cursor_slug(&path);
        assert_eq!(slug, "Users-ofirg-workspace-my-project");
    }

    #[test]
    fn test_cursor_slug_collapses_dashes() {
        let path = PathBuf::from("/Users///ofirg//workspace");
        let slug = cursor_slug(&path);
        assert_eq!(slug, "Users-ofirg-workspace");
    }

    #[test]
    fn test_cursor_chat_hash_deterministic() {
        let path = PathBuf::from("/tmp/test-workspace");
        let hash1 = cursor_chat_hash(&path);
        let hash2 = cursor_chat_hash(&path);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 32);
    }
}
