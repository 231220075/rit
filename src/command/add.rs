use std::path::PathBuf;
use clap::{Parser, Subcommand};

use crate::{
    GitError,
    Result,
};
use super::SubCommand;

// A simple CLI tool to add files or directories.
#[derive(Parser, Debug)]
#[command(name = "add", about = "用法：git add [<选项>] [--] <路径规格>...]]")]
struct Cli {
    // Enable dry-run mode (n  o actual changes)
    #[arg(short = 'n', long = "dry-run", action = clap::ArgAction::SetTrue)]
    dry_run:  bool,

    // Paths to add
    #[arg(required = true, num_args = 1..)]
    paths:  Vec<String>,
}



pub struct Add {
    dry_run: bool,
    paths: Vec<PathBuf>,
}

impl Add {
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        let cli = Cli::try_parse_from(args)?;
        if cli.dry_run {
            println!("add dry_run");
        }

        let mut non_exist = cli.paths.iter()
            .map(PathBuf::from)
            .filter(|p| !p.exists())
            .peekable();

        if non_exist.peek().is_none() {
            Ok(Box::new(Self {
                dry_run: cli.dry_run,
                paths: cli.paths.iter().map(PathBuf::from).collect(),
            }))
        }
        else {
            Err(GitError::new_file_notfound(
                    non_exist.flat_map(|x|x.to_str().map(String::from))
                        .fold(String::from(""), |mut pre: String, curr: String|  pre + &curr)
                )
            )
        }
    }
}

impl SubCommand for Add {
    fn run(&self) -> Result<()> {
        println!("{:?} {}", self.dry_run, self.paths.iter().flat_map(|x|x.to_str().map(String::from))
            .fold(String::from(""), |mut pre: String, curr: String| {
                            pre.push_str(" ");
                            pre.push_str(&curr);
                            pre
                        }));
        Ok(())
    }
}
