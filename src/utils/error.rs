use std::fmt;
use std::error::Error;
use std::path::Path;
use GitError::{InvalidCommand, FileNotFound, NoSubCommand};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, Clone)]
pub enum GitError {
    InvalidCommand(String),
    InvalidHash(String),
    InvalidBlob(String),
    InvalidFileMode(String),
    InvalidEntry(String),
    InvalidTree(String),
    InvalidObject(String),
    InvaildPathEncoding(String),
    FileNotFound(String),
    InvalidObj(String),
    NoSubCommand,
    NotInGitRepo,
}

impl GitError {
    pub fn invalid_obj(err: impl Error) -> Box::<dyn Error> {
        Box::new(
            Self::InvalidObj(err.to_string())
        )
    }
    pub fn invalid_tree(err: impl Error) -> Box::<dyn Error> {
        Box::new(
            Self::InvalidEntry(err.to_string())
        )
    }
    pub fn invalid_entry(err: impl Error) -> Box::<dyn Error> {
        Box::new(
            Self::InvalidEntry(err.to_string())
        )
    }
    pub fn invalid_filemode(mode_str: String) -> Box::<dyn Error> {
        Box::new(
            Self::InvalidFileMode(mode_str)
        )
    }
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

    pub fn invalid_blob(path: &str) -> Box<dyn Error> {
        Box::new(
            Self::InvalidBlob(format!("invlaid blob format: {}", path))
        )
    }

    pub fn invalid_hash(hash: &str) -> Box<dyn Error> {
        Box::new(
            Self::InvalidHash(format!("expect hash code of length 40 but got {} of length {}", hash, hash.len()))
        )
    }

    pub fn invaild_path_encoding(path: &str) -> Box<dyn Error>{
        Box::new(
            Self::InvaildPathEncoding(format!("invlaid path encoding: {}", path))
        )
    }

    pub fn invalid_object(path: &str) -> Box<dyn Error> {
        Box::new(
            Self::InvalidObject(format!("invlaid object format: {}", path))
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
            GitError::InvalidBlob(msg) => write!(f, "{}", msg),
            GitError::InvalidFileMode(mode) => write!(f, "Invalid FileMode {}", mode),
            GitError::InvalidEntry(msg) => write!(f, "Invalid Entry {}", msg),
            GitError::InvalidTree(msg) => write!(f, "Invalid Tree {}", msg),
            GitError::InvalidObj(msg) => write!(f, "Invalid Obj {}", msg),
            GitError::InvalidObject(msg) => write!(f, "{}", msg),
            GitError::InvaildPathEncoding(path) => write!(f, "invalid path encoding: {}", path),
        }
    }
}

impl Error for GitError {}
