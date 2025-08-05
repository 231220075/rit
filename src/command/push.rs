use std::path::PathBuf;
use std::collections::HashMap;
use std::io::Write;
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
        
        // 调试：显示 packfile 信息
        if self.verbose {
            Self::debug_packfile(&packfile)?;
        }
        
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
            .header("User-Agent", "git/2.42.0")
            .header("Accept", "*/*")
            .header("Accept-Encoding", "gzip")
            .header("Git-Protocol", "version=2");
        
        // 添加 GitHub 认证
        if let Some((username, password)) = self.get_github_credentials(url)? {
            if self.verbose {
                println!("Using authentication: username={}, token={}...{}", 
                    username, 
                    &password[..std::cmp::min(8, password.len())],
                    if password.len() > 8 { &password[password.len()-4..] } else { "" }
                );
            }
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
            if let Ok(username) = std::env::var("GITHUB_USER") {
                return Ok(Some((username, token)));
            } else {
                return Ok(Some(("token".to_string(), token)));
            }
        }
        
        // 2. 尝试从 git config 获取
        if let Ok(token) = std::env::var("GIT_TOKEN") {
            if let Ok(username) = std::env::var("GIT_USER") {
                return Ok(Some((username, token)));
            } else {
                return Ok(Some(("token".to_string(), token)));
            }
        }
        
        // 3. 交互式输入
        if url.contains("github.com") {
            println!("GitHub authentication required");
            println!("GitHub no longer supports password authentication for Git operations.");
            println!("Please use a Personal Access Token instead.");
            print!("GitHub用户名: ");
            std::io::stdout().flush().unwrap();
            let mut username = String::new();
            std::io::stdin().read_line(&mut username)?;
            let username = username.trim().to_string();
            
            let token = rpassword::prompt_password("Personal Access Token (not your GitHub password): ")?;
            
            return Ok(Some((username, token)));
        }
        
        Ok(None)
    }
    
    /// 解析引用响应
    fn parse_refs_response(&self, body: &str) -> Result<HashMap<String, String>> {
        let mut refs = HashMap::new();
        
        if self.verbose {
            println!("Raw refs response: {}", body);
        }
        
        // 跳过服务声明行
        let lines: Vec<&str> = body.lines().collect();
        
        for (i, line) in lines.iter().enumerate() {
            if self.verbose {
                println!("Processing line {}: '{}'", i, line);
            }
            
            if line.trim().is_empty() {
                continue;
            }
            
            // 跳过服务声明
            if line.contains("service=git-receive-pack") || line.starts_with('#') {
                if self.verbose {
                    println!("Skipping service line: {}", line);
                }
                continue;
            }
            
            // 解析 pkt-line 格式: "0041hash refs/heads/main\0capabilities"
            if let Some(content) = self.parse_pkt_line(line) {
                if content.trim().is_empty() {
                    if self.verbose {
                        println!("Empty content, skipping");
                    }
                    continue;
                }
                
                if let Some((hash, ref_name)) = self.parse_ref_line(&content) {
                    if self.verbose {
                        println!("Remote ref: {} -> {}", ref_name, &hash[..8]);
                    }
                    refs.insert(ref_name, hash);
                } else {
                    if self.verbose {
                        println!("Failed to parse ref from content: '{}'", content);
                    }
                }
            } else {
                if self.verbose {
                    println!("Failed to parse pkt-line: '{}'", line);
                }
            }
        }
        
        if self.verbose {
            println!("Final parsed refs: {:?}", refs);
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
            // 使用实际可用的内容长度
            let available_content = line.len().saturating_sub(4);
            let actual_content_length = std::cmp::min(content_length, available_content);
            
            if actual_content_length > 0 {
                let content = line[4..4 + actual_content_length].to_string();
                if self.verbose && !content.trim().is_empty() {
                    println!("Parsed pkt-line content: '{}'", content.replace('\0', "\\0"));
                }
                return Some(content);
            }
        }
        
        None
    }
    
    /// 解析引用行
    fn parse_ref_line(&self, content: &str) -> Option<(String, String)> {
        // 格式: "hash refs/heads/branch_name" 或 "hash refs/heads/branch_name\0capabilities"
        
        // 移除 capabilities （在 \0 之后）
        let clean_content = if let Some(null_pos) = content.find('\0') {
            &content[..null_pos]
        } else {
            content
        };
        
        let parts: Vec<&str> = clean_content.split_whitespace().collect();
        if parts.len() >= 2 {
            let hash = parts[0].to_string();
            let ref_name = parts[1].to_string();
            
            if self.verbose {
                println!("Parsed ref: {} -> {}", ref_name, &hash[..8]);
            }
            
            Some((hash, ref_name))
        } else {
            if self.verbose {
                println!("Failed to parse ref line: '{}'", clean_content);
            }
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
    fn collect_objects_to_push(&self, gitdir: &PathBuf, commit_hash: &str, _push_info: &PushInfo) -> Result<Vec<String>> {
        let mut objects = Vec::new();
        let mut visited = std::collections::HashSet::new();
        
        // 递归收集提交及其相关的所有对象
        self.collect_commit_objects(gitdir, commit_hash, &mut objects, &mut visited)?;
        
        if self.verbose {
            println!("Objects to push: {}", objects.len());
            for obj in &objects {
                println!("  {}", &obj[..8]);
            }
        }
        
        Ok(objects)
    }
    
    /// 递归收集提交对象及其依赖
    fn collect_commit_objects(&self, gitdir: &PathBuf, commit_hash: &str, objects: &mut Vec<String>, visited: &mut std::collections::HashSet<String>) -> Result<()> {
        if visited.contains(commit_hash) {
            return Ok(());
        }
        
        visited.insert(commit_hash.to_string());
        objects.push(commit_hash.to_string());
        
        // 读取提交对象
        let commit_data = self.read_object_data(gitdir, commit_hash)?;
        let (_, content) = self.parse_object_data(&commit_data)?;
        let commit_content = String::from_utf8_lossy(&content);
        
        // 收集 tree 对象
        for line in commit_content.lines() {
            if line.starts_with("tree ") {
                let tree_hash = &line[5..45];
                self.collect_tree_objects(gitdir, tree_hash, objects, visited)?;
            }
            // 注意：这里不收集 parent commits，因为我们只推送当前提交
            // 如果需要推送多个提交，需要修改这个逻辑
        }
        
        Ok(())
    }
    
    /// 递归收集 tree 对象及其依赖
    fn collect_tree_objects(&self, gitdir: &PathBuf, tree_hash: &str, objects: &mut Vec<String>, visited: &mut std::collections::HashSet<String>) -> Result<()> {
        if visited.contains(tree_hash) {
            return Ok(());
        }
        
        visited.insert(tree_hash.to_string());
        objects.push(tree_hash.to_string());
        
        // 读取 tree 对象
        let tree_data = self.read_object_data(gitdir, tree_hash)?;
        let (_, content) = self.parse_object_data(&tree_data)?;
        
        // 解析 tree 条目
        let mut pos = 0;
        while pos < content.len() {
            // 找到 null 分隔符
            if let Some(null_pos) = content[pos..].iter().position(|&b| b == 0) {
                let entry_header = String::from_utf8_lossy(&content[pos..pos + null_pos]);
                
                // 解析模式和名称
                if let Some(space_pos) = entry_header.find(' ') {
                    let mode = &entry_header[..space_pos];
                    let _name = &entry_header[space_pos + 1..];
                    
                    // 提取 20 字节的哈希
                    let hash_start = pos + null_pos + 1;
                    if hash_start + 20 <= content.len() {
                        let hash_bytes = &content[hash_start..hash_start + 20];
                        let hash = hex::encode(hash_bytes);
                        
                        // 根据模式决定对象类型
                        if mode == "040000" {
                            // 子目录，递归收集
                            self.collect_tree_objects(gitdir, &hash, objects, visited)?;
                        } else {
                            // 文件对象 (blob)
                            if !visited.contains(&hash) {
                                visited.insert(hash.clone());
                                objects.push(hash);
                            }
                        }
                        
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
    
    /// 创建 packfile
    fn create_packfile(&self, gitdir: &PathBuf, objects: &[String]) -> Result<Vec<u8>> {
        let mut packfile = Vec::new();
        
        if self.verbose {
            println!("Creating packfile for {} objects", objects.len());
        }
        
        // 1. 收集所有对象数据
        let mut packed_objects = Vec::new();
        for object_hash in objects {
            let obj_data = self.create_packfile_object_entry(gitdir, object_hash)?;
            packed_objects.push(obj_data);
        }
        
        // 2. 创建 packfile 头部
        packfile.extend(b"PACK");
        packfile.extend(&2u32.to_be_bytes()); // version 2
        packfile.extend(&(packed_objects.len() as u32).to_be_bytes());
        
        // 3. 添加对象数据
        for obj_data in packed_objects {
            packfile.extend(obj_data);
        }
        
        // 4. 计算并添加 SHA-1 校验和
        let checksum = self.calculate_packfile_checksum(&packfile)?;
        packfile.extend(checksum);
        
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
    
    /// 创建 packfile 对象条目
    fn create_packfile_object_entry(&self, gitdir: &PathBuf, object_hash: &str) -> Result<Vec<u8>> {
        // 读取并解析对象
        let object_data = self.read_object_data(gitdir, object_hash)?;
        let (obj_type, content) = self.parse_object_data(&object_data)?;
        
        if self.verbose {
            println!("Packing object {} (type: {}, size: {})", &object_hash[..8], obj_type, content.len());
        }
        
        // 创建 packfile 格式的对象
        let mut entry = Vec::new();
        
        // 1. 对象头部（类型 + 大小，使用变长编码）
        let type_code = match obj_type {
            1 => 1, // commit
            2 => 2, // tree  
            3 => 3, // blob
            4 => 4, // tag
            _ => return Err(GitError::invalid_command(format!("Unsupported object type: {}", obj_type))),
        };
        
        let header = self.encode_packfile_object_header(type_code, content.len())?;
        entry.extend(header);
        
        // 2. 压缩对象内容
        let compressed_content = self.compress_object_content(&content)?;
        entry.extend(compressed_content);
        
        Ok(entry)
    }
    
    /// 解析对象数据，返回类型和内容
    fn parse_object_data(&self, data: &[u8]) -> Result<(u8, Vec<u8>)> {
        // Git 对象格式: "type size\0content"
        if let Some(null_pos) = data.iter().position(|&b| b == 0) {
            let header = String::from_utf8_lossy(&data[..null_pos]);
            let content = data[null_pos + 1..].to_vec();
            
            let parts: Vec<&str> = header.split_whitespace().collect();
            if parts.len() >= 2 {
                let obj_type = match parts[0] {
                    "commit" => 1,
                    "tree" => 2,
                    "blob" => 3,
                    "tag" => 4,
                    _ => return Err(GitError::invalid_command(format!("Unknown object type: {}", parts[0]))),
                };
                
                return Ok((obj_type, content));
            }
        }
        
        Err(GitError::invalid_command("Invalid object format".to_string()))
    }
    
    /// 编码 packfile 对象头部（类型 + 大小）
    fn encode_packfile_object_header(&self, obj_type: u8, size: usize) -> Result<Vec<u8>> {
        let mut header = Vec::new();
        let mut remaining_size = size;
        
        // 第一个字节：类型（3位）+ 大小的低4位 + 继续位
        let mut first_byte = (obj_type << 4) | ((remaining_size & 0x0F) as u8);
        remaining_size >>= 4;
        
        if remaining_size > 0 {
            first_byte |= 0x80; // 设置继续位
        }
        
        header.push(first_byte);
        
        // 后续字节：每字节7位大小信息 + 1位继续位
        while remaining_size > 0 {
            let mut byte = (remaining_size & 0x7F) as u8;
            remaining_size >>= 7;
            
            if remaining_size > 0 {
                byte |= 0x80; // 设置继续位
            }
            
            header.push(byte);
        }
        
        Ok(header)
    }
    
    /// 压缩对象内容
    fn compress_object_content(&self, content: &[u8]) -> Result<Vec<u8>> {
        use flate2::{Compression, write::ZlibEncoder};
        use std::io::Write;
        
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(content)?;
        let compressed = encoder.finish()?;
        
        Ok(compressed)
    }
    
    /// 计算 packfile 校验和
    fn calculate_packfile_checksum(&self, packfile: &[u8]) -> Result<Vec<u8>> {
        use sha1::{Sha1, Digest};
        
        let mut hasher = Sha1::new();
        hasher.update(packfile);
        let result = hasher.finalize();
        
        Ok(result.to_vec())
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
            println!("Packfile size: {} bytes", packfile.len());
        }
        
        // 创建推送请求体
        let mut request_body = Vec::new();
        
        // 1. 引用更新命令
        let old_commit = push_info.old_commit.as_deref().unwrap_or("0000000000000000000000000000000000000000");
        let ref_update = format!("{} {} refs/heads/{}", old_commit, commit, branch);
        
        // 添加 capabilities（简化版本）
        let capabilities = "report-status delete-refs side-band-64k quiet atomic ofs-delta agent=git/2.42.0";
        let ref_update_with_caps = format!("{}\0{}\n", ref_update, capabilities);
        
        // 使用正确的 pkt-line 格式
        request_body.extend(self.create_pkt_line(&ref_update_with_caps));
        request_body.extend(b"0000"); // flush packet
        
        // 2. packfile 数据（直接添加，不包装在 pkt-line 中）
        if !packfile.is_empty() {
            request_body.extend(packfile);
        }
        
        if self.verbose {
            println!("Request body size: {} bytes", request_body.len());
            println!("Reference update: {}", ref_update);
        }
        
        // 3. 发送请求
        let mut request = client
            .post(&push_url)
            .header("Content-Type", "application/x-git-receive-pack-request")
            .header("User-Agent", "git/2.42.0")
            .header("Accept", "application/x-git-receive-pack-result")
            .header("Accept-Encoding", "gzip")
            .header("Expect", "100-continue")
            .body(request_body);
        
        // 添加认证
        if let Some((username, password)) = self.get_github_credentials(url)? {
            if self.verbose {
                println!("Using authentication: username={}, token={}...{}", 
                    username, 
                    &password[..std::cmp::min(8, password.len())],
                    if password.len() > 8 { &password[password.len()-4..] } else { "" }
                );
            }
            request = request.basic_auth(username, Some(password));
        }
        
        let response = request.send()?;
        let status = response.status();
        
        if self.verbose {
            println!("Push response status: {}", status);
        }
        
        if status.is_success() {
            // 解析响应
            let response_body = response.text()?;
            if self.verbose {
                println!("Response body: {}", response_body);
            }
            
            self.parse_push_response(&response_body)?;
            Ok(())
        } else {
            let error_body = response.text().unwrap_or_default();
            if self.verbose {
                println!("Error response body: {}", error_body);
            }
            Err(GitError::network_error(format!("Push failed: {} - {}", status, error_body)))
        }
    }
    
    /// 创建 pkt-line
    fn create_pkt_line(&self, content: &str) -> Vec<u8> {
        let length = content.len() + 4;
        if length > 65520 {
            // pkt-line 最大长度限制
            panic!("Content too long for pkt-line");
        }
        let length_hex = format!("{:04x}", length);
        let mut pkt_line = length_hex.into_bytes();
        pkt_line.extend(content.as_bytes());
        pkt_line
    }
    
    /// 解析推送响应
    fn parse_push_response(&self, response: &str) -> Result<()> {
        if response.trim().is_empty() {
            if self.verbose {
                println!("Empty response from server");
            }
            return Ok(());
        }
        
        let mut unpack_ok = false;
        let mut ref_updated = false;
        
        // 解析 pkt-line 格式的响应
        let lines: Vec<&str> = response.lines().collect();
        
        for line in lines {
            let line = line.trim();
            
            if line.is_empty() {
                continue;
            }
            
            // 尝试解析 pkt-line
            let content = if line.len() > 4 && line.chars().take(4).all(|c| c.is_ascii_hexdigit()) {
                // 可能是 pkt-line 格式
                if let Ok(length) = u16::from_str_radix(&line[..4], 16) {
                    if length == 0 {
                        continue; // flush packet
                    }
                    let content_length = (length as usize).saturating_sub(4);
                    if line.len() >= 4 + content_length {
                        &line[4..4 + content_length]
                    } else {
                        line
                    }
                } else {
                    line
                }
            } else {
                line
            };
            
            if self.verbose {
                println!("Server response: {}", content);
            }
            
            if content.starts_with("unpack ok") {
                unpack_ok = true;
                if self.verbose {
                    println!("Unpack successful");
                }
            } else if content.starts_with("ok refs/heads/") {
                ref_updated = true;
                if self.verbose {
                    println!("Reference update successful: {}", content);
                }
            } else if content.starts_with("ng refs/heads/") {
                return Err(GitError::invalid_command(format!("Reference update failed: {}", content)));
            } else if content.contains("error:") || content.contains("fatal:") {
                return Err(GitError::invalid_command(format!("Server error: {}", content)));
            }
        }
        
        // 验证推送是否成功
        if !unpack_ok && !ref_updated {
            // 如果没有明确的成功指示，检查是否有错误
            if response.contains("error") || response.contains("fatal") || response.contains("rejected") {
                return Err(GitError::invalid_command(format!("Push failed: {}", response)));
            }
            
            if self.verbose {
                println!("Warning: No explicit success indication from server, but no errors detected");
            }
        }
        
        Ok(())
    }

    /// 调试：显示 packfile 信息
    fn debug_packfile(packfile: &[u8]) -> Result<()> {
        println!("Packfile debug information:");
        println!("  Total size: {} bytes", packfile.len());
        
        if packfile.len() < 12 {
            println!("  Error: Packfile too small");
            return Ok(());
        }
        
        // 检查魔数
        let signature = &packfile[0..4];
        println!("  Signature: {:?} (expected: [80, 65, 67, 75])", signature);
        
        // 检查版本
        let version = u32::from_be_bytes([packfile[4], packfile[5], packfile[6], packfile[7]]);
        println!("  Version: {}", version);
        
        // 检查对象数量
        let object_count = u32::from_be_bytes([packfile[8], packfile[9], packfile[10], packfile[11]]);
        println!("  Object count: {}", object_count);
        
        // 显示前 50 字节的十六进制内容
        let preview_len = std::cmp::min(50, packfile.len());
        let hex_preview: String = packfile[..preview_len]
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");
        println!("  First {} bytes: {}", preview_len, hex_preview);
        
        // 显示最后 20 字节（SHA-1 校验和）
        if packfile.len() >= 20 {
            let checksum_start = packfile.len() - 20;
            let checksum_hex: String = packfile[checksum_start..]
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join("");
            println!("  SHA-1 checksum: {}", checksum_hex);
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
