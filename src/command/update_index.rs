use std::path::{PathBuf,Path};
use clap::{Parser, Subcommand};
use crate::cli::command;
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
#[command(name = "update-index", about = "update the index file")]
pub struct UpdateIndex {
    #[arg(long, help = "Add file contents to the index")]
    add: bool,

    #[arg(long, num_args = 3, help = "Sepcify file mode, hash and name")]
    cacheinfo: Option<Vec<String>>,

    #[arg(help = "Path to the file")]
    name: Option<String>, 

    //#[arg(skip)]
    //gitdir: PathBuf,
}

impl UpdateIndex {
    pub fn from_args(mut args: impl Iterator<Item = String>)
-> Result<Box<dyn SubCommand>>{
        //Ok(Box::new(UpdateIndex::try_parse_from(args)?))
        let mut update_index = UpdateIndex::try_parse_from(args)?;
        //update_index.gitdir = gitdir;
        Ok(Box::new(update_index))
}    
}
impl SubCommand for UpdateIndex {
    fn run(&self) -> Result<i32> {
        //let index_path = self.gitdir.join("index");
        let index_path = Path::new(".").join("index");
        let mut index = Index::new();
        if let Some(cacheinfo) =&self.cacheinfo {
            if cacheinfo.len() != 3 {
                return Err(Box::new(GitError::InvalidCommand("cacheinfo".to_string())));
            }
            let mode = u32::from_str_radix(&cacheinfo[0], 8).map_err(|_| {
                GitError::InvalidCommand("Invalid file mode".to_string())
            })?;
            let hash = cacheinfo[1].clone();
            let name = cacheinfo[2].clone();

            let entry = IndexEntry::new(mode, hash, name);
            index.add_entry(entry);
    
        }
        else if self.add {
            if let Some(name) = &self.name {
                let current_dir = std::env::current_dir()?;
                let file_path = current_dir.join(name);
                if !file_path.exists() {
                    return Err(Box::new(GitError::FileNotFound(file_path.to_string_lossy().to_string())));
                }
                let bytes = read_file_as_bytes(&file_path)?;
                let hash = hash_object(bytes, "blob")?;
                let mode = 0o100644; 
                let entry = IndexEntry::new(mode, hash, name.clone());
                index.add_entry(entry);
            }
        }
        else {
            return Err(Box::new(GitError::InvalidCommand("File name is required when using --add without --cacheinfo".to_string())));
        }
        index.write_to_file(&index_path)?;
        Ok(0)
    }
}




