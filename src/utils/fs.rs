use std::{
    io::{BufReader, Read},
    fs::{read, File},
    path::{PathBuf, Path}
};

use crate::{
    GitError,
    Result,
};

/*  check the whether s exists in git's objects directory  */
pub fn obj_to_pathbuf<T>(s: &str) -> std::result::Result<PathBuf, String> {
    if s.len() != 40 {
        Err(format!("{} 长度不等于40，实际长度: {}", s, s.len()))
    }
    else {
        let (first, second) = s.split_at(2);
        // todo! <++> let git_path = find_git_path();
        let mut git_path = PathBuf::from(".git/objects");
        git_path.extend([first, second].iter());
        Ok(git_path)
    }
}

pub fn read_file_as_bytes<T>(file_path: &T) -> Result<Vec<u8>>
where T: AsRef<Path>
{
    // 使用 fs::read 读取文件内容为字节数组
    Ok(read(file_path)?)
}

pub fn read_file_as_reader<T>(file_path: &T) -> Result<impl Read>
where T: AsRef<Path>
{
    let file = File::open(file_path)?;
    Ok(BufReader::new(file))
}
