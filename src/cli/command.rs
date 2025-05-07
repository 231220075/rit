#[allow(unused)]
use crate::{
    command::SubCommand,
    utils::fs::get_git_dir,
    GitError,
    Result,
};

pub fn git_execute(args: Box<dyn SubCommand>) -> Result<i32> {
    args.run(get_git_dir())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {

    }
}
