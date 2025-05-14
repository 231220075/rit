use std::{
    io::{
        self,
        Write,
    },
    result,
    fmt,
    convert::TryFrom,
    error::Error,
    path::PathBuf
};

use crate::utils::{
    objtype::{
        ObjType,
        parse_meta,
    },
    error::{
        GitError,
        Result
    },
};


#[derive(Clone)]
pub struct Blob(Vec<u8>);
impl ObjType for Blob {
    const VALUE: &'static str = "blob";
}

impl TryFrom<Vec<u8>> for Blob {
    type Error = Box<dyn Error>;

    fn try_from(bytes: Vec<u8>) -> result::Result<Self, Self::Error> {
        let _ = parse_meta(&bytes).map_err(GitError::invalid_obj)?;
        let index = bytes.iter().position(|&b| b == b'\0').expect("assert parse_meta implementation fails");
        Ok(Blob(bytes[index + 1..].to_vec()))
    }
}

impl Into<Vec<u8>> for Blob {
    fn into(self) -> Vec<u8> {
        self.0
    }
}

impl fmt::Display for Blob {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes: Vec<u8> = self.clone().into();
        let _ = io::stdout().write_all(&bytes);
        Ok(())
    }
}
