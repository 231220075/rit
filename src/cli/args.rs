use crate::utils::error::GitError; 
use std::env;
pub struct Args {
    pub command: String,
    pub files: Vec<String>,
    pub message: String,
}


pub fn get_args() ->Args{
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(||{
        let error = GitError::InvalidCommand("No command provided".to_string());
        eprintln!("{}", error);
        std::process::exit(1);
    });

    let mut files = Vec::new();
    let mut message = String::new();
    while let Some(arg) = args.next(){
        if arg == "--message" || arg == "-m" {
            message = args.next().unwrap_or_else(|| {
                let error = GitError::InvalidCommand("No message provided".to_string());
                eprintln!("{}", error);
                std::process::exit(1);
            })
        } else {
            files.push(arg);
        }

    }

    Args {
        command,
        files,
        message,
    }
}