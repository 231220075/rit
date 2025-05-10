use std::{
    env::current_dir,
    io::{BufReader, Read},
    fs::{read, File},
    path::{PathBuf, Path},
};

use crate::{
    GitError,
    Result,
};

use super::{
    hash::hash_object,
    zlib::compress_object,
    objtype::ObjType,
};


/*  check the whether s exists in git's objects directory  */
pub fn obj_to_pathbuf(s: &str) -> std::result::Result<PathBuf, String> {
    if s.len() != 40 {
        Err(format!("{} 长度不等于40，实际长度: {}", s, s.len()))
    }
    else {
        let (first, second) = s.split_at(2);
        if let Ok(mut git_path) = get_git_dir() {
            git_path.push("objects");
            git_path.extend([first, second].iter());
            Ok(git_path)
        }
        else {
            Err("not find git repo".to_string())
        }
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


fn search_dir(mut path: PathBuf, target: &str) -> Result<PathBuf>
{
    path.push(target);
    if path.exists() && path.is_dir() {
        Ok(path)
    }
    else if !path.pop() || !path.pop() {
        Err(GitError::not_in_gitrepo())
    }
    else {
        search_dir(path, target)
    }
}

pub fn get_git_dir() -> Result<PathBuf> {
    search_dir(
        current_dir().unwrap(),
        ".git")
}

pub fn write_object<T: ObjType>(mut gitdir: PathBuf, content: Vec<u8>) -> Result<String> {
    let commit_hash = hash_object::<T>(content.clone())?;

    gitdir.extend(["objects", &commit_hash[0..2], &commit_hash[2..]]);

    println!("gitdir = {}", gitdir.display());
    std::fs::create_dir_all(gitdir.parent().unwrap())?;
    std::fs::write(
        &gitdir,
    compress_object::<T>(content)?)?;

    Ok(commit_hash)
}
