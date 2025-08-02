use std::path::PathBuf;
use std::fs;
use clap::{Parser, Subcommand};
use crate::{GitError, Result};
use super::SubCommand;

#[derive(Parser, Debug)]
#[command(name = "remote", about = "管理远程仓库")]
pub struct Remote {
    #[command(subcommand)]
    command: Option<RemoteCommand>,
    
    /// 显示详细信息
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum RemoteCommand {
    /// 添加远程仓库
    Add {
        /// 远程仓库名称
        name: String,
        /// 远程仓库URL
        url: String,
    },
    /// 删除远程仓库
    Remove {
        /// 远程仓库名称
        name: String,
    },
    /// 显示远程仓库URL
    Show {
        /// 远程仓库名称（可选）
        name: Option<String>,
    },
    /// 设置远程仓库URL
    SetUrl {
        /// 远程仓库名称
        name: String,
        /// 新的URL
        url: String,
    },
}

impl Remote {
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        Ok(Box::new(Remote::try_parse_from(args)?))
    }
    
    /// 读取配置文件
    fn read_config(&self, gitdir: &PathBuf) -> Result<String> {
        let config_path = gitdir.join("config");
        if config_path.exists() {
            Ok(fs::read_to_string(config_path)?)
        } else {
            // 如果配置文件不存在，创建基本配置
            Ok(String::from("[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = false\n\tlogallrefupdates = true\n"))
        }
    }
    
    /// 写入配置文件
    fn write_config(&self, gitdir: &PathBuf, content: &str) -> Result<()> {
        let config_path = gitdir.join("config");
        fs::write(config_path, content)?;
        Ok(())
    }
    
    /// 添加远程仓库
    fn add_remote(&self, gitdir: &PathBuf, name: &str, url: &str) -> Result<()> {
        let mut config = self.read_config(gitdir)?;
        
        // 检查远程仓库是否已存在
        if config.contains(&format!("[remote \"{}\"]", name)) {
            return Err(GitError::invalid_command(format!("Remote '{}' already exists", name)));
        }
        
        // 添加远程仓库配置
        let remote_config = format!(
            "\n[remote \"{}\"]\n\turl = {}\n\tfetch = +refs/heads/*:refs/remotes/{}/*\n",
            name, url, name
        );
        
        config.push_str(&remote_config);
        self.write_config(gitdir, &config)?;
        
        println!("Added remote '{}' -> {}", name, url);
        Ok(())
    }
    
    /// 删除远程仓库
    fn remove_remote(&self, gitdir: &PathBuf, name: &str) -> Result<()> {
        let config = self.read_config(gitdir)?;
        let lines: Vec<&str> = config.lines().collect();
        let mut new_lines = Vec::new();
        let mut in_remote_section = false;
        let mut found_remote = false;
        
        for line in lines {
            let trimmed = line.trim();
            
            if trimmed == &format!("[remote \"{}\"]", name) {
                in_remote_section = true;
                found_remote = true;
                continue; // 跳过这一行
            }
            
            if trimmed.starts_with('[') && trimmed.ends_with(']') && in_remote_section {
                in_remote_section = false;
            }
            
            if !in_remote_section {
                new_lines.push(line.to_string());
            }
        }
        
        if !found_remote {
            return Err(GitError::invalid_command(format!("Remote '{}' not found", name)));
        }
        
        let new_config = new_lines.join("\n");
        self.write_config(gitdir, &new_config)?;
        
        // 删除远程跟踪分支
        let remote_refs_path = gitdir.join("refs").join("remotes").join(name);
        if remote_refs_path.exists() {
            fs::remove_dir_all(&remote_refs_path)?;
        }
        
        println!("Removed remote '{}'", name);
        Ok(())
    }
    
