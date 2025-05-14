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
    objtype::{
        ObjType,
        Obj,
    },
    blob::Blob,
    tree::Tree,
    commit::Commit,
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
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
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
        let gitdir = gitdir?;

        if self.write {
            write_object::<Blob>(gitdir, bytes)?;
            Ok(0)
        }
        else {
            println!("{}", path);
            Ok(0)
        }
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::{
        test::{
            shell_spawn,
            setup_test_git_dir,
            mktemp_in,
        },
    };

    #[test]
    fn test_blob() {
        let temp = setup_test_git_dir();
        let temp_path = temp.path();
        let temp_path_str = temp_path.to_str().unwrap();

        let file1 = mktemp_in(&temp).unwrap();
        let file1_str = file1.to_str().unwrap();

        let origin = shell_spawn(&["git", "-C", temp_path_str, "hash-object", file1_str]).unwrap();
        let real = shell_spawn(&["cargo", "run", "--quiet", "--", "-C", temp_path_str, "hash-object", file1_str]).unwrap();

        assert_eq!(origin, real);
    }
}
