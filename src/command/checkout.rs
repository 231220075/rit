use std::path::{
    PathBuf,
    Path,
};
use clap::{Parser, Subcommand};
use crate::{
    GitError,
    Result,
    utils::refs::{read_head_ref, write_head_ref, read_ref_commit, write_ref_commit},
};
use super::SubCommand;
use std::fs;
use crate::utils::zlib::decompress_file_bytes;

#[derive(Parser, Debug)]
#[command(name = "checkout", about = "切换分支")]
pub struct Checkout {
    #[arg(short = 'b', long = "branch", help = "创建新分支")]
    create_new_branch: bool,

    #[arg(required = true, help = "分支名")]
    branch_name: String,
}

impl Checkout {
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        Ok(Box::new(Checkout::try_parse_from(args)?))
    }

    pub fn restore_workspace(gitdir: &Path, commit_hash: &str) -> Result<()> {
        let commit_path = gitdir.join("objects").join(&commit_hash[0..2]).join(&commit_hash[2..]);
        let decompressed = decompress_file_bytes(&commit_path)?;
        if let Some(tree_hash) = Checkout::extract_tree_hash(&decompressed) {
            println!("tree_hash: {}", tree_hash);
            // let tree_path = gitdir.join("objects").join(&tree_hash[0..2]).join(&tree_hash[2..]);
            // let tree_data = decompress_file_bytes(&tree_path)?;
            // let tree: Tree = bincode::deserialize(&tree_data).map_err(|_| GitError::failed_to_deserialize())?;
            // for entry in tree.entries {
            //     let file_path = gitdir.join(entry.name);
            //     match entry.mode {
            //         FileMode::Blob => {
            //             fs::write(file_path, &entry.data).map_err(|_| GitError::failed_to_write_file(&file_path.to_string_lossy()))?;
            //         }
            //         FileMode::Tree => {
            //             fs::create_dir_all(file_path).map_err(|_| GitError::failed_to_create_dir(&file_path.to_string_lossy()))?;
            //         }
            //     }
            // }
        }
        else {
            return Err(GitError::invalid_command(format!("commit {} does not contain a tree", commit_hash)));
        }
        Ok(())
    }

    fn extract_tree_hash(data: &[u8]) -> Option<String> {
        // 将字节数据转换为字符串
        let mut content = String::from_utf8_lossy(data).to_string();
        content = content.replace("tree ", "\ntree ");
        // 按行分割数据
        // println!("content: {}", content);
        for line in content.lines() {
            // 检查是否以 "tree " 开头
            if let Some(hash) = line.strip_prefix("tree ") {
                return Some(hash.to_string()); // 提取 tree_hash
            }
        }

        None // 如果没有找到 "tree "，返回 None
    }
}

impl SubCommand for Checkout {
    fn run(&self, gitdir: Result<PathBuf>) -> Result<i32> {
        let gitdir = gitdir?;
        let heads_dir = gitdir.join("refs/heads");
        let branch_path = heads_dir.join(&self.branch_name);
        if self.create_new_branch {
            if branch_path.exists() {
                return Err(GitError::invalid_command(format!("branch '{}' already exists", self.branch_name)));
            }
            let head_ref = read_head_ref(&gitdir)?;
            let commit_hash = read_ref_commit(&gitdir, &head_ref)?;
            fs::write(&branch_path, format!("{}\n", commit_hash))
                .map_err(|_| GitError::failed_to_write_file(&branch_path.to_string_lossy()))?;
            println!("Created new branch '{}'", self.branch_name);
            Ok(0)
        }else {
            if !branch_path.exists() {
                return Err(GitError::invalid_command(format!("branch '{}' does not exist", self.branch_name)));
            }
            let current_ref = read_head_ref(&gitdir)?;
            if format!("refs/heads/{}", &self.branch_name) == current_ref {
                return Err(GitError::invalid_command(format!("already on branch '{}'", self.branch_name)));
            }
            let commit_hash = read_ref_commit(&gitdir, &branch_path.to_string_lossy())?;
            write_head_ref(&gitdir, &format!("refs/heads/{}", self.branch_name))?;
            Checkout::restore_workspace(&gitdir, &commit_hash)?;
            println!("Switched to branch '{}'", self.branch_name);
            Ok(0)
        }
    }
}
