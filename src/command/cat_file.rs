use std::path::PathBuf;
use clap::{Parser, Subcommand};

use crate::utils::zlib::decompress_text;
use crate::{
    GitError,
    Result,
};
use super::SubCommand;


#[derive(Parser, Debug)]
#[command(name = "cat-file", about = "Provide contents or details of repository objects")]
pub struct CatFile {

    #[arg(short = 'p', group = "option", help = "pretty-print <object> content")]
    print: bool,

    #[arg(short = 'e', group = "option", help = "check if <object> exists")]
    exist: bool,

    #[arg(short = 't', group = "option", help = "show object type (one of 'blob', 'tree', 'commit', 'tag', ...)")]
    show_type: bool,

    #[arg(required = true, value_parser = obj_to_pathbuf)]
    objpath: PathBuf,
}

impl CatFile {
    pub fn from_args(mut args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        Ok(Box::new(CatFile::try_parse_from(args)?))
    }

    pub fn cat(&self) -> Result<String> {
        Ok(format!(""))
    }
}

fn obj_to_pathbuf(s:  &str) -> std::result::Result<PathBuf, String> {
    if s.len() != 40 {
        Err(format!("{} 长度不等于40，实际长度: {}", s, s.len()))
    }
    else {
        let (first, second) = s.split_at(2);
        // todo! <++> let git_path = find_git_path();
        let mut git_path = PathBuf::from(".git/objects");
        git_path.extend([first, second].iter());
        if !git_path.exists() {
            Err(format!("{} 不存在", git_path.to_str().unwrap()))
        }
        else {
            Ok(git_path)
        }
    }
}


impl SubCommand for CatFile {
    fn run(&self) -> Result<()> {
        let text = decompress_text(&self.objpath)?;
        if let Some(index) = text.find('\0') {
            print!("{}", &text[index + 1..]);
        }
        Ok(())

    }
}

