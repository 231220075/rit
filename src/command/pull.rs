use std::path::PathBuf;
use clap::Parser;
use crate::{GitError, Result};
use crate::utils::refs::{read_head_ref, head_to_hash};
use super::{SubCommand, Fetch, Merge, Checkout, ReadTree};

#[derive(Parser, Debug)]
#[command(name = "pull", about = "从远程仓库拉取并合并到当前分支")]
pub struct Pull {
    /// 远程仓库名称（默认为origin）
    #[arg(default_value = "origin")]
    remote: String,
    
    /// 远程分支名称（可选，默认为当前分支对应的远程分支）
    branch: Option<String>,
    
    /// 特定的引用规范（可选）
    refspecs: Vec<String>,
    
    /// 显示详细信息
    #[arg(short, long)]
    verbose: bool,
    
    /// 使用 rebase 而不是 merge
    #[arg(long)]
    rebase: bool,
}

impl Pull {
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        Ok(Box::new(Pull::try_parse_from(args)?))
    }
    
    /// 获取当前分支名称
    fn get_current_branch(&self, gitdir: &PathBuf) -> Result<String> {
        let head_ref = read_head_ref(gitdir)?;
        if head_ref.starts_with("refs/heads/") {
            Ok(head_ref.strip_prefix("refs/heads/").unwrap().to_string())
        } else {
            Err(GitError::invalid_command("Not on a branch (detached HEAD)".to_string()))
        }
    }
    
    /// 获取要拉取的远程分支名称
    fn get_remote_branch(&self, gitdir: &PathBuf) -> Result<String> {
        if let Some(ref branch) = self.branch {
            Ok(branch.clone())
        } else {
            // 使用当前分支对应的远程分支
            let current_branch = self.get_current_branch(gitdir)?;
            Ok(current_branch)
        }
    }
    
    /// 检查远程分支是否存在
    fn check_remote_branch_exists(&self, gitdir: &PathBuf, remote_branch: &str) -> Result<bool> {
        let remote_ref_path = gitdir
            .join("refs")
            .join("remotes")
            .join(&self.remote)
            .join(remote_branch);
        Ok(remote_ref_path.exists())
    }
}

impl SubCommand for Pull {
    fn run(&self, gitdir: Result<PathBuf>) -> Result<i32> {
        let gitdir = gitdir?;
        
        println!("Pulling from {}", self.remote);
        
        // 步骤1: 先执行 fetch
        if self.verbose {
            println!("Step 1: Fetching from remote...");
        }
        
        // 构造 fetch 命令参数
        let mut fetch_args = vec!["fetch".to_string(), self.remote.clone()];
        fetch_args.extend(self.refspecs.clone());
        if self.verbose {
            fetch_args.push("-v".to_string());
        }
        
        let fetch_cmd = Fetch::from_args(fetch_args.into_iter())?;
        let fetch_result = fetch_cmd.run(Ok(gitdir.clone()))?;
        if fetch_result != 0 {
            return Err(GitError::invalid_command("Fetch failed".to_string()));
        }
        
        // 步骤2: 确定要合并的分支
        let remote_branch = self.get_remote_branch(&gitdir)?;
        let remote_ref_name = format!("{}/{}", self.remote, remote_branch);
        
        if self.verbose {
            println!("Step 2: Checking remote branch {}...", remote_ref_name);
        }
        
        // 检查远程分支是否存在
        if !self.check_remote_branch_exists(&gitdir, &remote_branch)? {
            return Err(GitError::invalid_command(format!(
                "Remote branch '{}' not found", 
                remote_ref_name
            )));
        }
        
        // 步骤3: 检查当前分支是否存在
        let current_branch = self.get_current_branch(&gitdir)?;
        let current_branch_exists = self.check_current_branch_exists(&gitdir, &current_branch)?;
        
        if self.verbose {
            println!("Current branch: {}", current_branch);
            println!("Remote branch: {}", remote_ref_name);
            if !current_branch_exists {
                println!("Local branch '{}' does not exist, will be created from remote", current_branch);
            }
        }

        // 步骤4: 根据本地分支是否存在选择不同的处理方式
        if !current_branch_exists {
            // 本地分支不存在，创建分支并导入远程内容
            if self.verbose {
                println!("Step 3: Creating local branch from remote...");
            }
            self.create_branch_from_remote(&gitdir, &current_branch, &remote_ref_name)?;
        } else {
            // 本地分支存在，检查是否有本地修改
            let has_local_changes = self.check_local_changes(&gitdir)?;
            if has_local_changes {
                println!("Warning: You have local changes. Please commit or stash them before pulling.");
            }

            // 执行合并或rebase
            if self.rebase {
                if self.verbose {
                    println!("Step 3: Rebasing onto {}...", remote_ref_name);
                }
                // TODO: 实现 rebase 功能
                println!("Rebase not implemented yet, falling back to merge");
            }
            
            if self.verbose {
                println!("Step 3: Merging {}...", remote_ref_name);
            }
            
            // 构造 merge 命令参数 - 使用完整的远程引用路径
            let remote_ref_path = format!("refs/remotes/{}", remote_ref_name);
            let merge_args = vec!["merge".to_string(), remote_ref_path];
            let merge_cmd = Merge::from_args(merge_args.into_iter())?;
            
            let merge_result = merge_cmd.run(Ok(gitdir.clone()))?;
            if merge_result != 0 {
                return Err(GitError::invalid_command("Merge failed".to_string()));
            }
        }
        
        println!("Successfully pulled from {}/{}", self.remote, remote_branch);
        
        Ok(0)
    }
}

