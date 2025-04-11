use std::fmt;
use std::error::Error;
use GitError::{InvalidCommand, FileNotFound, NoSubCommand};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub enum GitError {
    InvalidCommand(String),
    FileNotFound(String),
    NoSubCommand,
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
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvalidCommand(cmd) => write!(f, "Invalid command: {}", cmd),
            FileNotFound(file)  => write!(f, "File not found: {}",  file),
            NoSubCommand => write!(f, "no sub command"),
        }
    }
}

impl Error for GitError {}
