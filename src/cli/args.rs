use crate::{
    GitError,
    Result,
}; 
use crate::command::{Init, Add, Rm, Commit, SubCommand};


pub fn get_args(mut raw_args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
    let command = raw_args.next()
        .ok_or(GitError::no_subcommand())?;

    match command.as_str() {
        "commit" => Commit::from_args(raw_args),
        "init"   => Init::from_args(raw_args),
        "add"    => Add::from_args(raw_args),
        "rm"     => Rm::from_args(raw_args),
        _        => Err(GitError::invalid_command())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_init() {
        
    }
}
