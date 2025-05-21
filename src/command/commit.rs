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

        let tree_hash = WriteTree::lazy_fucker(gitdir.clone())?;

        let head_ref = read_head_ref(&gitdir)?;
        let parent_commit = read_ref_commit(&gitdir, &head_ref).ok();
        let commit_tree = CommitTree {
            tree_hash,
            message: self.message.clone().unwrap(),
            pcommit: parent_commit
        };

        let commit_hash = commit_tree.asshole(gitdir.clone())?;

        let update_ref = UpdateRef {
            ref_path: head_ref,
            commit_hash: commit_hash.clone()
        };
        update_ref.run(Ok(gitdir))?;

        println!("{}", commit_hash);
        Ok(0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::test::{
        shell_spawn,
        setup_test_git_dir,
        mktemp_in,
        touch_file_in,
        cmd_seq,
        tempdir,
        cp_dir,
        run_both,
        ArgsList,
    };

    #[test]
    fn test_simple() {
        let temp1 = setup_test_git_dir();
        let temp_path1 = temp1.path();
        let temp_path_str1 = temp_path1.to_str().unwrap();

        let temp2 = tempdir().unwrap();
        let temp_path2 = temp2.path();
        let temp_path_str2 = temp_path2.to_str().unwrap();

        let file1 = mktemp_in(&temp1).unwrap();
        let file1_str = file1.file_name().unwrap();
        let file1_str = file1_str.to_str().unwrap();

        let file2 = mktemp_in(&temp1).unwrap();
        let file2_str = file2.file_name().unwrap().to_str();
        let file2_str = file2_str.unwrap();

        let _ = cp_dir(temp_path1, temp_path2).unwrap();

        let cmds: ArgsList = &[
            (&["add", file1_str, file2_str], true),
            (&["commit", "-m", "commit message"], true),
        ];
        let git = &["git", "-C", temp_path_str1];
        let cargo = &["cargo", "run", "--quiet", "--", "-C", temp_path_str2];
        let _ = run_both(cmds, git, cargo).unwrap();

        println!("{}", shell_spawn(&["ls", "-lahR", temp_path_str2]).unwrap());
        println!("{}", shell_spawn(&["cat", PathBuf::from(temp_path_str2).join(".git").join("HEAD").to_str().unwrap()]).unwrap());

        let origin = shell_spawn(&["git", "-C", temp_path_str1, "symbolic-ref", "HEAD"]).unwrap();
        let real = shell_spawn(&["git", "-C", temp_path_str2, "symbolic-ref", "HEAD"]).unwrap();
        assert_eq!(origin, real);
    }
}
