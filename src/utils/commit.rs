use crate::utils:: {
    objtype::ObjType,
};

use std::{
    fmt,
    result,
    convert::TryFrom,
    error::Error,
    path::PathBuf
};

pub struct Commit;
impl ObjType for Commit {
    const VALUE: &'static str = "commit";
}

impl TryFrom<Vec<u8>> for Commit {
    type Error = Box<dyn Error>;

    fn try_from(_: Vec<u8>) -> result::Result<Self, Self::Error> {
        Ok(Commit {})
    }
}

impl Into<Vec<u8>> for Commit {
    fn into(self) -> Vec<u8> {
        let a = [];
        a.into()
    }
}

impl fmt::Display for Commit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Commit Object")
    }
}
