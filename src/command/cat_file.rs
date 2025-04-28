use std::path::PathBuf;
use clap::{Parser, Subcommand};

use crate::{
    utils::{
        zlib::decompress_file,
        fs::obj_to_pathbuf,
    },
};
use crate::{
    GitError,
    Result,
};
use super::SubCommand;


#[derive(Parser, Debug)]
#[command(name = "cat-file", about = "Provide contents or details of repository objects")]
pub struct CatFile {

    #[arg(required = true, short = 'p', group = "option", help = "pretty-print <object> content")]
    print: bool,

    #[arg(required = true, short = 'e', group = "option", help = "check if <object> exists")]
    check_exist: bool,

    #[arg(short = 't', group = "option", help = "show object type (one of 'blob', 'tree', 'commit', 'tag', ...)")]
    show_type: bool,

    #[arg(required = true, value_parser = obj_to_pathbuf::<PathBuf>)]
    objpath: PathBuf,
}

impl CatFile {
    pub fn from_args(mut args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        Ok(Box::new(CatFile::try_parse_from(args)?))
    }

    pub fn cat(&self) -> Result<()> {
        let mut text = decompress_file(&self.objpath)?;
        let index = text.find('\0').expect("decompress_text 实现错误，返回对象不符合");
        print!("{}", &text[index + 1..]);
        Ok(())
    }
}


impl SubCommand for CatFile {
    fn run(&self) -> Result<i32> {
        if !self.objpath.exists()
        {
            if self.check_exist {
                return Ok(1);
            }
            else {
                return Err(GitError::new_file_notfound(format!("{} 不存在", self.objpath.to_str().unwrap())));
            }
        }
        else if self.print {
            self.cat();
        }
        Ok(0)
    }
}

