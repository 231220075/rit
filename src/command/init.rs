use crate::{
    GitError,
    Result,
};
use super::SubCommand;

#[derive(Debug)]
pub struct Init;

impl Init {
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        Ok(Box::new(Self {}))
    }
}

impl SubCommand for Init {
    fn run(&self) -> Result<()> {
        Ok(())
    }
}
