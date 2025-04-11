use std::fmt;
#[derive(Debug)]
pub enum GitError {
    InvalidCommand(String),
    FileNotFound(String),
}
impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitError::InvalidCommand(cmd) => write!(f, "Invalid command: {}", cmd),
            GitError::FileNotFound(file) => write!(f, "File not found: {}", file),
        }
    }
}