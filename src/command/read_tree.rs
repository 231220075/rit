use std::path::{PathBuf,Path};
use clap::{Parser, Subcommand};
use crate::utils::index;
use crate::utils::zlib::{compress_object, decompress_file_bytes};
use crate::{
    GitError,
    Result,
};
use crate::utils::{
    fs::read_file_as_bytes,
    hash::hash_object,
    index::{Index, IndexEntry},
};
use super::SubCommand;

#[derive(Parser, Debug)]
#[command(name = "read-tree", about = "create a tree object according to the current index")]
pub struct ReadTree {
    #[arg(long, required = true, help = "Prefix to add to all paths in the tree")]
    prefix: String,

    #[arg(required = true, help = "tree hash")]
    tree_hash: String,

}

impl ReadTree {
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        let read_tree = ReadTree::try_parse_from(args)?;
        Ok(Box::new(read_tree))
    }
}

fn restore_tree_to_index(gitdir: &Path, tree_hash: &str, prefix: &str, index: &mut Index) -> Result<()> {
    // 1. 读取 tree 对象内容
    let tree_bytes = read_object_from_gitdir(gitdir, tree_hash)?;
    let entries = parse_tree_object(&tree_bytes)?;

    for entry in entries {
        match entry.mode {
            0o40000 => {
                // 目录，递归
                let sub_prefix = format!("{}/{}", prefix, entry.name);
                restore_tree_to_index(gitdir, &entry.hash, &sub_prefix, index)?;
            }
            0o100644 | 0o100755 | 0o120000 => {
                // 普通文件、可执行文件、符号链接
                let file_path = if prefix.is_empty() {
                    entry.name.clone()
                } else {
                    format!("{}/{}", prefix, entry.name)
                };
                let index_entry = IndexEntry::new(entry.mode, entry.hash.clone(), file_path);
                index.add_entry(index_entry);
            }
            _ => {}
        }
    }
    Ok(())
}


fn read_object_from_gitdir(gitdir: &Path, hash: &str) -> Result<Vec<u8>> {
    let object_path = gitdir.join("objects").join(&hash[0..2]).join(&hash[2..]);
    let decompressed = decompress_file_bytes(&object_path)?;
    Ok(decompressed)
}
fn parse_tree_object(tree_bytes: &[u8]) -> Result<Vec<IndexEntry>> {
    let mut entries = Vec::new();

    let header_end = tree_bytes.iter().position(|&b| b == 0).unwrap();
    let mut offset = header_end + 1;
    while offset < tree_bytes.len() {
        let mode_end = tree_bytes[offset..].iter().position(|&b| b == b' ').ok_or(GitError::InvalidCommand("Invalid tree object".to_string()))?;
        let mode = u32::from_str_radix(std::str::from_utf8(&tree_bytes[offset..offset + mode_end]).unwrap(), 8).map_err(|_| GitError::InvalidCommand("Invalid mode".to_string()))?;
        offset += mode_end + 1;

        let name_end = tree_bytes[offset..].iter().position(|&b| b == b'\0').ok_or(GitError::InvalidCommand("Invalid tree object".to_string()))?;
        let name = std::str::from_utf8(&tree_bytes[offset..offset + name_end]).map_err(|_| GitError::InvalidCommand("Invalid name".to_string()))?.to_string();
        offset += name_end + 1;

        let hash = hex::encode(&tree_bytes[offset..offset + 20]);
        offset += 20;

        entries.push(IndexEntry::new(mode, hash, name));
    }
    Ok(entries)
}

impl SubCommand for ReadTree {
    fn run(&self, gitdir: Result<PathBuf>) -> Result<i32> {
        let gitdir = gitdir?;
        let mut index_path = gitdir.clone();
        index_path.push("index");
        if !index_path.exists() {
            return Err(Box::new(GitError::InvalidCommand("Index file does not exist".to_string())));
        }
        let mut index = Index::new();
        index = index.read_from_file(&index_path).map_err(|_| {
            GitError::InvalidCommand("Failed to read index file".to_string())
        })?;
        for entry in &index.entries {
            println!("mode: {}, hash: {}, name: {}", entry.mode, entry.hash, entry.name);
        }
        let mode = 0o040000;
        let hash = self.tree_hash.clone();
        let name = self.prefix.clone();
        let entry = IndexEntry::new(mode, hash, name);
        index.add_entry(entry);
        index.write_to_file(&index_path).map_err(|_| {
            GitError::InvalidCommand("Failed to write index file".to_string())
        })?;
        Ok(0)

        // index = index.read_from_file(&index_path).map_err(|_| {
        //     GitError::InvalidCommand("Failed to read index file".to_string())
        // })?;
        // restore_tree_to_index(&gitdir, &self.tree_hash, &self.prefix, &mut index)?;
        // index.write_to_file(&index_path).map_err(|_| {
        //     GitError::InvalidCommand("Failed to write index file".to_string())
        // })?;
        // Ok(0)
    }


    
}

#[cfg(test)]
mod test {
    use crate::utils::test::{
        shell_spawn,
        setup_test_git_dir,
        mktemp_in,
    };

    #[test]
    fn test_simple() {
        let temp = setup_test_git_dir();
        let temp_path = temp.path();
        let temp_path_str = temp_path.to_str().unwrap();

        let file1 = mktemp_in(&temp).unwrap();
        let file1_str = file1.to_str().unwrap();
        let file2 = mktemp_in(&temp).unwrap();
        let file2_str = file2.to_str().unwrap();
        let _ = shell_spawn(&["git", "-C", temp_path_str, "update-index", "--add", &file1_str, &file2_str]).unwrap();
        let tree_hash = shell_spawn(&["git", "-C", temp_path_str, "write-tree"]).unwrap();
        let tree_hash = tree_hash.trim();
        let _ = shell_spawn(&["git", "-C", temp_path_str, "rm", "-r" ,"--cached", ":/"]).unwrap();
        let _ = shell_spawn(&["cargo", "run", "--", "-C", temp_path_str, "read-tree", "--prefix=apk", tree_hash]).unwrap();
        let out = shell_spawn(&["git", "-C", temp_path_str, "ls-files", "--stage"]).unwrap();
        println!("out: {}", out);
        //assert!(out.contains("apk/"));
    }

}
