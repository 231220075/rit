use std::path::PathBuf;
use std::result;
use clap::{Parser, Subcommand};

use crate::{
    GitError,
    Result,
};
use super::SubCommand;


#[derive(Parser, Debug)]
#[command(name = "add", about = "将文件内容添加到索引中")]
pub struct Add {
    #[arg(short = 'n', long = "dry-run", help = "dry run", action = clap::ArgAction::SetTrue, required = false)]
    dry_run: bool,

    #[arg(required = true, num_args = 1.., value_parser = Add::parse_paths)]
    paths: Vec<PathBuf>,
}

impl Add {
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        Ok(Box::new(Add::try_parse_from(args)?))
    }

    fn parse_paths(arg: &str) -> result::Result<PathBuf, String> {
        let path = PathBuf::from(arg);
        if path.exists() {
            Ok(path)
        }
        else {
            Err(format!("{} not found", arg))
        }
    }

}

impl SubCommand for Add {
    fn run(&self, _gitdir: Result<PathBuf>) -> Result<i32> {
        println!("{:?} {}", self.dry_run, self.paths.iter().flat_map(|x|x.to_str().map(String::from))
            .fold(String::from(""), |mut pre: String, curr: String| {
                            pre.push(' ');
                            pre.push_str(&curr);
                            pre
                        }));
        Ok(0)
    }
}
