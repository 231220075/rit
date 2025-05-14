use std::path::{PathBuf,Path};
use clap::{Parser, Subcommand};
use crate::{
    GitError,
    Result,
};
use crate::utils::{
    fs::read_file_as_bytes,
    hash::hash_object,
    zlib::compress_object,
    index::{Index, IndexEntry},
    tree::Tree,
};
use super::SubCommand;
use hex;

#[derive(Parser, Debug)]
#[command(name = "write-tree", about = "create a tree object according to the current index")]
pub struct WriteTree {
    //#[arg(skip)]
    //gitdir: PathBuf,
}
impl WriteTree {
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        let write_tree = WriteTree::try_parse_from(args)?;
        //write_tree.gitdir = gitdir;
        Ok(Box::new(write_tree))
    }

    fn build_tree_content(&self, index: &Index) -> Result<Vec<u8>>{
        let mut tree_content = Vec::new();
        //tree_content.extend_from_slice(b"tree ");
        let mut temp =Vec::new();

        for entry in &index.entries {
            let mode = format!("{:o}", entry.mode);
            temp.extend_from_slice(mode.as_bytes());
            temp.push(b' ');
            temp.extend_from_slice(entry.name.as_bytes());
            temp.push(b'\0');
            let hash_bytes = hex::decode(&entry.hash).map_err(|_| {
                GitError::InvalidCommand(format!("Invalid hash format: {}", entry.hash))
            })?;
            temp.extend_from_slice(&hash_bytes);
        }
        //let len_str = format!("{}\0", temp.len());
        //tree_content.extend_from_slice(len_str.as_bytes());
        tree_content.extend_from_slice(&temp);
        Ok(tree_content)
    }
}
impl SubCommand for WriteTree {
    fn run(&self, gitdir: Result<PathBuf>) -> Result<i32> {
        let gitdir = gitdir?;
        let index_path =gitdir.clone().join("index");
        let index = Index::new();
        let index = index.read_from_file(&index_path).map_err(|_| {
            GitError::InvalidCommand(index_path.to_str().unwrap().to_string())
        })?;
        
        let tree_content = self.build_tree_content(&index)?;
        let tree_hash = hash_object::<Tree>(tree_content.clone())?;
        //let mut objpath = self.gitdir.join("objects");
        let mut objpath = gitdir.clone().join("objects");
        objpath.push(&tree_hash[0..2]);
        objpath.push(&tree_hash[2..]);
        std::fs::create_dir_all(objpath.parent().unwrap())?;
        
        let compressed = compress_object::<Tree>(tree_content)?;
        
        std::fs::write(objpath, compressed)?;
        println!("{}", tree_hash);
        Ok(0)
    }

}
