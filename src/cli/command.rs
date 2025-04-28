#[allow(unused)]
use crate::{
    GitError,
    Result,
};
use crate::command::SubCommand;

pub fn git_execute(args: Box<dyn SubCommand>) -> Result<i32> {
    args.run()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        
    }
}
