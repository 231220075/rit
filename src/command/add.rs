use std::{
    path::{
        PathBuf,
        Path,
    },
    result,
    iter::once,
};
use clap::{Parser, Subcommand};
use itertools::Either;

use crate::{
    GitError,
    Result,
    utils::{
        objtype::ObjType,
        index::{
            Index,
            IndexEntry,
        },
        fs::{
            write_object,
            read_file_as_bytes,
            add_object,
        },
        tree::FileMode,
        blob::Blob,
    },
};

use super::SubCommand;


#[derive(Parser, Debug)]
#[command(name = "add", about = "将文件内容添加到索引中")]
pub struct Add {
    #[arg(short = 'n', long = "dry-run", help = "dry run", action = clap::ArgAction::SetTrue, required = false)]
    dry_run: bool,

    #[arg(required = true, num_args = 1..)]
    paths: Vec<PathBuf>,
}

impl Add {
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        Ok(Box::new(Add::try_parse_from(args)?))
    }

    fn walk_path(&self, project_root: PathBuf) -> Result<Vec<PathBuf>> {
        self.paths.clone()
            .into_iter()
            .map(|path| {
                if project_root.join(&path).is_dir() {
                    path.read_dir()?
                        .map(|x|
                            x.map(|x|x.path())
                                .map_err(GitError::no_permision)
                        )
                        .collect::<Result<Vec<_>>>()
                }
                else {
                    Ok([path].to_vec())
                }
            })
            .collect::<Result<Vec<_>>>()
            .map(|vv| {
                println!("len = {}", vv.len());
                vv.into_iter().flat_map(|x|x.into_iter())
                    .collect::<Vec<_>>()
            })

    }
}

impl SubCommand for Add {
    fn run(&self, gitdir: Result<PathBuf>) -> Result<i32> {
        let gitdir = gitdir?;
        let index_file = gitdir.join("index");
        let project_root = gitdir.parent().expect("find git dir implementation fail");

        let mut index = Index::new();
        if index_file.exists() {
            index.read_from_file(&gitdir.join("index"))?;
        }

        let _ = self.walk_path(project_root.to_path_buf())?
            .into_iter()
            .map(|path| -> Result<()> {
                index.add_entry(add_object::<Blob>(gitdir.clone(), path)?);
                Ok(())
            })
            .collect::<Result<Vec<_>>>()?;
        index.write_to_file(&index_file)?;
        Ok(0)
    }
}


#[cfg(test)]
mod test {
    use crate::utils::test::{
        shell_spawn,
        setup_test_git_dir,
        mktemp_in,
        cmd_seq,
        tempdir,
        cp_dir,
        run_both,
        ArgsList,
    };

    #[test]
    fn test_basic() {
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
        ];
        let git = &["git", "-C", temp_path_str1];
        let cargo = &["cargo", "run", "--quiet", "--", "-C", temp_path_str2];
        let _ = run_both(cmds, git, cargo).unwrap();

        println!("{}", shell_spawn(&["ls", "-lahR", temp_path_str1]).unwrap());

        let origin = shell_spawn(&["git", "-C", temp_path_str1, "ls-files", "--stage", "|", "sort"]).unwrap();
        let real = shell_spawn(&["git", "-C", temp_path_str2, "ls-files", "--stage", "|", "sort"]).unwrap();
        assert_eq!(origin, real);
    }
}
