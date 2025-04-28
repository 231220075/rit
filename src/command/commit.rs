use std::path::PathBuf;
use clap::{Parser, Subcommand};
use std::io;

use crate::{
    GitError,
    Result,
};
use super::SubCommand;


#[derive(Parser, Debug)]
#[command(name = "commit", about = "记录对存储库的更改")]
pub struct Commit {
    #[arg(short, long, value_name="MESSAGE", help = "commit message")]
    pub message: Option<String>,

    #[arg(short, long, help = "commit all changed files")]
    pub all: bool
}

impl Commit {
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        let cli = Commit::try_parse_from(args)?;
        cli.message
            .ok_or_else(||GitError::new_invalid_command("todo, 在这里调用$EDITOR".to_string()))
            .map(|message| Box::new(Commit {
                message: Some(message),
                all: cli.all,
            }) as Box<dyn SubCommand>)
    }
}

impl SubCommand for Commit {
    fn run(&self) -> Result<i32> {
        println!("message: {:?}, all: {}", self.message, self.all);
        Ok(0)
    }
}
