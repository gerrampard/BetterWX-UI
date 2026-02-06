use crate::errors::Result;
use crate::version::Version;
use core::result::Result::Ok;
use std::fs::remove_dir_all;
use fs_extra::dir::CopyOptions;
use fs_extra::dir::copy as copy_dir;
use std::fs::copy;
use std::path::Path;
use thiserror::Error;
use winsys::fileinfo::FileInfo;

#[derive(Debug, Error)]
pub enum FileError {
    #[error("文件不存在：{0}")]
    FileNotExists(String),

    #[error("文件无效：{0}")]
    FileInvalidError(String),
    
    #[error("拷贝文件夹失败, 错误: {0}")]
    CopyDirError(String),

    #[error("删除文件夹失败, 错误: {0}")]
    DelDirError(String),
}

pub fn check_file_exists(file: &str) -> Result<()> {
    if file.is_empty() || !Path::new(file).exists() {
        return Err(FileError::FileNotExists(file.to_string()).into());
    }
    Ok(())
}

pub fn get_file_name(file: &str) -> Result<String> {
    let file_name = Path::new(file)
        .file_name()
        .ok_or(FileError::FileInvalidError(file.to_string()))?;
    let file_name = file_name.to_string_lossy().to_string();
    Ok(file_name)
}

pub fn remove_file(file: &str) -> Result<()> {
    if Path::new(file).exists() {
        std::fs::remove_file(file)?;
    }
    Ok(())
}

pub fn back_file(from: &str, to: &str) -> Result<()> {
    if !Path::new(to).exists() {
        copy(from, to)?;
    } else {
        if !file_is_equal(from, to)? {
            copy(from, to)?;
        }
    }
    Ok(())
}

pub fn copy_directory(from: &str, to: &str) -> Result<()> {
    let options = CopyOptions::new()
        .overwrite(true) // 覆盖已存在的文件
        .skip_exist(true) // 不跳过已存在的文件
        .copy_inside(true); // 复制文件夹内容到目标文件夹内
    // 复制文件夹
    copy_dir(from, to, &options)
        .map_err(|e| FileError::CopyDirError(e.to_string()))?;

    Ok(())
}

pub fn del_directory(to: &str) -> Result<()> {
    remove_dir_all(to)
        .map_err(|e| FileError::DelDirError(e.to_string()))?;

    Ok(())
}


pub fn file_is_equal(from: &str, to: &str) -> Result<bool> {
    let from_path = Path::new(from);
    let to_path = Path::new(to);
    if !from_path.exists() || !to_path.exists() {
        return Ok(false);
    }
    let from_file = &FileInfo::new(from);
    let to_file = &FileInfo::new(to);
    // 修改为使用文件大小比较
    file_is_equal_by_size(from_file, to_file)
}

fn _file_is_equal_by_version(from: &FileInfo, to: &FileInfo) -> Result<bool> {
    let from_ver = Version::new(from.get_version()?.as_str());
    let to_ver = Version::new(to.get_version()?.as_str());
    let from_size = from.get_size()?;
    let to_size = to.get_size()?;
    Ok(to_ver == from_ver && to_size == from_size)
}

fn file_is_equal_by_size(from: &FileInfo, to: &FileInfo) -> Result<bool> {
    let from_size = from.get_size()?;
    let to_size = to.get_size()?;
    Ok(to_size == from_size)
}
