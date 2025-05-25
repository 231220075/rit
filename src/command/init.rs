use std::fs;
use std::path::PathBuf;
use std::convert::Into;
use std::env::current_dir;
use clap::{Parser, Subcommand, CommandFactory};

use crate::utils::{
    zlib::{
        decompress_file,
        decompress_file_as_bytes,
    },
    fs::obj_to_pathbuf,
    objtype::{
        ObjType,
        parse_meta,
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

use nom::{
    bytes::complete::{tag, take, take_until},
    number::complete::be_u32,
    IResult,
};


#[derive(Parser, Debug)]
#[command(name = "init", about = "Create an empty Git repository or reinitialize an existing one")]
pub struct Init {
    #[arg(help = "directory to be initialized")]
    pub dir: Option<String>
}

impl Init {
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        Ok(Box::new(Init::try_parse_from(args)?))
    }
}

impl SubCommand for Init {
    fn run(&self, _: Result<PathBuf>) -> Result<i32> {
        let curr_path = current_dir()?;
        let gitdir = if self.dir.is_some() {
            curr_path.join(self.dir.clone().unwrap())
        }
        else {
            curr_path
        }.join(".git");

        let refs = gitdir.join("refs");
        fs::create_dir_all(&refs)?;
        fs::create_dir_all(refs.join("heads"))?;
        fs::create_dir_all(refs.join("tags"))?;
        fs::create_dir_all(refs.join("remote"))?;

        let objects = gitdir.join("objects");
        fs::create_dir_all(&objects)?;
        fs::create_dir_all(objects.join("info"))?;
        fs::create_dir_all(objects.join("pack"))?;

        std::fs::write( gitdir.join("HEAD"), b"ref: refs/heads/master")?;
        Ok(0)
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
    fn test_basic() {
        let temp = setup_test_git_dir();
        let temp_path = temp.path();
        let temp_path_str = temp_path.to_str().unwrap();

        let file1 = mktemp_in(&temp).unwrap();
        let file1_str = file1.to_str().unwrap();

        let _ = shell_spawn(&["ls", "-lahR", temp_path_str]).unwrap();

        let _ = shell_spawn(&["git", "-C", temp_path_str, "add", &file1_str]).unwrap();
        let _ = shell_spawn(&["git", "-C", temp_path_str, "hash-object", file1_str]).unwrap();

        let _ = shell_spawn(&["cargo", "run", "--quiet", "--", "init", temp_path_str]).unwrap();
        let _ = shell_spawn(&["git", "-C", temp_path_str, "status"]).unwrap();
    }
}
