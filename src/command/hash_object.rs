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
    },
    hash::hash_object,
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
        Ok(hash_object(bytes, "blob")?)
    }
}


impl SubCommand for HashObject {
    /*  fn run(&self, gitdir: path) -> Result<i32>  */
    fn run(&self) -> Result<i32> {
        let bytes = read_file_as_bytes(&self.filepath)?;
        let path = self.hash(bytes.clone())?;

        if !self.write {
            println!("{}", path);
            Ok(0)
        }
        else {

            /*  dummy implementation!! should mkdir first  */
            let mut objpath = PathBuf::from("./.git/objects/");
            objpath.push(&path[..2]);
            objpath.push(&path[2..]);
            println!("write to {}", objpath.display());

            compress_object(bytes)
                .and_then(|b| Ok(write(objpath, b)?))
                .map(|_| 0)
        }
    }
}
