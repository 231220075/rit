use crate::utils:: {
    objtype::{
        ObjType,
        parse_meta,
    },
    error::{
        GitError,
        Result
    },
};

use std::{
    fmt,
    result,
    convert::TryFrom,
    error::Error,
    path::PathBuf
};

pub struct Commit(Vec<u8>);
impl ObjType for Commit {
    const VALUE: &'static str = "commit";
    const MODE: u32 = 0o160000;
}

impl TryFrom<Vec<u8>> for Commit {
    type Error = Box<dyn Error>;

    fn try_from(bytes: Vec<u8>) -> result::Result<Self, Self::Error> {
        let (rest, _) = parse_meta(&bytes)
            .map_err(|e|GitError::invalid_commit(&e.to_string()))?;
        Ok(Commit(rest.to_vec()))
    }
}

impl From<Commit> for Vec<u8> {
    fn from(commit: Commit) -> Vec<u8> {
        commit.0
    }
}

impl fmt::Display for Commit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from_utf8(self.0.clone()).expect("assert write-tree implementation failed"))
    }
}
