use std::path::Path;
use crate::{GitError, Result};
use std::fs;

pub fn read_head_ref(gitdir: &Path) -> Result<String> {
    let head_path = gitdir.join("HEAD");
    let content = fs::read_to_string(&head_path)
        .map_err(|_| GitError::FileNotFound(head_path.display().to_string()))?;
    // 解析 ref: refs/heads/main
    if let Some(rest) = content.strip_prefix("ref: ") {
        Ok(rest.trim().to_string())
    } else {
        Ok(content.trim().to_string())
    }
}

pub fn write_head_ref(gitdir: &Path, ref_path: &str) -> Result<()> {
    let head_file = gitdir.join("HEAD");
    fs::write(&head_file, format!("ref: {}\n", ref_path))
        .map_err(|_| GitError::failed_to_write_file(&head_file.to_string_lossy()))?;
    Ok(())
}

pub fn read_ref_commit(gitdir: &Path, refname: &str) -> Result<String> {
    let ref_path = gitdir.join(refname);
    let content = fs::read_to_string(&ref_path)
        .map_err(|_| GitError::FileNotFound(ref_path.display().to_string()))?;
    Ok(content.trim().to_string())
}

pub fn write_ref_commit(gitdir: &Path, ref_path: &str, hash: &str) -> Result<()> {
    let ref_file = gitdir.join(ref_path);
    fs::write(&ref_file, format!("{}\n", hash))
        .map_err(|_| GitError::failed_to_write_file(&ref_file.to_string_lossy()))?;
    Ok(())
}