use std::path::PathBuf;
use clap::Parser;
use crate::{GitError, Result};
use crate::utils::refs::{read_head_ref, head_to_hash};
use super::{SubCommand, Fetch, Merge};

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
        
        // 步骤3: 检查当前状态
        let current_branch = self.get_current_branch(&gitdir)?;
        if self.verbose {
            println!("Current branch: {}", current_branch);
            println!("Remote branch: {}", remote_ref_name);
        }
        
        // 检查是否有本地修改（简化检查）
        let has_local_changes = self.check_local_changes(&gitdir)?;
        if has_local_changes {
            println!("Warning: You have local changes. Please commit or stash them before pulling.");
            // 在实际实现中，这里可能需要更严格的检查
        }
        
        // 步骤4: 执行合并或rebase
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
}
