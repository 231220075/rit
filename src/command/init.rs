use crate::{
    GitError,
    Result,
};
use super::{
    SubCommand,
    PathBuf
};
use std::fs;

#[derive(Debug)]
pub struct Init;

impl Init {
    pub fn from_args(_args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        Ok(Box::new(Self {}))
    }
}

impl SubCommand for Init {
    fn run(&self, _gitdir: Result<PathBuf>) -> Result<i32> {
        let current_dir = std::env::current_dir().map_err(|_| GitError::invalid_command("Failed to get current directory".to_string()))?;
        let git_dir = current_dir.join(".git");
        if git_dir.exists() {
            return Err(GitError::invalid_command(".git directory already exists".to_string()));
        }
        fs::create_dir(&git_dir).map_err(|_| GitError::invalid_command("Failed to create .git directory".to_string()))?;
        let refs_heads_dir = git_dir.join("refs").join("heads");
        fs::create_dir_all(&refs_heads_dir).map_err(|_| GitError::invalid_command("Failed to create refs/heads directory".to_string()))?;
        let objects_dir = git_dir.join("objects");
        fs::create_dir(&objects_dir).map_err(|_| GitError::invalid_command("Failed to create objects directory".to_string()))?;
        let head_file = git_dir.join("HEAD");
        fs::write(&head_file, "ref: refs/heads/master\n").map_err(|_| GitError::invalid_command("Failed to create HEAD file".to_string()))?;

        //println!("Initialized empty Git repository in {}", git_dir.display());
        Ok(0)
    }
}
