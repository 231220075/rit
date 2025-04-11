use crate::{
    GitError,
    Result,
};
use super::SubCommand;

enum CommitOption {
    Message {
        message: String
    },              //  [-m | --message] commit message
    All,            // [-a | --all ] commit all changed files
}

pub struct Commit {
    option: Option<CommitOption>,
}

impl Commit {
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        Err(GitError::new_file_notfound("InvalidCommand".to_string()))
    }
}

impl SubCommand for Commit {
    fn run(&self) -> Result<()> {
        Ok(())
    }
}
