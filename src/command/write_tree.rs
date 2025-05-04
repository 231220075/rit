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
#[command(name = "write-tree", about = "create a tree object according to the current index")]
pub struct WriteTree {
    //#[arg(skip)]
    //gitdir: PathBuf,
}
impl WriteTree {
    pub fn from_args(mut args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        let mut write_tree = WriteTree::try_parse_from(args)?;
        //write_tree.gitdir = gitdir;
        Ok(Box::new(write_tree))
    }

    fn build_tree_content(&self, index: &Index) -> Result<Vec<u8>>{
        let mut tree_content = Vec::new();
        for entry in &index.entries {
            let mode = format!("{:o}", entry.mode);
            tree_content.extend_from_slice(mode.as_bytes());
            tree_content.push(b' ');

            tree_content.extend_from_slice(entry.name.as_bytes());
            tree_content.push(0);

            let hash_bytes = hex::decode(&entry.hash).map_err(|_| {
                GitError::InvalidCommand("Invalid hash format".to_string())
            })?;
            tree_content.extend_from_slice(&hash_bytes);

        }
        Ok(tree_content)
    }
}
impl SubCommand for WriteTree {
    fn run(&self) -> Result<i32>{
        //let index_path = self.gitdir.join("index");
        let index_path = Path::new(".").join("index");
        let index = Index::read_from_file(&index_path).map_err(|_| {
            GitError::InvalidCommand("Failed to read index file".to_string())
        })?;
        let tree_content = self.build_tree_content(&index)?;
        let tree_hash = hash_object(tree_content.clone(), "tree")?;
        //let mut objpath = self.gitdir.join("objects");
        let mut objpath = Path::new(".").join("objects");
        objpath.push(&tree_hash[0..2]);
        objpath.push(&tree_hash[2..]);
        std::fs::create_dir_all(objpath.parent().unwrap())?;

        let compressed = compress_object(tree_content)?;
        std::fs::write(objpath, compressed)?;
        println!("{}", tree_hash);
        Ok(0)
    }

}