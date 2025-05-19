use std::path::PathBuf;
use clap::{Parser, Subcommand, Command};
use std::io::{self, Write, Cursor};
use std::process::Command as ProcessCommand;

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
        let tree_hash = {
            let output = ProcessCommand::new("sh")
                .arg("-c")
                .arg(format!("echo $(./git write-tree)"))
                .output()
                .expect("Failed to execute WriteTree");
            String::from_utf8(output.stdout).unwrap().trim().to_string()
        };

        let head_ref = read_head_ref(&gitdir)?;
        let parent_commit = read_ref_commit(&gitdir, &head_ref).ok();
        let commit_tree_args = {
            let mut args = vec![];
            if let Some(msg) = &self.message {
                args.push("-m".to_string());
                args.push(msg.clone());
            }
            if let Some(parent) = parent_commit {
                if !parent.trim().is_empty() { // 判断 parent 是否为空白
                    args.push("-p".to_string());
                    args.push(parent);
                }
            }
            args.push(tree_hash.to_string()); // 将 TREE_HASH 放在最后
            args
        };
        let commit_hash = {
            let output = ProcessCommand::new("sh")
                .arg("-c")
                .arg(format!("echo $(./git commit-tree {})", commit_tree_args.join(" ")))
                .output()
                .expect("Failed to execute CommitTree");
            String::from_utf8(output.stdout).unwrap().trim().to_string()

        };

        
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
