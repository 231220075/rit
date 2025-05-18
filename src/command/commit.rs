use std::path::PathBuf;
use clap::{Parser, Subcommand};
use std::io;

use crate::{
    GitError,
    Result,
};
use super::SubCommand;
use crate::command::{WriteTree, CommitTree, UpdateRef};
use crate::utils::refs::{read_head_ref, read_ref_commit};

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
            .ok_or_else(||GitError::invalid_command("todo, 在这里调用$EDITOR".to_string()))
            .map(|message| Box::new(Commit {
                message: Some(message),
                all: cli.all,
            }) as Box<dyn SubCommand>)
    }
}

impl SubCommand for Commit {
    fn run(&self, gitdir: Result<PathBuf>) -> Result<i32> {
        let gitdir = gitdir?;
        let tree_hash = WriteTree{}.run(Ok(gitdir.clone()))?;

        let head_ref = read_head_ref(&gitdir)?;
        let parent_commit = read_ref_commit(&gitdir, &head_ref).ok();

        let commit_tree_args = {
            let mut args = vec!["commit-tree".to_string(), tree_hash.to_string()];
            if let Some(msg) = &self.message {
                args.push("-m".to_string());
                args.push(msg.clone());
            }
            if let Some(parent) = parent_commit {
                args.push("-p".to_string());
                args.push(parent);
            }
            args
        };
        let commit_hash = CommitTree::from_args(commit_tree_args.into_iter())?
            .run(Ok(gitdir.clone()))?;

        
        //todo: update HEAD
        let update_ref_args = vec![
            "update-ref".to_string(),
            head_ref.clone(),
            commit_hash.to_string(),
        ];
        UpdateRef::from_args(update_ref_args.into_iter())?
            .run(Ok(gitdir.clone()))?;

        println!("{}", commit_hash);
        Ok(0)
    }
}
