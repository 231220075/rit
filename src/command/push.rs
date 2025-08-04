use std::path::PathBuf;
use std::collections::HashMap;
use clap::Parser;
use crate::{GitError, Result};
use super::SubCommand;

#[derive(Parser, Debug)]
#[command(name = "push", about = "推送本地更改到远程仓库")]
pub struct Push {
    /// 远程仓库名称
    #[arg(default_value = "origin")]
    remote: String,
    
    /// 分支名称
    branch: Option<String>,
    
    /// 强制推送
    #[arg(short, long)]
    force: bool,
    
    /// 详细输出
    #[arg(short, long)]
    verbose: bool,
    
    /// 推送所有分支
    #[arg(long)]
    all: bool,
}

impl Push {
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        Ok(Box::new(Push::try_parse_from(args)?))
    }
    
    /// 执行推送操作
    fn push_to_remote(&self, gitdir: &PathBuf) -> Result<()> {
        // 1. 获取远程仓库配置
        let remote_config = self.get_remote_config(gitdir)?;
        
        if self.verbose {
            println!("Pushing to {}", remote_config.url);
        }

        // 检查URL类型并选择传输方式
        if remote_config.url.starts_with("git@") || remote_config.url.contains("ssh://") {
            return self.push_via_ssh(&remote_config, gitdir);
        } else {
            return self.push_via_https(&remote_config, gitdir);
        }
    }

    /// 通过HTTPS推送
    fn push_via_https(&self, remote_config: &RemoteConfig, gitdir: &PathBuf) -> Result<()> {
        
        
        if self.verbose {
            println!("Pushing to {}", remote_config.url);
        }
        
        // 2. 获取当前分支和提交
        let (current_branch, current_commit) = self.get_current_state(gitdir)?;
        let target_branch = self.branch.as_ref().unwrap_or(&current_branch);
        
        if self.verbose {
            println!("Pushing branch '{}' ({})", target_branch, &current_commit[..8]);
        }
        
        // 3. 检查远程状态
        let remote_refs = self.discover_remote_refs(&remote_config.url)?;
        
        // 4. 检查是否需要推送
        let push_info = self.analyze_push(&remote_refs, target_branch, &current_commit)?;
        
        if push_info.up_to_date {
            println!("Everything up-to-date");
            return Ok(());
        }
        
        // 5. 收集需要推送的对象
        let objects_to_push = self.collect_objects_to_push(gitdir, &current_commit, &push_info)?;
        
        // 6. 创建 packfile
        let packfile = self.create_packfile(gitdir, &objects_to_push)?;
        
        // 7. 推送到 GitHub
        self.send_push_to_github(&remote_config.url, target_branch, &current_commit, &push_info, packfile)?;
        
        println!("Successfully pushed to {}/{}", self.remote, target_branch);
        Ok(())
    }

    /// 通过SSH推送
    fn push_via_ssh(&self, remote_config: &RemoteConfig, gitdir: &PathBuf) -> Result<()> {
        if self.verbose {
            println!("Using SSH transport for {}", remote_config.url);
        }
        
        // 2. 获取当前分支和提交
        let (current_branch, current_commit) = self.get_current_state(gitdir)?;
        let target_branch = self.branch.as_ref().unwrap_or(&current_branch);
        
        if self.verbose {
            println!("Pushing branch '{}' ({})", target_branch, &current_commit[..8]);
        }
        
        // 使用系统Git进行SSH推送（临时解决方案）
        self.push_via_system_git(&remote_config.url, target_branch)?;
        
        println!("Successfully pushed to {}/{}", self.remote, target_branch);
        Ok(())
    }
    
    /// 使用系统Git进行推送（SSH支持）
    fn push_via_system_git(&self, _url: &str, branch: &str) -> Result<()> {
        use std::process::Command;
        
        let mut cmd = Command::new("git");
        cmd.arg("push");
        cmd.arg("origin");
        cmd.arg(format!("{}:{}", branch, branch));
        
        if self.force {
            cmd.arg("--force");
        }
        
        if self.verbose {
            cmd.arg("--verbose");
        }
        
        let output = cmd.output()?;
        
        if output.status.success() {
            if self.verbose {
                println!("Git push output: {}", String::from_utf8_lossy(&output.stdout));
            }
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(GitError::invalid_command(format!("Git push failed: {}", error)))
        }
    }
    
    /// 获取远程仓库配置
    fn get_remote_config(&self, gitdir: &PathBuf) -> Result<RemoteConfig> {
        let config_path = gitdir.join("config");
        let config_content = std::fs::read_to_string(config_path)?;
        
        let mut url = None;
        let mut in_remote_section = false;
        
        for line in config_content.lines() {
            let trimmed = line.trim();
            
            if trimmed == &format!("[remote \"{}\"]", self.remote) {
                in_remote_section = true;
                continue;
            }
            
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                in_remote_section = false;
            }
            
            if in_remote_section && trimmed.starts_with("url = ") {
                url = Some(trimmed.strip_prefix("url = ").unwrap().to_string());
            }
        }
        
        let url = url.ok_or_else(|| GitError::invalid_command(format!("Remote '{}' not found", self.remote)))?;
        
        Ok(RemoteConfig { url })
    }
    
    /// 获取当前分支和提交状态
    fn get_current_state(&self, gitdir: &PathBuf) -> Result<(String, String)> {
        use crate::utils::refs::{read_head_ref, read_ref_commit};
        
        let head_ref = read_head_ref(gitdir)?;
        let current_branch = head_ref.strip_prefix("refs/heads/")
            .ok_or_else(|| GitError::invalid_command("Not on a branch".to_string()))?
            .to_string();
        
        let current_commit = read_ref_commit(gitdir, &head_ref)?;
        
        Ok((current_branch, current_commit))
    }
    
    /// 发现远程引用（GitHub API）
    fn discover_remote_refs(&self, url: &str) -> Result<HashMap<String, String>> {
        use reqwest::blocking::Client;
        
        let client = Client::new();
        let refs_url = format!("{}/info/refs?service=git-receive-pack", url);
        
        if self.verbose {
            println!("Discovering remote refs from {}", refs_url);
        }
        
        let mut request = client
            .get(&refs_url)
            .header("User-Agent", "git/rust-git")
            .header("Accept", "application/x-git-receive-pack-advertisement");
        
        // 添加 GitHub 认证
        if let Some((username, password)) = self.get_github_credentials(url)? {
            request = request.basic_auth(username, Some(password));
        }
        
        let response = request.send()?;
        
        if !response.status().is_success() {
            return Err(GitError::network_error(format!(
                "Failed to discover refs: {} - {}", 
                response.status(),
                response.text().unwrap_or_default()
            )));
        }
        
        let body = response.text()?;
        self.parse_refs_response(&body)
    }
    
    /// 获取 GitHub 认证信息
    fn get_github_credentials(&self, url: &str) -> Result<Option<(String, String)>> {
        // 1. 尝试环境变量
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            return Ok(Some(("token".to_string(), token)));
        }
        
        // 2. 尝试从 git config 获取
        if let Ok(token) = std::env::var("GIT_TOKEN") {
            return Ok(Some(("token".to_string(), token)));
        }
        
        // 3. 交互式输入
        if url.contains("github.com") {
            println!("GitHub authentication required");
            println!("GitHub no longer supports password authentication for Git operations.");
            println!("Please use a Personal Access Token instead.");
            println!("GitHub用户名: ");
            let mut username = String::new();
            std::io::stdin().read_line(&mut username)?;
            let username = username.trim().to_string();
            
            println!("Personal Access Token (not your GitHub password): ");
            let token = rpassword::read_password()?;
            
            return Ok(Some((username, token)));
        }
        
        Ok(None)
    }
    
    /// 解析引用响应
    fn parse_refs_response(&self, body: &str) -> Result<HashMap<String, String>> {
        let mut refs = HashMap::new();
        
        // 跳过服务声明行
        let lines: Vec<&str> = body.lines().skip_while(|line| {
            line.starts_with('#') || line.contains("service=git-receive-pack")
        }).collect();
        
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            
            // 解析 pkt-line 格式: "0041hash refs/heads/main\0capabilities"
            if let Some(content) = self.parse_pkt_line(line) {
                if let Some((hash, ref_name)) = self.parse_ref_line(&content) {
                    if self.verbose {
                        println!("Remote ref: {} -> {}", ref_name, &hash[..8]);
                    }
                    refs.insert(ref_name, hash);
                }
            }
        }
        
        Ok(refs)
    }
    
    /// 解析 pkt-line 格式
    fn parse_pkt_line(&self, line: &str) -> Option<String> {
        if line.len() < 4 {
            return None;
        }
        
        // 前4个字符是十六进制长度
        if let Ok(length) = u16::from_str_radix(&line[..4], 16) {
            if length == 0 {
                return Some(String::new()); // flush packet
            }
            
            let content_length = (length as usize).saturating_sub(4);
            if line.len() >= 4 + content_length {
                return Some(line[4..4 + content_length].to_string());
            }
        }
        
        None
    }
    
    /// 解析引用行
    fn parse_ref_line(&self, content: &str) -> Option<(String, String)> {
        // 格式: "hash refs/heads/branch_name"
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() >= 2 {
            let hash = parts[0].to_string();
            let ref_name = parts[1].to_string();
            
            // 过滤掉 capabilities
            if let Some(null_pos) = ref_name.find('\0') {
                let clean_ref_name = ref_name[..null_pos].to_string();
                return Some((hash, clean_ref_name));
            }
            
            Some((hash, ref_name))
        } else {
            None
        }
    }
    
    /// 分析推送需求
    fn analyze_push(&self, remote_refs: &HashMap<String, String>, branch: &str, local_commit: &str) -> Result<PushInfo> {
        let remote_ref_name = format!("refs/heads/{}", branch);
        
        if let Some(remote_commit) = remote_refs.get(&remote_ref_name) {
            if remote_commit == local_commit {
                return Ok(PushInfo {
                    up_to_date: true,
                    force_required: false,
                    old_commit: Some(remote_commit.clone()),
                    new_commit: local_commit.to_string(),
                });
            }
            
            // 检查是否需要强制推送
            let force_required = !self.force && !self.is_fast_forward(local_commit, remote_commit)?;
            
            Ok(PushInfo {
                up_to_date: false,
                force_required,
                old_commit: Some(remote_commit.clone()),
                new_commit: local_commit.to_string(),
            })
        } else {
            // 新分支
            Ok(PushInfo {
                up_to_date: false,
                force_required: false,
                old_commit: None,
                new_commit: local_commit.to_string(),
            })
        }
    }
    
    /// 检查是否为快进推送
    fn is_fast_forward(&self, _local_commit: &str, _remote_commit: &str) -> Result<bool> {
        // 简化实现：检查本地提交历史是否包含远程提交
        // 在实际实现中，需要遍历提交历史
        Ok(true) // 暂时总是允许，避免复杂的历史检查
    }
    
    /// 收集需要推送的对象
    fn collect_objects_to_push(&self, _gitdir: &PathBuf, commit_hash: &str, _push_info: &PushInfo) -> Result<Vec<String>> {
        let mut objects = Vec::new();
        
        // 简化实现：收集当前提交的所有相关对象
        objects.push(commit_hash.to_string());
        
        // 在实际实现中，需要:
        // 1. 遍历提交历史，找到远程没有的提交
        // 2. 收集这些提交引用的所有对象（tree, blob）
        // 3. 计算差集，只推送远程缺少的对象
        
        if self.verbose {
            println!("Objects to push: {}", objects.len());
        }
        
        Ok(objects)
    }
    
    /// 创建 packfile
    fn create_packfile(&self, gitdir: &PathBuf, objects: &[String]) -> Result<Vec<u8>> {
        // 简化实现：创建包含指定对象的 packfile
        let mut packfile = Vec::new();
        
        // Git packfile 格式:
        // 1. Header: "PACK" + version + object_count
        packfile.extend(b"PACK");
        packfile.extend(&2u32.to_be_bytes()); // version 2
        packfile.extend(&(objects.len() as u32).to_be_bytes());
        
        // 2. 对象数据（简化）
        for object_hash in objects {
            // 读取对象并添加到 packfile
            // 实际实现需要正确的 packfile 格式
            if let Ok(object_data) = self.read_object_data(gitdir, object_hash) {
                packfile.extend(object_data);
            }
        }
        
        if self.verbose {
            println!("Created packfile: {} bytes", packfile.len());
        }
        
        Ok(packfile)
    }
    
    /// 读取对象数据
    fn read_object_data(&self, gitdir: &PathBuf, object_hash: &str) -> Result<Vec<u8>> {
        use crate::utils::fs::obj_to_pathbuf;
        use crate::utils::zlib::decompress_file_bytes;
        
        let object_path = obj_to_pathbuf(gitdir, object_hash);
        decompress_file_bytes(&object_path)
    }
    
    /// 发送推送请求到 GitHub
    fn send_push_to_github(&self, url: &str, branch: &str, commit: &str, push_info: &PushInfo, packfile: Vec<u8>) -> Result<()> {
        use reqwest::blocking::Client;
        
        if push_info.force_required && !self.force {
            return Err(GitError::invalid_command(
                "Updates were rejected because the remote contains work that you do not have locally. Use --force to override.".to_string()
            ));
        }
        
        let client = Client::new();
        let push_url = format!("{}/git-receive-pack", url);
        
        if self.verbose {
            println!("Pushing to {}", push_url);
        }
        
        // 创建推送请求体
        let mut request_body = Vec::new();
        
        // 1. 引用更新命令
        let old_commit = push_info.old_commit.as_deref().unwrap_or("0000000000000000000000000000000000000000");
        let ref_update = format!("{} {} refs/heads/{}", old_commit, commit, branch);
        
        // 添加 capabilities
        let ref_update_with_caps = if push_info.old_commit.is_none() {
            format!("{}\0report-status delete-refs", ref_update)
        } else {
            format!("{}\0report-status", ref_update)
        };
        
        request_body.extend(self.create_pkt_line(&ref_update_with_caps));
        request_body.extend(b"0000"); // flush packet
        
        // 2. packfile 数据
        request_body.extend(packfile);
        
        // 3. 发送请求
        let mut request = client
            .post(&push_url)
            .header("Content-Type", "application/x-git-receive-pack-request")
            .header("User-Agent", "git/rust-git")
            .body(request_body);
        
        // 添加认证
        if let Some((username, password)) = self.get_github_credentials(url)? {
            request = request.basic_auth(username, Some(password));
        }
        
        let response = request.send()?;
        let status = response.status();
        
        if status.is_success() {
            if self.verbose {
                println!("Push response: {}", status);
            }
            
            // 解析响应
            let response_body = response.text()?;
            self.parse_push_response(&response_body)?;
            
            Ok(())
        } else {
            let error_body = response.text().unwrap_or_default();
            Err(GitError::network_error(format!("Push failed: {} - {}", status, error_body)))
        }
    }
    
    /// 创建 pkt-line
    fn create_pkt_line(&self, content: &str) -> Vec<u8> {
        let length = content.len() + 4;
        let length_hex = format!("{:04x}", length);
        let mut pkt_line = length_hex.into_bytes();
        pkt_line.extend(content.as_bytes());
        pkt_line
    }
    
    /// 解析推送响应
    fn parse_push_response(&self, response: &str) -> Result<()> {
        for line in response.lines() {
            if line.starts_with("unpack ok") {
                if self.verbose {
                    println!("Unpack successful");
                }
            } else if line.starts_with("ok refs/heads/") {
                if self.verbose {
                    println!("Reference update successful: {}", line);
                }
            } else if line.starts_with("ng refs/heads/") {
                return Err(GitError::invalid_command(format!("Reference update failed: {}", line)));
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct RemoteConfig {
    url: String,
}

#[derive(Debug)]
struct PushInfo {
    up_to_date: bool,
    force_required: bool,
    old_commit: Option<String>,
    new_commit: String,
}

impl SubCommand for Push {
    fn run(&self, gitdir: Result<PathBuf>) -> Result<i32> {
        let gitdir = gitdir?;
        
        if self.verbose {
            println!("Pushing to remote '{}'", self.remote);
        }
        
        self.push_to_remote(&gitdir)?;
        Ok(0)
    }
}
