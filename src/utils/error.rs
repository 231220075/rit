use std::fmt;
use std::error::Error;
use std::path::Path;
use GitError::{InvalidCommand, FileNotFound, NoSubCommand};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub enum GitError {
    InvalidCommand(String),
    InvalidHash(String),
    InvalidObject(String),
    InvaildPathEncoding(String),
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

    pub fn file_notfound(msg: String) -> Box<dyn Error> {
        Box::new(
            Self::FileNotFound(msg)
        )
    }

    pub fn invalid_command(msg: String) -> Box<dyn Error> {
        Box::new(
            Self::InvalidCommand(msg.to_string())
        )
    }

    pub fn not_in_gitrepo() -> Box<dyn Error> {
        Box::new(
            Self::NotInGitRepo
        )
    }

    pub fn invalid_hash(hash: &str) -> Box<dyn Error> {
        Box::new(
            Self::InvalidHash(format!("expect hash code of length 40 but got {} of length {}", hash, hash.len()))
        )
    }

    pub fn invalid_object(path: &str) -> Box<dyn Error> {
        Box::new(
            Self::InvalidObject(format!("invlaid object format: {}", path))
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

    pub fn invaild_path_encoding(path: &str) -> Box<dyn Error>{
        Box::new(
            Self::InvaildPathEncoding(format!("invlaid path encoding: {}", path))
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
            GitError::InvalidObject(msg) => write!(f, "{}", msg),
            GitError::InvaildPathEncoding(path) => write!(f, "invalid path encoding: {}", path),
        }
    }
}

impl Error for GitError {}
