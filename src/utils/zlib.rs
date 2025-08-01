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

pub fn decompress_file_as_bytes<P>(input_path: &P) -> Result<Vec<u8>>
where P: AsRef<Path>
{
    read_file_as_bytes(input_path)
        .and_then(decompress)

}

pub fn decompress_file<P>(input_path: &P) -> Result<String>
where P: AsRef<Path>
{
    decompress_file_as_bytes(input_path)
        .and_then(
        |bytes|Ok(String::from_utf8(bytes)?))

}

pub fn decompress_file_bytes<P>(input_path: &P) -> Result<Vec<u8>>
where P: AsRef<Path>
{
    read_file_as_bytes(input_path)
        .and_then(decompress)
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
    // println!("compress_file : {}", path.as_ref().display());

    let bytes = read_file_as_bytes(path)?;
    // println!("{}, {}", bytes.len(), path.as_ref().display());
    compress(bytes)
}

pub fn compress_object<T: ObjType>(bytes: Vec<u8>) -> Result<Vec<u8>>
{
    let meta = format!("{} {}\0", T::VALUE, bytes.len()).into_bytes().into_iter();
    compress(meta.chain(bytes))
}

/// 解压缩对象数据（从文件内容）
pub fn decompress_object(compressed_data: &[u8]) -> Result<Vec<u8>> {
    decompress(compressed_data.to_vec())
}
