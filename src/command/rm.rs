use clap::{Parser, Subcommand};
use itertools::Itertools;
use std::{
    path::{
        PathBuf,
        Path
    },
    fs::remove_file,
};
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

    #[arg(short='r', long="recursive", help = "rm dir recursively")]
    recursive: bool,

    #[arg(required = true, value_name="paths", num_args = 1..)]
    paths: Vec<PathBuf>,
}

impl Rm {
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        let a = Rm::try_parse_from(args)?;
        //println!("{:?}", a);
        Ok(Box::new(a))
    }

    fn walks_all_path(&self, project_root: PathBuf, index: &Index) -> Result<impl IntoIterator<Item = PathBuf> + use<>> {
        let paths = self.paths.iter()
            .map(|path|calc_relative_path(&project_root, path))
            .collect::<Result<Vec<_>>>()?;

        let possible_dir = paths.iter().filter(|p|p.is_dir()).collect::<Vec<_>>();
        let possible_file = paths.iter().filter(|p|p.is_file()).collect::<Vec<_>>();

        if (!self.recursive) && (!possible_dir.is_empty()) {
            Err(GitError::not_a_repofile(possible_dir[0]))
        }
        else if let Some(path) = possible_file
            .iter()
            .filter(|p| !index.entries.iter().any(|en| en.name == p.to_str().unwrap()))
            .take(1).next()
        {
                    Err(GitError::not_a_repofile(path))
        }
        else {
            possible_dir
                .into_iter()
                .cloned()
                .map(walk)
                .collect::<Result<Vec<_>>>()
                .map(|x|x
                    .into_iter()
                    .flatten()
                    .filter(move|x| !x.starts_with(project_root.join(".git")))
                    .chain(possible_file.into_iter().cloned().collect::<Vec<_>>())
                )
        }
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

        let all_paths = self.walks_all_path(project_root.to_path_buf(), &index)?;
        if self.cached {
            all_paths.into_iter()
            .for_each(|path| {
                if let Some((idx, _)) = index.entries
                    .iter()
                    .enumerate()
                    .find(|(_, en)|en.name == path.to_str().unwrap())
                {
                    index.entries.remove(idx);
                };
            });
        }
        else {
            let mut removed_file = vec![];
            all_paths.into_iter()
            .for_each(|path| {
                if let Some((idx, _)) = index.entries
                    .iter()
                    .enumerate()
                    .find(|(_, en)|en.name == path.to_str().unwrap())
                {
                    let path = project_root.join(index.entries[idx].name.clone());
                    let result = remove_file(&path)
                        .map_err(move|e|GitError::failed_to_remove_file(format!("unable to remove file {} due to {}", path.display(), e.to_string())));
                    removed_file.push(result);
                    index.entries.remove(idx);
                };
            });
            removed_file.into_iter()
                .collect::<Result<Vec<_>>>()?;
        }
        index.write_to_file(&index_file)?;
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
    fn test_rm_dir() {
        let temp1 = setup_test_git_dir();
        let temp_path1 = temp1.path();
        let temp_path_str1 = temp_path1.to_str().unwrap();

        let temp2 = tempdir().unwrap();
        let temp_path2 = temp2.path();
        let temp_path_str2 = temp_path2.to_str().unwrap();

        let _ = mktemp_in(temp_path1.join("inner")).unwrap();
        // let file1_str = file1.file_name().unwrap();
        // let file1_str = file1_str.to_str().unwrap();

        let _ = mktemp_in(temp_path1.join("inner").join("close")).unwrap();
        // let file2_str = file2.file_name().unwrap().to_str();
        // let file2_str = file2_str.unwrap();

        let _ = cp_dir(temp_path1, temp_path2).unwrap();

        let file2_path = PathBuf::from("inner/close");
        let file1_path = PathBuf::from("inner");

        let cmds: ArgsList = &[
            (&["add", file1_path.to_str().unwrap(), file2_path.to_str().unwrap()], true),
            (&["rm", "--cached", "-r", file1_path.to_str().unwrap(), file2_path.to_str().unwrap()], true),
        ];
        let git = &["git", "-C", temp_path_str1];
        let cargo = &["cargo", "run", "--quiet", "--", "-C", temp_path_str2];
        let _ = run_both(cmds, git, cargo).unwrap();

        println!("{}", shell_spawn(&["ls", "-lahR", temp_path_str1]).unwrap());

        let origin = shell_spawn(&["git", "-C", temp_path_str1, "ls-files", "--stage"]).unwrap();
        let real = shell_spawn(&["git", "-C", temp_path_str2, "ls-files", "--stage"]).unwrap();
        assert_eq!(
            real.split("\n")
                .into_iter()
                .sorted()
                .collect::<String>(),
            origin.split("\n")
                .into_iter()
                .sorted()
                .collect::<String>()
        );
    }

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

        let origin = shell_spawn(&["git", "-C", temp_path_str1, "ls-files", "--stage"]).unwrap();
        let real = shell_spawn(&["git", "-C", temp_path_str2, "ls-files", "--stage"]).unwrap();
        assert_eq!(
            real.split("\n")
                .into_iter()
                .sorted()
                .collect::<String>(),
            origin.split("\n")
                .into_iter()
                .sorted()
                .collect::<String>()
        );
    }
}
