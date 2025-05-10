use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::result;
use crate::{
    GitError,
    Result,
};
use super::SubCommand;


#[derive(Parser, Debug)]
#[command(name = "rm", about = "从工作树和索引中删除文件")]
pub struct Rm {
    #[arg(long, help = "only remove from the index")]
    cached: bool,

    #[arg(short='n', long="dry-run", help = "dry run")]
    dry_run: bool,

    #[arg(required = true, value_name="paths", num_args = 1.., value_parser = Rm::parse_path)]
    paths: Vec<PathBuf>,
}

impl Rm {
    fn parse_path(arg: &str) -> result::Result<PathBuf, String> {
        let path = PathBuf::from(arg);
        if path.exists() {
            Ok(path)
        }
        else {
            Err(format!("{} not found", arg))
        }
    }

    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        let a = Rm::try_parse_from(args)?;

        println!("{:?}", a);
        Ok(Box::new(a))
    }
}

impl SubCommand for Rm {
    fn run(&self, _gitdir: Result<PathBuf>) -> Result<i32> {
        println!("{:?} {}", self.cached, self.paths.iter().flat_map(|x|x.to_str().map(String::from))
            .fold(String::from(""), |mut pre: String, curr: String| {
                            pre.push(' ');
                            pre.push_str(&curr);
                            pre
                        }));
        Ok(0)
    }
}
