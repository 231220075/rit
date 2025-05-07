use crate::{
    GitError,
    Result,
};
use super::{
    SubCommand,
    PathBuf
};

#[derive(Debug)]
pub struct Init;

impl Init {
    pub fn from_args(_args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        Ok(Box::new(Self {}))
    }
}

impl SubCommand for Init {
    fn run(&self, _gitdir: Result<PathBuf>) -> Result<i32> {
        Ok(0)
    }
}
