use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::result;
use crate::{
    GitError,
    Result,
    utils::{
        index::Index,
        fs::{
            calc_relative_path,
            walk,
        },
    }
};
use super::SubCommand;


#[derive(Parser, Debug)]
#[command(name = "rm", about = "从工作树和索引中删除文件")]
pub struct Rm {
    #[arg(long, help = "only remove from the index")]
    cached: bool,

    #[arg(short='n', long="dry-run", help = "dry run")]
    dry_run: bool,

    #[arg(required = true, value_name="paths", num_args = 1..)]
    paths: Vec<PathBuf>,
}

impl Rm {
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        let a = Rm::try_parse_from(args)?;
        println!("{:?}", a);
        Ok(Box::new(a))
    }
}

impl SubCommand for Rm {
    fn run(&self, gitdir: Result<PathBuf>) -> Result<i32> {
        let gitdir = gitdir?;
        let index_file = gitdir.join("index");
        let project_root = gitdir.parent().expect("find git dir implementation fail");

        let mut index = Index::new();
        if index_file.exists() {
            index.read_from_file(&gitdir.join("index"))?;
        }

        if self.cached {
            let paths = self.paths.iter().map(|path|calc_relative_path(project_root, path)).collect::<Result<Vec<_>>>()?;
            let possible_file = paths.iter().filter(|p|p.is_file()).collect::<Vec<_>>();
            let possible_dir = paths.iter().filter(|p|p.is_dir()).collect::<Vec<_>>();
            if let Some(path) = possible_file
                .iter()
                .filter(|p| !index.entries.iter().any(|en| en.name == p.to_str().unwrap()))
                .take(1).next()
            {
                        Err(GitError::not_a_repofile(path))
            }
            else {
                possible_dir
                    .into_iter()
                    .map(walk)
                    .collect::<Result<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .chain(possible_file.iter().map(|x|(**x).clone()))
                    .for_each(|path| {
                        if let Some((idx, _)) = index.entries
                            .iter()
                            .enumerate()
                            .find(|(_, en)|en.name == path.to_str().unwrap())
                        {
                            index.entries.remove(idx);
                        };
                    });
                index.write_to_file(&index_file)?;
                Ok(0)
            }
        }
        else {
            todo!("直接从文件系统中删除");
        }
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
            (&["rm", "--cached", file1_str, file2_str], true),
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
