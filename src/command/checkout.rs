use std::path::PathBuf;
use clap::{Parser, Subcommand};
use crate::{
    GitError,
    Result,
    utils::refs::{read_head_ref, write_head_ref, read_ref_commit, write_ref_commit},
};
use super::SubCommand;
use std::fs;
use crate::utils::zlib::decompress_file_bytes;
use crate::utils::{
    tree::{Tree, FileMode},
    blob::Blob,
    index::Index,
};
use bincode;
use crate::utils::hash::hash_object;
use crate::utils::index::IndexEntry;

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

    pub fn restore_workspace(gitdir: &PathBuf, commit_hash: &str) -> Result<()> {
        let commit_path = gitdir.join("objects").join(&commit_hash[0..2]).join(&commit_hash[2..]);
        let decompressed = decompress_file_bytes(&commit_path)?;
        if let Some(tree_hash) = Checkout::extract_tree_hash(&decompressed) {
            println!("tree_hash: {}", tree_hash);
            let tree_path = gitdir.join("objects").join(&tree_hash[0..2]).join(&tree_hash[2..]);
            let tree_data = decompress_file_bytes(&tree_path)?;

            let tree: Tree = Tree::try_from(tree_data)
                .map_err(|_| GitError::invalid_command(format!("failed to parse tree data for {}", tree_hash)))?;
            Checkout::restore_tree(gitdir, &PathBuf::from("."), &tree)?;
        }
        else {
            return Err(GitError::invalid_command(format!("commit {} does not contain a tree", commit_hash)));
        }
        Ok(())
    }

    fn extract_tree_hash(data: &[u8]) -> Option<String> {
        let mut content = String::from_utf8_lossy(data).to_string();
        content = content.replace("tree ", "\ntree ");
        // println!("content: {}", content);
        for line in content.lines() {
            if let Some(hash) = line.strip_prefix("tree ") {
                return Some(hash.to_string()); // 提取 tree_hash
            }
        }

        None 
    }

    fn restore_tree(gitdir: &PathBuf, base_path:&PathBuf, tree: &Tree) -> Result<()> {
        for entry in &tree.0 {
            let file_path = base_path.join(&entry.path);

            // if let Some(staged_data) = Checkout::get_staged_file(gitdir, &entry.path)?{
            //     println!("staged_data: {:?}", &staged_data);
            //     fs::write(&file_path, &staged_data)
            //         .map_err(|_| GitError::failed_to_write_file(&file_path.to_string_lossy()))?;
            //     continue;
            // }

            match entry.mode {
                FileMode::Blob =>{
                    // if let Some(staged_data) = Checkout::get_staged_file(gitdir, &entry.path)?{
                    //     println!("staged_data: {:?}", &staged_data);
                    //     fs::write(&file_path, &staged_data)
                    //         .map_err(|_| GitError::failed_to_write_file(&file_path.to_string_lossy()))?;
                    //     continue;
                    // }

                    let blob_path = gitdir.join("objects").join(&entry.hash[0..2]).join(&entry.hash[2..]);
                    let blob_data = decompress_file_bytes(&blob_path)?;
                    let blob = Blob::try_from(blob_data)
                        .map_err(|_| GitError::invalid_command(format!("failed to parse blob data for {}", entry.hash)))?;
                    let content: Vec<u8> = Vec::from(blob);
                    println!("content: {:?}", content);
                    fs::write(&file_path, &content)
                        .map_err(|_| GitError::failed_to_write_file(&file_path.to_string_lossy()))?;
                }
                FileMode::Tree => {
                    fs::create_dir_all(&file_path)
                        .map_err(|_| GitError::failed_to_write_file(&file_path.to_string_lossy()))?;
                    let sub_tree_path = gitdir.join("objects").join(&entry.hash[0..2]).join(&entry.hash[2..]);
                    let sub_tree_data = decompress_file_bytes(&sub_tree_path)?;
                    let sub_tree: Tree = Tree::try_from(sub_tree_data)
                        .map_err(|_| GitError::invalid_command(format!("failed to parse tree data for {}", entry.hash)))?;
                    Checkout::restore_tree(gitdir, &file_path, &sub_tree)?;
                }  
                _ => {
                    return Err(GitError::invalid_command(format!("unsupported file mode: {:?}", entry.mode)));
                }          
            }
        }
        Ok(())
    }

    fn get_staged_file(gitdir: &PathBuf, path: &PathBuf) -> Result<Option<Vec<u8>>> {
        let index_path = gitdir.join("index");

        let index = Index::new().read_from_file(&index_path).map_err(|_| {
            GitError::failed_to_read_file(&index_path.to_string_lossy())
        })?;

        
        if let Some(entry) = index.entries.iter().find(|e| e.name == path.to_string_lossy()) {
            let blob_path = gitdir.join("objects").join(&entry.hash[0..2]).join(&entry.hash[2..]);
            let blob_data = decompress_file_bytes(&blob_path)?;
            let blob: Blob = Blob::try_from(blob_data)
                .map_err(|_| GitError::invalid_command(format!("failed to parse blob data for {}", entry.hash)))?;
            return Ok(Some(Vec::from(blob)));
        }
        Ok(None)
    }

    fn is_workspace_modified(gitdir: &PathBuf) -> Result<bool> {
        let index_path = gitdir.join("index");
        let index = Index::new().read_from_file(&index_path).map_err(|_| {
            GitError::failed_to_read_file(&index_path.to_string_lossy())
        })?;

        // 遍历 index 中的所有条目
        for entry in &index.entries {
            let file_path = PathBuf::from(&entry.name);

            // 检查工作区中是否存在对应的文件
            if !file_path.exists() {
                println!("File deleted: {:?}", file_path);
                return Ok(true); // 文件被删除
            }

            // 如果是文件（blob），计算文件哈希并比较
            if entry.mode == 0o100644{
                let file_content = fs::read(&file_path).map_err(|_| {
                    GitError::failed_to_read_file(&file_path.to_string_lossy())
                })?;
                let file_hash = hash_object::<Blob>(file_content)?;
                if file_hash != entry.hash {
                    println!("File modified: {:?}", file_path);
                    return Ok(true); // 文件内容不同
                }
            }

            // 如果是目录（tree），递归检查子条目
            if entry.mode == 0o40000 {
                let tree_path = gitdir.join("objects").join(&entry.hash[0..2]).join(&entry.hash[2..]);
                let tree_data = decompress_file_bytes(&tree_path)?;
                let tree: Tree = Tree::try_from(tree_data)
                    .map_err(|_| GitError::invalid_command(format!("failed to parse tree data for {}", entry.hash)))?;

                if Self::is_workspace_modified_for_tree(gitdir, &file_path, &tree)? {
                    return Ok(true);
                }
            }
        }

        Ok(false) // 工作区和 index 一致
    }

    fn is_workspace_modified_for_tree(gitdir: &PathBuf, base_path: &PathBuf, tree: &Tree) -> Result<bool> {
        for entry in &tree.0 {
            let file_path = base_path.join(&entry.path);

            // 检查工作区中是否存在对应的文件
            if !file_path.exists() {
                println!("File deleted: {:?}", file_path);
                return Ok(true); // 文件被删除
            }

            // 如果是文件（blob），计算文件哈希并比较
            if entry.mode == FileMode::Blob {
                let file_content = fs::read(&file_path).map_err(|_| {
                    GitError::failed_to_read_file(&file_path.to_string_lossy())
                })?;
                let file_hash = hash_object::<Blob>(file_content)?;
                if file_hash != entry.hash {
                    println!("File modified: {:?}", file_path);
                    return Ok(true); // 文件内容不同
                }
            }

            // 如果是目录（tree），递归检查子条目
            if entry.mode == FileMode::Tree {
                let sub_tree_path = gitdir.join("objects").join(&entry.hash[0..2]).join(&entry.hash[2..]);
                let sub_tree_data = decompress_file_bytes(&sub_tree_path)?;
                let sub_tree: Tree = Tree::try_from(sub_tree_data)
                    .map_err(|_| GitError::invalid_command(format!("failed to parse tree data for {}", entry.hash)))?;

                if Self::is_workspace_modified_for_tree(gitdir, &file_path, &sub_tree)? {
                    return Ok(true);
                }
            }
        }

        Ok(false) // 工作区和 tree 一致
    }
    fn is_index_modified(gitdir: &PathBuf, tree: &Tree) -> Result<bool> {
        // 读取 index 文件
        let index_path = gitdir.join("index");
        let index = Index::new().read_from_file(&index_path).map_err(|_| {
            GitError::failed_to_read_file(&index_path.to_string_lossy())
        })?;

        // 创建一个集合存储 tree 中的所有文件路径
        let tree_paths: std::collections::HashSet<_> = tree.0.iter().map(|entry| &entry.path).collect();

        // 遍历 tree 中的文件
        for entry in &tree.0 {
            if let Some(index_entry) = index.entries.iter().find(|e| e.name == entry.path.to_string_lossy()) {
                // 比较 tree 文件的哈希值与 index 中的哈希值
                if entry.hash != index_entry.hash {
                    println!("entry.hash: {:?}", entry.hash);
                    println!("index_entry.hash: {:?}", index_entry.hash);
                    return Ok(true); // 文件内容不同
                }
            } else {
                // 如果 tree 中的文件在 index 中不存在
                println!("File missing in index: {:?}", entry.path);
                return Ok(true); // 文件缺失
            }
        }

        // 检查 index 中是否有多余的条目
        for index_entry in &index.entries {
            if !tree_paths.contains(&PathBuf::from(&index_entry.name)) {
                println!("Extra file in index: {:?}", index_entry.name);
                return Ok(true); // 多余的文件
            }
        }

        Ok(false) // index 和 tree 一致
    }

    fn merge_tree_into_index(gitdir: &PathBuf, tree: &Tree) -> Result<()> {
        let index_path = gitdir.join("index");
        let mut index = Index::new().read_from_file(&index_path).map_err(|_| {
            GitError::failed_to_read_file(&index_path.to_string_lossy())
        })?;

        for entry in &tree.0 {
            if let Some(index_entry) = index.entries.iter_mut().find(|e| e.name == entry.path.to_string_lossy()) {
                // 如果 index 中已存在条目，保留内容不同的原条目
                if index_entry.hash != entry.hash {
                    println!("Conflict in index for file: {:?}", entry.path);
                    continue;
                }
            } else {
                // 如果 index 中不存在条目，添加新的条目
                index.entries.push(IndexEntry {
                    name: entry.path.to_string_lossy().to_string(),
                    mode: entry.mode as u32,
                    hash: entry.hash.clone(),
                });
            }
        }

        index.write_to_file(&index_path).map_err(|_| {
            GitError::failed_to_write_file(&index_path.to_string_lossy())
        })?;

        Ok(())
    }

    fn merge_index_into_workspace(gitdir: &PathBuf) -> Result<()> {
        let index_path = gitdir.join("index");
        let index = Index::new().read_from_file(&index_path).map_err(|_| {
            GitError::failed_to_read_file(&index_path.to_string_lossy())
        })?;

        for entry in &index.entries {
            let file_path = PathBuf::from(&entry.name);

            match entry.mode {
                0o100644 => {
                    // 如果是文件（blob），处理文件内容
                    if file_path.exists() {
                        let file_content = fs::read(&file_path).map_err(|_| {
                            GitError::failed_to_read_file(&file_path.to_string_lossy())
                        })?;
                        let file_hash = hash_object::<Blob>(file_content)?;
                        if file_hash != entry.hash {
                            println!("Conflict in workspace for file: {:?}", file_path);
                            continue;
                        }
                    }
                    let blob_path = gitdir.join("objects").join(&entry.hash[0..2]).join(&entry.hash[2..]);
                    let blob_data = decompress_file_bytes(&blob_path)?;
                    let blob: Blob = Blob::try_from(blob_data).map_err(|_| {
                        GitError::invalid_command(format!("failed to parse blob data for {}", entry.hash))
                    })?;
                    let content: Vec<u8> = Vec::from(blob);
                    fs::write(&file_path, content).map_err(|_| {
                        GitError::failed_to_write_file(&file_path.to_string_lossy())
                    })?;
                }
                0o40000 => {
                    // 如果是目录（tree），递归处理子条目
                    if !file_path.exists() {
                        fs::create_dir_all(&file_path).map_err(|_| {
                            GitError::failed_to_write_file(&file_path.to_string_lossy())
                        })?;
                    }
                    let tree_path = gitdir.join("objects").join(&entry.hash[0..2]).join(&entry.hash[2..]);
                    let tree_data = decompress_file_bytes(&tree_path)?;
                    let tree: Tree = Tree::try_from(tree_data).map_err(|_| {
                        GitError::invalid_command(format!("failed to parse tree data for {}", entry.hash))
                    })?;
                    Self::merge_index_into_workspace_for_tree(gitdir, &file_path, &tree)?;
                }
                _ => {
                    return Err(GitError::invalid_command(format!(
                        "Unsupported file mode: {:?}",
                        entry.mode
                    )));
                }
            }
        }
        Ok(())
    }

    fn merge_index_into_workspace_for_tree(gitdir: &PathBuf, base_path: &PathBuf, tree: &Tree) -> Result<()> {
        for entry in &tree.0 {
            let file_path = base_path.join(&entry.path);

            match entry.mode {
                FileMode::Blob => {
                    // 如果是文件（blob），处理文件内容
                    if file_path.exists() {
                        let file_content = fs::read(&file_path).map_err(|_| {
                            GitError::failed_to_read_file(&file_path.to_string_lossy())
                        })?;
                        let file_hash = hash_object::<Blob>(file_content)?;
                        if file_hash != entry.hash {
                            println!("Conflict in workspace for file: {:?}", file_path);
                            continue;
                        }
                    }
                    let blob_path = gitdir.join("objects").join(&entry.hash[0..2]).join(&entry.hash[2..]);
                    let blob_data = decompress_file_bytes(&blob_path)?;
                    fs::write(&file_path, blob_data).map_err(|_| {
                        GitError::failed_to_write_file(&file_path.to_string_lossy())
                    })?;
                }
                FileMode::Tree => {
                    // 如果是目录（tree），递归处理子条目
                    if !file_path.exists() {
                        fs::create_dir_all(&file_path).map_err(|_| {
                            GitError::failed_to_write_file(&file_path.to_string_lossy())
                        })?;
                    }
                    let sub_tree_path = gitdir.join("objects").join(&entry.hash[0..2]).join(&entry.hash[2..]);
                    let sub_tree_data = decompress_file_bytes(&sub_tree_path)?;
                    let sub_tree: Tree = Tree::try_from(sub_tree_data).map_err(|_| {
                        GitError::invalid_command(format!("failed to parse tree data for {}", entry.hash))
                    })?;
                    Self::merge_index_into_workspace_for_tree(gitdir, &file_path, &sub_tree)?;
                }
                _ => {
                    return Err(GitError::invalid_command(format!(
                        "Unsupported file mode: {:?}",
                        entry.mode
                    )));
                }
            }
        }
        Ok(())
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
            write_head_ref(&gitdir, &format!("refs/heads/{}", self.branch_name))?;
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

            
            let current_commit_hash = read_ref_commit(&gitdir, &current_ref)?;
            let tree_hash = Checkout::extract_tree_hash(&decompress_file_bytes(
                &gitdir.join("objects").join(&current_commit_hash[0..2]).join(&current_commit_hash[2..]),
                )?).ok_or_else(|| {
                    GitError::invalid_command(format!("commit {} does not contain a tree", current_commit_hash))
                })?;
            let tree_path = gitdir.join("objects").join(&tree_hash[0..2]).join(&tree_hash[2..]);
            let tree_data = decompress_file_bytes(&tree_path)?;
            let tree: Tree = Tree::try_from(tree_data)
                .map_err(|_| GitError::invalid_command(format!("failed to parse tree data for {}", tree_hash)))?; 
            
            let workspace_modified = Self::is_workspace_modified(&gitdir)?;// 检查工作区是否有未暂存的修改
            let index_modified = Self::is_index_modified(&gitdir, &tree)?;//检查index是否有未commit的修改

            if !workspace_modified && !index_modified {
                let commit_hash = read_ref_commit(&gitdir, &branch_path.to_string_lossy())?;

                // 如果没有未暂存或未提交的更改
                println!("No uncommitted changes. Switching branch...");
                write_head_ref(&gitdir, &format!("refs/heads/{}", self.branch_name))?;
                Checkout::restore_workspace(&gitdir, &commit_hash)?;
                return Ok(0);
            }
            println!("workspace_modified: {:?}", workspace_modified);
            println!("index_modified: {:?}", index_modified);

            println!("Uncommitted changes detected. Attempting to merge changes...");
            Checkout::merge_tree_into_index(&gitdir, &tree)?;
            Checkout::merge_index_into_workspace(&gitdir)?;
            write_head_ref(&gitdir, &format!("refs/heads/{}", self.branch_name))?;
            println!("Switched to branch '{}'", self.branch_name);

            Ok(0)
            
        }
    }
}