use std::path::PathBuf;
use clap::Parser;
use std::collections::HashMap;
use crate::{GitError, Result, utils::refs::*};
use crate::utils::protocol::GitProtocol;
use crate::utils::packfile::PackfileProcessor;
use super::SubCommand;

#[derive(Parser, Debug)]
#[command(name = "fetch", about = "从远程仓库下载对象和引用")]
pub struct Fetch {
    /// 远程仓库名称（默认为origin）
    #[arg(default_value = "origin")]
    remote: String,
    
    /// 特定的引用规范（可选）
    refspecs: Vec<String>,
    
    /// 显示详细信息
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug)]
pub struct FetchResult {
    pub updated_refs: HashMap<String, String>,
    pub new_refs: HashMap<String, String>,
    pub deleted_refs: Vec<String>,
}

impl Fetch {
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        Ok(Box::new(Fetch::try_parse_from(args)?))
    }
    
    /// 模拟fetch操作 - 用于测试和开发
    fn simulate_fetch(&self, gitdir: &PathBuf) -> Result<FetchResult> {
        // 检查远程配置而不是目录
        let _config = self.read_remote_config(gitdir)?;
        
        println!("From {}", self.remote);
        
        let updated_refs = HashMap::new();
        let mut new_refs = HashMap::new();
        
        // 模拟创建一个远程跟踪分支
        let remote_ref_path = gitdir.join("refs").join("remotes").join(&self.remote);
        if let Some(parent) = remote_ref_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // 模拟分支引用
        let ref_name = format!("refs/remotes/{}/main", self.remote);
        let fake_commit = "0000000000000000000000000000000000000000";
        
        new_refs.insert(ref_name, fake_commit.to_string());
        println!(" * [simulated]       main -> {}/main", self.remote);
        
        Ok(FetchResult {
            updated_refs,
            new_refs,
            deleted_refs: vec![],
        })
    }
    
    fn read_remote_config(&self, gitdir: &PathBuf) -> Result<RemoteConfig> {
        let config_path = gitdir.join("config");
        let config_content = std::fs::read_to_string(config_path)?;
        
        // 简单的config解析
        let mut url = None;
        let mut fetch_specs = Vec::new();
        let mut in_remote_section = false;
        
        for line in config_content.lines() {
            let line = line.trim();
            
            if line == &format!("[remote \"{}\"]", self.remote) {
                in_remote_section = true;
                continue;
            }
            
            if line.starts_with('[') && line.ends_with(']') {
                in_remote_section = false;
                continue;
            }
            
            if in_remote_section {
                if let Some(url_value) = line.strip_prefix("url = ") {
                    url = Some(url_value.to_string());
                } else if let Some(fetch_value) = line.strip_prefix("fetch = ") {
                    fetch_specs.push(fetch_value.to_string());
                }
            }
        }
        
        Ok(RemoteConfig {
            name: self.remote.clone(),
            url: url.ok_or_else(|| GitError::invalid_command(
                format!("No URL found for remote '{}'", self.remote)
            ))?,
            fetch_specs,
        })
    }
    
    fn fetch_from_remote(&self, gitdir: &PathBuf) -> Result<FetchResult> {
        let config = self.read_remote_config(gitdir)?;
        
        if self.verbose {
            println!("Fetching from {}", config.url);
        }
        
        // 对于HTTP(S) URL，可以使用git的smart protocol
        if config.url.starts_with("http") {
            self.fetch_via_http(gitdir, &config)
        } else if config.url.starts_with("git@") || config.url.contains("ssh://") {
            self.fetch_via_ssh(gitdir, &config)
        } else {
            // 本地路径或其他协议
            self.fetch_via_local(gitdir, &config)
        }
    }
    
    fn fetch_via_http(&self, gitdir: &PathBuf, config: &RemoteConfig) -> Result<FetchResult> {
        println!("Fetching via HTTP from {}...", config.url);
        
        let protocol = GitProtocol::new()?;
        
        // 确定要获取的引用
        let wanted_refs = if self.refspecs.is_empty() {
            // 默认获取所有远程分支
            vec![]
        } else {
            self.refspecs.clone()
        };
        
        // 从远程获取数据
        let packfile_data = protocol.fetch_via_http(&config.url, &wanted_refs)?;
        
        if packfile_data.data.is_empty() {
            println!("Already up to date");
            return Ok(FetchResult {
                updated_refs: HashMap::new(),
                new_refs: HashMap::new(),
                deleted_refs: vec![],
            });
        }
        
        // 处理packfile
        let mut processor = PackfileProcessor::new(gitdir.clone());
        let created_objects = processor.process_packfile(&packfile_data.data)?;
        
        if self.verbose {
            println!("Received {} objects", created_objects.len());
        }
        
        // 更新远程跟踪分支
        let mut updated_refs = HashMap::new();
        let mut new_refs = HashMap::new();
        
        for remote_ref in &packfile_data.refs {
            if remote_ref.name.starts_with("refs/heads/") {
                let branch_name = remote_ref.name.strip_prefix("refs/heads/").unwrap();
                let local_remote_ref_path = gitdir
                    .join("refs")
                    .join("remotes")
                    .join(&self.remote)
                    .join(branch_name);
                
                // 创建目录
                if let Some(parent) = local_remote_ref_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                
                let ref_name = format!("refs/remotes/{}/{}", self.remote, branch_name);
                
                if local_remote_ref_path.exists() {
                    let old_commit = std::fs::read_to_string(&local_remote_ref_path)?.trim().to_string();
                    if old_commit != remote_ref.hash {
                        updated_refs.insert(ref_name, remote_ref.hash.clone());
                        println!("   {}..{}  {}", &old_commit[..8], &remote_ref.hash[..8], branch_name);
                    }
                } else {
                    new_refs.insert(ref_name, remote_ref.hash.clone());
                    println!(" * [new branch]      {} -> {}/{}", branch_name, self.remote, branch_name);
                }
                
                // 写入引用
                std::fs::write(&local_remote_ref_path, format!("{}\n", remote_ref.hash))?;
            }
        }
        
        // 写入FETCH_HEAD
        let all_refs: HashMap<String, String> = updated_refs.iter()
            .chain(new_refs.iter())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        self.write_fetch_head(gitdir, &all_refs)?;
        
        Ok(FetchResult {
            updated_refs,
            new_refs,
            deleted_refs: vec![],
        })
    }
    
    fn fetch_via_ssh(&self, gitdir: &PathBuf, config: &RemoteConfig) -> Result<FetchResult> {
        println!("SSH fetch not fully implemented yet");
        println!("Falling back to simulation for SSH URL: {}", config.url);
        self.simulate_fetch(gitdir)
    }
    
    fn fetch_via_local(&self, gitdir: &PathBuf, config: &RemoteConfig) -> Result<FetchResult> {
        // 本地路径fetch（对于开发测试很有用）
        let remote_gitdir = PathBuf::from(&config.url);
        if !remote_gitdir.exists() {
            return Err(GitError::invalid_command(
                format!("Remote path does not exist: {}", config.url)
            ));
        }
        
        // 从本地仓库复制对象和引用
        self.fetch_from_local_repo(gitdir, &remote_gitdir)
    }
    
    fn fetch_from_local_repo(&self, gitdir: &PathBuf, remote_gitdir: &PathBuf) -> Result<FetchResult> {
        let mut updated_refs = HashMap::new();
        let mut new_refs = HashMap::new();
        
        // 复制远程分支引用
        let remote_heads = remote_gitdir.join("refs").join("heads");
        if remote_heads.exists() {
            for entry in std::fs::read_dir(&remote_heads)? {
                let entry = entry?;
                let branch_name = entry.file_name().to_string_lossy().to_string();
                let remote_commit = std::fs::read_to_string(entry.path())?.trim().to_string();
                
                // 创建/更新本地的远程跟踪分支
                let local_remote_ref = gitdir.join("refs").join("remotes").join(&self.remote);
                std::fs::create_dir_all(&local_remote_ref)?;
                
                let local_remote_branch = local_remote_ref.join(&branch_name);
                let ref_name = format!("refs/remotes/{}/{}", self.remote, branch_name);
                
                if local_remote_branch.exists() {
                    let old_commit = std::fs::read_to_string(&local_remote_branch)?.trim().to_string();
                    if old_commit != remote_commit {
                        updated_refs.insert(ref_name.clone(), remote_commit.clone());
                        println!("   {}..{}  {}", &old_commit[..8], &remote_commit[..8], branch_name);
                    }
                } else {
                    new_refs.insert(ref_name.clone(), remote_commit.clone());
                    println!(" * [new branch]      {} -> {}/{}", branch_name, self.remote, branch_name);
                }
                
                std::fs::write(&local_remote_branch, format!("{}\n", remote_commit))?;
                
                // 递归复制所有依赖对象
                self.copy_object_recursive(gitdir, remote_gitdir, &remote_commit)?;
            }
        }
        
        // 写入FETCH_HEAD
        let all_refs = updated_refs.iter().chain(new_refs.iter())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        self.write_fetch_head(gitdir, &all_refs)?;
        
        Ok(FetchResult {
            updated_refs,
            new_refs,
            deleted_refs: vec![],
        })
    }
    
    fn copy_missing_objects(&self, gitdir: &PathBuf, remote_gitdir: &PathBuf, commit_hash: &str) -> Result<()> {
        self.copy_object_recursive(gitdir, remote_gitdir, commit_hash)?;
        Ok(())
    }

    fn copy_object_recursive(&self, gitdir: &PathBuf, remote_gitdir: &PathBuf, object_hash: &str) -> Result<()> {
        let obj_path = crate::utils::fs::obj_to_pathbuf(gitdir, object_hash);
        if obj_path.exists() {
            return Ok(()); // 对象已存在
        }
        
        let remote_obj_path = crate::utils::fs::obj_to_pathbuf(remote_gitdir, object_hash);
        if !remote_obj_path.exists() {
            return Err(GitError::invalid_command(
                format!("Object {} not found in remote repository", object_hash)
            ));
        }
        
        // 复制对象文件
        if let Some(parent) = obj_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&remote_obj_path, &obj_path)?;
        
        if self.verbose {
            println!("Copied object {}", object_hash);
        }
        
        // 解析对象内容，递归复制依赖的对象
        let obj_content = std::fs::read(&obj_path)?;
                let obj_data = crate::utils::zlib::decompress_object(&obj_content)?;        // 根据对象类型解析依赖
        if let Some(null_pos) = obj_data.iter().position(|&b| b == 0) {
            let header = String::from_utf8_lossy(&obj_data[..null_pos]);
            let content = &obj_data[null_pos + 1..];
            
            if header.starts_with("commit") {
                self.copy_commit_dependencies(gitdir, remote_gitdir, content)?;
            } else if header.starts_with("tree") {
                self.copy_tree_dependencies(gitdir, remote_gitdir, content)?;
            }
            // blob对象没有依赖
        }
        
        Ok(())
    }

    fn copy_commit_dependencies(&self, gitdir: &PathBuf, remote_gitdir: &PathBuf, content: &[u8]) -> Result<()> {
        let content_str = String::from_utf8_lossy(content);
        for line in content_str.lines() {
            if line.starts_with("tree ") {
                let tree_hash = &line[5..45];
                self.copy_object_recursive(gitdir, remote_gitdir, tree_hash)?;
            } else if line.starts_with("parent ") {
                let parent_hash = &line[7..47];
                self.copy_object_recursive(gitdir, remote_gitdir, parent_hash)?;
            }
        }
        Ok(())
    }

    fn copy_tree_dependencies(&self, gitdir: &PathBuf, remote_gitdir: &PathBuf, content: &[u8]) -> Result<()> {
        let mut pos = 0;
        while pos < content.len() {
            // 解析tree entry: mode name\0hash
            if let Some(null_pos) = content[pos..].iter().position(|&b| b == 0) {
                let entry_header = String::from_utf8_lossy(&content[pos..pos + null_pos]);
                if let Some(space_pos) = entry_header.find(' ') {
                    let _mode = &entry_header[..space_pos];
                    let _name = &entry_header[space_pos + 1..];
                    
                    // 提取20字节的hash
                    let hash_start = pos + null_pos + 1;
                    if hash_start + 20 <= content.len() {
                        let hash_bytes = &content[hash_start..hash_start + 20];
                        let hash = hex::encode(hash_bytes);
                        
                        // 递归复制依赖对象
                        self.copy_object_recursive(gitdir, remote_gitdir, &hash)?;
                        
                        pos = hash_start + 20;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    fn write_fetch_head(&self, gitdir: &PathBuf, refs: &HashMap<String, String>) -> Result<()> {
        let fetch_head_path = gitdir.join("FETCH_HEAD");
        let mut content = String::new();
        
        for (ref_name, commit_hash) in refs {
            if let Some(branch_name) = ref_name.strip_prefix(&format!("refs/remotes/{}/", self.remote)) {
                content.push_str(&format!("{}\t\tbranch '{}' of {}\n", 
                    commit_hash, 
                    branch_name, 
                    self.remote
                ));
            }
        }
        
        std::fs::write(fetch_head_path, content)?;
        Ok(())
    }
}

#[derive(Debug)]
struct RemoteConfig {
    name: String,
    url: String,
    fetch_specs: Vec<String>,
}

impl SubCommand for Fetch {
    fn run(&self, gitdir: Result<PathBuf>) -> Result<i32> {
        let gitdir = gitdir?;
        
        println!("Fetching from {}...", self.remote);
        
        let result = if std::env::var("GIT_FETCH_SIMULATE").is_ok() {
            // 开发模式：使用模拟fetch
            self.simulate_fetch(&gitdir)?
        } else {
            // 生产模式：尝试真实fetch
            self.fetch_from_remote(&gitdir)?
        };
        
        // 显示结果统计
        let total_updates = result.updated_refs.len() + result.new_refs.len();
        if total_updates > 0 {
            println!("Fetched {} reference(s)", total_updates);
        } else {
            println!("Already up to date");
        }
        
        Ok(0)
    }
}
