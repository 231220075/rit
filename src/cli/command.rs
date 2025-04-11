#[allow(unused)]
use crate::{
    GitError,
    Result,
};
use crate::command::SubCommand;

pub fn git_execute(args: Box<dyn SubCommand>) -> Result<()> {
    match args.run() {
        Ok(()) => Ok(()),
        Err(err) => {
            println!("{}", err);
            Ok(())
        },
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        
    }
}
