use std::fmt;
use std::error::Error;
use GitError::{InvalidCommand, FileNotFound, NoSubCommand};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub enum GitError {
    InvalidCommand(String),
    InvalidHash(String),
    FileNotFound(String),
    NoSubCommand,
    NotInGitRepo,
}

impl GitError {
    pub fn no_subcommand() -> Box::<dyn Error> {
        Box::new(
            Self::NoSubCommand
        )
    }

    pub fn invalid_command() -> Box<dyn Error> {
        Box::new(
            Self::InvalidCommand("invalid command!".to_string())
        )
    }

    pub fn file_notfound() -> Box<dyn Error> {
        Box::new(
            Self::FileNotFound("file not found".to_string())
        )
    }

    pub fn new_invalid_command(msg: String) -> Box<dyn Error> {
        Box::new(
            Self::InvalidCommand(msg.to_string())
        )
    }

    pub fn new_file_notfound(msg: String) -> Box<dyn Error> {
        Box::new(
            Self::FileNotFound(msg.to_string())
        )
    }

    pub fn new_not_in_gitrepo() -> Box<dyn Error> {
        Box::new(
            Self::NotInGitRepo
        )
    }

    pub fn new_invalid_hash(hash: &str) -> Box<dyn Error> {
        Box::new(
            Self::InvalidHash(format!("expect hash code of length 40 but got {} of length {}", hash, hash.len()))
        )
    }
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvalidCommand(cmd) => write!(f, "Invalid command: {}", cmd),
            FileNotFound(file)  => write!(f, "File not found: {}",  file),
            GitError::InvalidHash(hash) => write!(f, "Invalid hash: {}", hash),
            GitError::NoSubCommand => write!(f, "no sub command"),
            GitError::NotInGitRepo => write!(f, "not in a git repository"),
        }
    }
}

impl Error for GitError {}
