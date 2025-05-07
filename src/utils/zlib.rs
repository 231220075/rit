use std::fs::File;
use std::path::Path;
use std::io::{self, Read, Write, BufReader};
use flate2::{
    Compression,
    read::ZlibDecoder,
    bufread::ZlibEncoder,
};
use crate::{
    utils::{
        fs::read_file_as_bytes,
        objtype::ObjType,
    },
    GitError,
    Result,
};

pub fn decompress(data: Vec<u8>) -> Result<Vec<u8>>
{
    let mut decoder = ZlibDecoder::new(data.as_slice());

    let mut buffer = Vec::new();
    decoder.read_to_end(&mut buffer)?;

    Ok(buffer)
}


pub fn decompress_file<P>(input_path: &P) -> Result<String>
where P: AsRef<Path>
{
    read_file_as_bytes(input_path)
        .and_then(decompress)
        .and_then(
        |bytes|Ok(String::from_utf8(bytes)?))
}


pub fn compress<T>(data: T) -> Result<Vec<u8>>
where T: IntoIterator<Item=u8>
{
    let bytes = data.into_iter().collect::<Vec<_>>();
    let mut z = ZlibEncoder::new(BufReader::new(bytes.as_slice()), Compression::fast());
    let mut buffer = Vec::new();
    z.read_to_end(&mut buffer)?;

    Ok(buffer)
}

pub fn compress_file<P>(path: &P) -> Result<Vec<u8>>
where P: AsRef<Path>
{
    let bytes = read_file_as_bytes(path)?;
    compress(bytes)
}

pub fn compress_object<T: ObjType>(bytes: Vec<u8>) -> Result<Vec<u8>>
{
    let meta = format!("{} {}\0", T::VALUE, bytes.len()).into_bytes().into_iter();
    compress(meta.chain(bytes))
}
