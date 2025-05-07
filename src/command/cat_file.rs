use std::path::PathBuf;
use clap::{Parser, Subcommand, CommandFactory};

use crate::utils::{
        zlib::decompress_file,
        fs::obj_to_pathbuf,
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

    #[arg(required = true, value_parser = obj_to_pathbuf)]
    objpath: PathBuf,
}

impl CatFile {
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Box<dyn SubCommand>> {
        Ok(Box::new(CatFile::try_parse_from(args)?))
    }

    pub fn cat(&self, gitdir: PathBuf) -> Result<()> {
        let text = decompress_file(&gitdir)?;
        let index = text.find('\0').ok_or(GitError::invalid_object(&gitdir.to_string_lossy()))?;
        print!("{}", &text[index + 1..]);
        Ok(())
    }

    pub fn cat_type(&self, gitdir: PathBuf) -> Result<()> {
        let text = decompress_file(&gitdir)?;
        let index = text.find(' ').ok_or(GitError::invalid_object(&gitdir.to_string_lossy()))?;
        println!("{}", &text[..index]);
        Ok(())
    }
}


impl SubCommand for CatFile {
    fn run(&self, gitdir: Result<PathBuf>) -> Result<i32> {
        let mut gitdir = gitdir?;
        gitdir.push(&self.objpath);
        if !gitdir.exists()
        {
            if self.check_exist {
                Ok((!gitdir.exists()) as i32)
            }
            else {
                Err(GitError::file_notfound(format!("{} 不存在", gitdir.to_str().unwrap())))
            }
        }
        else if self.print {
            self.cat(gitdir)?;
            Ok(0)
        }
        else if self.show_type {
            self.cat_type(gitdir)?;
            Ok(0)
        }
        else {
            let mut cmd = CatFile::command(); // 获取底层的 Command 对象
            let _ = cmd.print_help();     // 打印帮助信息
            Ok(0)
        }
    }
}

