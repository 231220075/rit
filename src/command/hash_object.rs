use std::fs::write;
use std::path::{
    Path,
    PathBuf
};
use clap::{Parser, Subcommand};

use crate::utils::{
    zlib::{
        decompress_file,
        compress_object
    },
    fs::{
        obj_to_pathbuf,
        read_file_as_bytes,
        write_object,
    },
    hash::hash_object,
    objtype::Blob,
};

use crate::{
    GitError,
    Result,
};
use super::SubCommand;


#[derive(Parser, Debug)]
#[command(name = "hash-object", about = "Compute object ID and optionally create an object from a file")]
pub struct HashObject {

    #[arg(required = false, short = 'w', group = "option", help = "Actually write the object into the object database.")]
    write: bool,

    #[arg(required = true)]
    filepath: PathBuf,
}

impl HashObject {
    pub fn from_args(mut args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        Ok(Box::new(HashObject::try_parse_from(args)?))
    }

    pub fn hash(&self, bytes: Vec<u8>) -> Result<String> {
        hash_object::<Blob>(bytes)
    }
}


impl SubCommand for HashObject {
    /*  fn run(&self, gitdir: path) -> Result<i32>  */
    fn run(&self, gitdir: Result<PathBuf>) -> Result<i32> {
        let bytes = read_file_as_bytes(&self.filepath)?;
        let path = self.hash(bytes.clone())?;
        let mut gitdir = gitdir?;

        if !self.write {
            println!("{}", path);
            Ok(0)
        }
        else {

            println!("write to {}", gitdir.clone().display());
            write_object::<Blob>(gitdir, bytes)?;
            Ok(0)
        }
    }
}
