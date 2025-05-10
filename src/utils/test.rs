use std::{
    mem,
    io::{
        self,
        Write,
    },
    fs,
    path::{
        Path,
        PathBuf
    },
    process::Command
};
use tempfile::{
    tempdir,
    Builder,
};
use itertools::Itertools;

pub fn shell_spawn(command_list: &[&str]) -> Result<String,String> {
    let command = command_list[0];
    // 创建 Command 实例并运行命令
    let output = Command::new(command)
        .args(&command_list[1..])
        .output()
        .map_err(|e| format!("Failed to execute command '{}': {}", command, e))?;

    // 检查命令的退出状态
    if !output.status.success() {
        Err(format!(
            "Command '{}' failed with exit code: {:?}, output: {}",
            command_list.into_iter().join(" "),
            output.status.code(),
            String::from_utf8(output.stderr).unwrap() + & String::from_utf8(output.stdout).unwrap(),
        ))
    }
    else {
        // 将 stdout 转换为 String
        Ok(String::from_utf8(output.stderr).unwrap() + &String::from_utf8(output.stdout).unwrap())
    }
}

pub fn setup_test_git_dir() -> tempfile::TempDir {
    let temp_dir = tempdir().unwrap();
    let git_dir = temp_dir.path().join(".git");
    fs::create_dir_all(git_dir.join("objects")).unwrap();
    let _ = shell_spawn(&["git", "-C", temp_dir.path().to_str().unwrap(), "init"]).unwrap();
    temp_dir
}


pub fn mktemp_in<T>(dir: T) -> std::io::Result<PathBuf>
where T: AsRef<Path>
{
    // 指定目录路径
    let dir_path = dir;

    // 使用 Builder 创建临时文件
    let temp_file = Builder::new()
        .prefix("temp_") // 可选：为文件名添加前缀
        .suffix(".txt")  // 可选：为文件名添加后缀
        .rand_bytes(8)   // 随机字节数（默认是 6 字节）
        .tempfile_in(dir_path)?; // 在指定目录中创建临时文件

    // 获取临时文件的路径
    let file_path = temp_file.keep();

    Ok(file_path.unwrap().1)
}
