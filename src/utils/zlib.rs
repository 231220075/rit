use std::fs::File;
use std::path::Path;
use std::io::{self, Read, Write};
use flate2::read::ZlibDecoder;    // 导入 ZlibDecoder
use crate::{
    GitError,
    Result,
};

pub fn decompress<P: AsRef<Path>>(input_path: &P) -> Result<Vec<u8>> {
    let input_file = File::open(input_path)?;
    let mut decoder = ZlibDecoder::new(input_file);

    let mut buffer = Vec::new();
    decoder.read_to_end(&mut buffer)?;

    Ok(buffer)
}


pub fn decompress_text<P: AsRef<Path>>(input_path: &P) -> Result<String> {
    decompress(input_path)
        .and_then(
        |bytes|Ok(String::from_utf8(bytes)?))
}
