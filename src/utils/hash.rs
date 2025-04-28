use std::{
    fmt,
    io::Read,
    fs::File,
    path::{PathBuf, Path}
};
use itertools::Itertools;
use sha1::{Sha1, Digest};

use crate::{
    utils::fs::read_file_as_bytes,
    GitError,
    Result,
};

fn sha_hash<T>(data: T) -> String
where T: IntoIterator<Item=u8> + Clone
{
    let mut hasher = Sha1::new();

    data.into_iter().chunks(1024)
        .into_iter()
        .map(|c| c.collect::<Vec<_>>())
        .for_each(|c| hasher.update(c));

    format!("{:x}", hasher.finalize())
}

fn hash_file<T>(path: &T) -> Result<String>
where T: AsRef<Path>
{
    read_file_as_bytes(path)
        .map(sha_hash)
}

pub fn hash_object(bytes: Vec<u8>) -> Result<String>
{
    let meta = format!("blob {}\0", bytes.len()).into_bytes().into_iter();
    Ok(sha_hash(meta.chain(bytes)))
}
