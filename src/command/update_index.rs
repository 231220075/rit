use std::path::{PathBuf,Path};
use clap::{Parser, Subcommand};
use crate::cli::command;
use crate::{
    GitError,
    Result,
};
use crate::utils::{
    fs::{
        read_file_as_bytes,
        get_git_dir,
    },
    hash::hash_object,
    index::{Index, IndexEntry},
    objtype::Blob,
};
use super::SubCommand;

#[derive(Parser, Debug)]
#[command(name = "update-index", about = "update the index file")]
pub struct UpdateIndex {
    #[arg(long, help = "Add file contents to the index")]
    add: bool,

    #[arg(long, help = "Remove file from the index")]
    rm: bool,

    #[arg(long, num_args = 3, help = "Sepcify file mode, hash and name")]
    cacheinfo: Option<Vec<String>>,

    #[arg(help = "Path to the file")]
    name: Option<String>, 

    //#[arg(skip)]
    //gitdir: PathBuf,
}

impl UpdateIndex {
    pub fn from_args(args: impl Iterator<Item = String>)
-> Result<Box<dyn SubCommand>>{
        let update_index = UpdateIndex::try_parse_from(args)?;
        //update_index.gitdir = gitdir;
        Ok(Box::new(update_index))
    }
}


impl SubCommand for UpdateIndex {
    fn run(&self, gitdir: Result<PathBuf>) -> Result<i32> {
        let mut index_path = gitdir?;
        index_path.push("index");
        let mut index = Index::new();

        if index_path.exists() {
            index = index.read_from_file(&index_path)?;
        }
        if let Some(cacheinfo) = &self.cacheinfo {
            if cacheinfo.len() != 3 {
                return Err(Box::new(GitError::InvalidCommand("cacheinfo".to_string())));
            }
            let mode = u32::from_str_radix(&cacheinfo[0], 8).map_err(|_| {
                GitError::InvalidCommand("Invalid file mode".to_string())
            })?;
            let hash = cacheinfo[1].clone();
            let name = cacheinfo[2].clone();

            let entry = IndexEntry::new(mode, hash, name);
            index.add_entry(entry);
        }
        else if self.add {
            if let Some(name) = &self.name {
                let current_dir = std::env::current_dir()?;
                let file_path = current_dir.join(name);
                if !file_path.exists() {
                    return Err(Box::new(GitError::FileNotFound(name.clone())));
                }
                let bytes = read_file_as_bytes(&file_path)?;
                let hash = hash_object::<Blob>(bytes)?;
                let mode = 0o100644;
                let entry = IndexEntry::new(mode, hash, name.clone());
                index.add_entry(entry);
            } else {
                return Err(Box::new(GitError::InvalidCommand(
                    "File name is required when using --add".to_string(),
                )));
            }
        }
        else if self.rm {
            if let Some(name) = &self.name {
                if !index.remove_entry(name) {
                    return Err(Box::new(GitError::FileNotFound(name.clone())));
                }
            } else {
                return Err(Box::new(GitError::InvalidCommand(
                    "File name is required when using --rm".to_string(),
                )));
            }
        } else {
            return Err(Box::new(GitError::InvalidCommand(
                "Invalid command: either --add, --rm, or --cacheinfo must be specified".to_string(),
            )));
        }

        index.write_to_file(&index_path)?;
        Ok(0)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    /// 设置测试环境，创建临时 `.git/index` 文件夹
    fn setup_test_git_dir() -> tempfile::TempDir {
        let temp_dir = tempdir().unwrap();
        let git_dir = temp_dir.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        temp_dir
    }

    #[test]
    fn test_update_index_with_cacheinfo() {
        let temp_dir = setup_test_git_dir();
        let index_path = temp_dir.path().join(".git").join("index");

        // 设置当前工作目录
        std::env::set_current_dir(&temp_dir).unwrap();

        // 模拟命令行参数
        let args = vec![
            "update-index".to_string(),
            "--cacheinfo".to_string(),
            "100644".to_string(),
            "d8329fc1cc938780ffdd9f94e0d364e0ea74f579".to_string(),
            "test.txt".to_string(),
        ];

        let update_index = UpdateIndex::try_parse_from(args).unwrap();
        let result = update_index.run(get_git_dir());

        // 验证运行结果
        assert!(result.is_ok());

        // 验证索引文件是否写入
        println!("Index path: {:?}", index_path); // 调试输出
        println!("courrent dir: {:?}", std::env::current_dir()); // 调试输出
        println!("Index path exists: {:?}", index_path.exists()); // 调试输出
        assert!(index_path.exists());
        let index_content = fs::read(&index_path).unwrap();
        assert!(!index_content.is_empty());
    }

    #[test]
    fn test_update_index_with_add() {
        let temp_dir = setup_test_git_dir();
        let index_path = temp_dir.path().join(".git").join("index");
        let test_file_path = temp_dir.path().join("test.txt");

        // 创建测试文件
        fs::write(&test_file_path, b"Hello, world!").unwrap();

        // 设置当前工作目录
        std::env::set_current_dir(&temp_dir).unwrap();

        // 模拟命令行参数
        let args = vec![
            "update-index".to_string(),
            "--add".to_string(),
            "test.txt".to_string(),
        ];

        let update_index = UpdateIndex::try_parse_from(args).unwrap();
        println!("{:?}", temp_dir.path().join(".git"));
        let result = update_index.run(Ok(temp_dir.path().join(".git")));

        // 验证运行结果
        assert!(result.is_ok());

        // 验证索引文件是否写入
        assert!(index_path.exists());
        let index_content = fs::read(&index_path).unwrap();
        assert!(!index_content.is_empty());
    }

    #[test]
    fn test_update_index_missing_file() {
        let temp_dir = setup_test_git_dir();

        // 设置当前工作目录
        std::env::set_current_dir(&temp_dir).unwrap();

        // 模拟命令行参数
        let args = vec![
            "update-index".to_string(),
            "--add".to_string(),
            "nonexistent.txt".to_string(),
        ];

        let update_index = UpdateIndex::try_parse_from(args).unwrap();
        let result = update_index.run(get_git_dir());

        // 验证运行结果
        assert!(result.is_err());
        if let Err(err) = result {
            assert_eq!(
                err.to_string(),
                "File not found: nonexistent.txt"
            );
        }
    }
}
