use std::path::{PathBuf,Path};
use clap::{Parser, Subcommand};
use crate::cli::command;
use crate::utils::index;
use crate::utils::zlib::compress_object;
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
    pub fn from_args(mut args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        let mut read_tree = ReadTree::try_parse_from(args)?;
        Ok(Box::new(read_tree))
    }
}

impl SubCommand for ReadTree {
    fn run(&self) -> Result<i32> {
        let index_path = Path::new(".git").join("index");
        let mut index = Index::new();
        index = index.read_from_file(&index_path).map_err(|_| {
            GitError::InvalidCommand("Failed to read index file".to_string())
        })?;
        println!("index len = {}", index.entries.len());
        for entry in &index.entries {
            println!("mode: {}, hash: {}, name: {}", entry.mode, entry.hash, entry.name);
        }
        let mode = 0o040000;
        let hash = self.tree_hash.clone();
        let name = self.prefix.clone();
        let entry = IndexEntry::new(mode, hash, name);
        index.add_entry(entry);
        println!("index len = {}", index.entries.len());
        index.write_to_file(&index_path).map_err(|_| {
            GitError::InvalidCommand("Failed to write index file".to_string())
        })?;
        Ok(0)
    }
}