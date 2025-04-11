use std::path::PathBuf;
use crate::{
    GitError,
    Result,
};
use super::SubCommand;

enum RmOption {
    Cached, // [--cahced] only remove from the index
    DryRun, // [-n | --dry-run]  dry run
}
pub struct Rm {
    option: Option<RmOption>,
    paths: Vec<PathBuf>,
}

impl Rm {
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        Err(GitError::new_file_notfound("InvalidCommand".to_string()))
    }
}

impl SubCommand for Rm {
    fn run(&self) -> Result<()> {
        Ok(())
    }
}