impl Pull {
    /// 简单检查是否有本地修改（这里简化实现）
    fn check_local_changes(&self, gitdir: &PathBuf) -> Result<bool> {
        // 检查index文件是否存在且非空
        let index_path = gitdir.join("index");
        if index_path.exists() {
            let metadata = std::fs::metadata(&index_path)?;
            Ok(metadata.len() > 0)
        } else {
            Ok(false)
        }
    }
    
    /// 检查当前分支是否存在
    fn check_current_branch_exists(&self, gitdir: &PathBuf, branch_name: &str) -> Result<bool> {
        let branch_path = gitdir.join("refs").join("heads").join(branch_name);
        Ok(branch_path.exists())
    }
    
    /// 从远程分支创建本地分支
    fn create_branch_from_remote(&self, gitdir: &PathBuf, local_branch: &str, remote_ref_name: &str) -> Result<()> {
        use crate::utils::refs::{read_ref_commit, write_head_ref};
        use std::fs;
        
        // 读取远程分支的提交哈希
        let remote_ref_path = format!("refs/remotes/{}", remote_ref_name);
        let commit_hash = read_ref_commit(gitdir, &remote_ref_path)?;
        
        if self.verbose {
            println!("Creating local branch '{}' from commit {}", local_branch, &commit_hash[..8]);
        }
        
        // 创建本地分支引用
        let local_branch_path = gitdir.join("refs").join("heads").join(local_branch);
        if let Some(parent) = local_branch_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&local_branch_path, format!("{}\n", commit_hash))?;
        
        // 更新 HEAD 指向新分支
        let local_branch_ref = format!("refs/heads/{}", local_branch);
        write_head_ref(gitdir, &local_branch_ref)?;
        
        // 检查并创建 index 文件，然后导入远程 tree
        self.setup_index_and_workspace(gitdir, &commit_hash)?;
        
        if self.verbose {
            println!("Successfully created and switched to branch '{}'", local_branch);
        }
        
        Ok(())
    }
    
    /// 设置 index 和工作区
    fn setup_index_and_workspace(&self, gitdir: &PathBuf, commit_hash: &str) -> Result<()> {
        use crate::utils::index::Index;
        
        let index_path = gitdir.join("index");
        
        // 检查 index 是否存在
        if !index_path.exists() {
            if self.verbose {
                println!("Creating index file...");
            }
            // 创建空的 index 文件
            let index = Index::new();
            index.write_to_file(&index_path)?;
        }
        
        // 从提交中获取 tree 并导入到 index 和工作区
        self.import_tree_to_index_and_workspace(gitdir, commit_hash)?;
        
        Ok(())
    }
    
    /// 将 tree 导入到 index 和工作区
    fn import_tree_to_index_and_workspace(&self, gitdir: &PathBuf, commit_hash: &str) -> Result<()> {
        use crate::command::{Checkout, ReadTree};
        
        if self.verbose {
            println!("Importing tree from commit to index and workspace...");
        }
        
        // 恢复工作区到该提交
        Checkout::restore_workspace(gitdir, commit_hash)?;
        
        // 更新 index
        let tree_hash = self.get_tree_hash_from_commit(gitdir, commit_hash)?;
        let read_tree = ReadTree {
            prefix: None,
            tree_hash,
        };
        read_tree.run(Ok(gitdir.clone()))?;
        
        if self.verbose {
            println!("Successfully imported tree to index and workspace");
        }
        
        Ok(())
    }
    
    /// 从提交中获取 tree 哈希
    fn get_tree_hash_from_commit(&self, gitdir: &PathBuf, commit_hash: &str) -> Result<String> {
        use crate::utils::zlib::decompress_file_bytes;
        use crate::utils::fs::obj_to_pathbuf;
        
        let commit_path = obj_to_pathbuf(gitdir, commit_hash);
        let decompressed = decompress_file_bytes(&commit_path)?;
        
        // 解析提交内容获取 tree 哈希
        let content = String::from_utf8_lossy(&decompressed);
        
        // 查找 "tree " 在整个内容中的位置
        if let Some(tree_start) = content.find("tree ") {
            let tree_line = &content[tree_start..];
            if let Some(tree_hash) = tree_line.strip_prefix("tree ") {
                // 提取 tree 哈希（40个字符）
                if tree_hash.len() >= 40 {
                    return Ok(tree_hash[..40].to_string());
                }
            }
        }
        
        // 如果上面的方法失败，尝试按行解析
        for line in content.lines() {
            if self.verbose {
                println!("DEBUG: Commit {} line: {}", commit_hash, line);
            }
            if let Some(tree_hash) = line.strip_prefix("tree ") {
                return Ok(tree_hash.to_string());
            }
        }
        
        Err(GitError::invalid_command(format!("No tree found in commit {}", commit_hash)))
    }
}