    /// 显示远程仓库
    fn show_remotes(&self, gitdir: &PathBuf, name: Option<&str>) -> Result<()> {
        let config = self.read_config(gitdir)?;
        let mut remotes = Vec::new();
        let mut current_remote = None;
        let mut current_url = None;
        
        for line in config.lines() {
            let trimmed = line.trim();
            
            // 匹配远程仓库section
            if let Some(start) = trimmed.find("[remote \"") {
                if let Some(end) = trimmed.find("\"]") {
                    if let Some(prev_remote) = current_remote {
                        remotes.push((prev_remote, current_url.unwrap_or_else(|| "".to_string())));
                    }
                    let remote_name = &trimmed[start + 9..end]; // "[remote \"" 的长度是 9
                    current_remote = Some(remote_name.to_string());
                    current_url = None;
                }
            } else if let Some(url) = trimmed.strip_prefix("url = ") {
                current_url = Some(url.to_string());
            } else if trimmed.starts_with('[') && trimmed.ends_with(']') {
                // 新的section开始
                if let Some(prev_remote) = current_remote {
                    remotes.push((prev_remote, current_url.unwrap_or_else(|| "".to_string())));
                    current_remote = None;
                    current_url = None;
                }
            }
        }
        
        // 处理最后一个远程仓库
        if let Some(prev_remote) = current_remote {
            remotes.push((prev_remote, current_url.unwrap_or_else(|| "".to_string())));
        }
        
        if let Some(target_name) = name {
            // 显示特定远程仓库
            if let Some((_, url)) = remotes.iter().find(|(n, _)| n == target_name) {
                if self.verbose {
                    println!("* remote {}", target_name);
                    println!("  Fetch URL: {}", url);
                    println!("  Push  URL: {}", url);
                } else {
                    println!("{}", url);
                }
            } else {
                return Err(GitError::invalid_command(format!("Remote '{}' not found", target_name)));
            }
        } else {
            // 显示所有远程仓库
            if remotes.is_empty() {
                println!("No remotes configured");
            } else {
                for (name, url) in remotes {
                    if self.verbose {
                        println!("{}\t{} (fetch)", name, url);
                        println!("{}\t{} (push)", name, url);
                    } else {
                        println!("{}", name);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// 设置远程仓库URL
    fn set_url(&self, gitdir: &PathBuf, name: &str, new_url: &str) -> Result<()> {
        let config = self.read_config(gitdir)?;
        let lines: Vec<&str> = config.lines().collect();
        let mut new_lines = Vec::new();
        let mut in_remote_section = false;
        let mut found_remote = false;
        let mut updated = false;
        
        for line in lines {
            let trimmed = line.trim();
            
            if trimmed == &format!("[remote \"{}\"]", name) {
                in_remote_section = true;
                found_remote = true;
                new_lines.push(line.to_string());
                continue;
            }
            
            if trimmed.starts_with('[') && trimmed.ends_with(']') && in_remote_section {
                in_remote_section = false;
            }
            
            if in_remote_section && trimmed.starts_with("url = ") {
                new_lines.push(format!("\turl = {}", new_url));
                updated = true;
            } else {
                new_lines.push(line.to_string());
            }
        }
        
        if !found_remote {
            return Err(GitError::invalid_command(format!("Remote '{}' not found", name)));
        }
        
        if !updated {
            return Err(GitError::invalid_command(format!("No URL found for remote '{}'", name)));
        }
        
        let new_config = new_lines.join("\n");
        self.write_config(gitdir, &new_config)?;
        
        println!("Updated remote '{}' URL to {}", name, new_url);
        Ok(())
    }
}

impl SubCommand for Remote {
    fn run(&self, gitdir: Result<PathBuf>) -> Result<i32> {
        let gitdir = gitdir?;
        
        match &self.command {
            Some(RemoteCommand::Add { name, url }) => {
                self.add_remote(&gitdir, name, url)?;
            }
            Some(RemoteCommand::Remove { name }) => {
                self.remove_remote(&gitdir, name)?;
            }
            Some(RemoteCommand::Show { name }) => {
                self.show_remotes(&gitdir, name.as_deref())?;
            }
            Some(RemoteCommand::SetUrl { name, url }) => {
                self.set_url(&gitdir, name, url)?;
            }
            None => {
                // 默认显示所有远程仓库
                self.show_remotes(&gitdir, None)?;
            }
        }
        
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test::{setup_test_git_dir, tempdir};
    
    #[test]
    fn test_add_remote() -> Result<()> {
        let repo = setup_test_git_dir();
        let gitdir = repo.path().join(".git");
        
        let remote = Remote {
            command: Some(RemoteCommand::Add {
                name: "origin".to_string(),
                url: "https://github.com/user/repo.git".to_string(),
            }),
            verbose: false,
        };
        
        remote.run(Ok(gitdir.clone()))?;
        
        let config = remote.read_config(&gitdir)?;
        assert!(config.contains("https://github.com/user/repo.git"));
        
        Ok(())
    }
    
    #[test]
    fn test_show_remotes() -> Result<()> {
        let repo = setup_test_git_dir();
        let gitdir = repo.path().join(".git");
        
        let remote = Remote {
            command: Some(RemoteCommand::Add {
                name: "origin".to_string(),
                url: "https://github.com/user/repo.git".to_string(),
            }),
            verbose: false,
        };
        
        remote.run(Ok(gitdir.clone()))?;
        
        let show_remote = Remote {
            command: None,
            verbose: false,
        };
        
        show_remote.run(Ok(gitdir))?;
        
        Ok(())
    }
}
