use crate::errors::Result;
use crate::types::wstr::WSTR;
use log::trace;
use thiserror::Error;
use windows::Win32::Storage::FileSystem::GetFileAttributesExW;
use windows::Win32::Storage::FileSystem::GetFileVersionInfoSizeW;
use windows::Win32::Storage::FileSystem::GetFileVersionInfoW;
use windows::Win32::Storage::FileSystem::VS_FIXEDFILEINFO;
use windows::Win32::Storage::FileSystem::VerQueryValueW;
use windows::Win32::Storage::FileSystem::WIN32_FILE_ATTRIBUTE_DATA;
use windows::core::BOOL;
use windows::core::w;

#[derive(Debug, Error)]
pub enum FileInfoError {
    #[error("获取文件信息失败")]
    GetFileInfoError,
}

pub struct FileInfo {
    path: String,
}

impl FileInfo {
    pub fn new<S: Into<String>>(path: S) -> Self {
        Self { path: path.into() }
    }

    pub fn get_size(&self) -> Result<u64> {
        let lpname = WSTR::new(Some(&self.path));
        let mut file_data = WIN32_FILE_ATTRIBUTE_DATA::default();
        unsafe {
            GetFileAttributesExW(
                lpname.to_pcwstr(),
                windows::Win32::Storage::FileSystem::GetFileExInfoStandard,
                &mut file_data as *mut _ as *mut _,
            )
        }
        .map_err(|_| FileInfoError::GetFileInfoError)?;

        Ok((file_data.nFileSizeHigh as u64) << 32 | (file_data.nFileSizeLow as u64))
    }

    pub fn get_version(&self) -> Result<String> {
        let lpname = WSTR::new(Some(&self.path));
        // 第一步：获取版本信息大小
        let mut dummy = 0;
        let info_size = unsafe { GetFileVersionInfoSizeW(lpname.to_pcwstr(), Some(&mut dummy)) };
        if info_size == 0 {
            trace!("{:?} get_version GetFileVersionInfoSizeW error: {:?}", self.path, info_size);
            return Err(FileInfoError::GetFileInfoError.into());
        }
        // 第二步：分配缓冲区并获取版本信息
        let mut buffer: Vec<u8> = vec![0; info_size as usize];
        let _ = unsafe {
            GetFileVersionInfoW(
                lpname.to_pcwstr(),
                Some(0),
                info_size,
                buffer.as_mut_ptr() as *mut _,
            )
        }
        .map_err(|e| {
            trace!("{:?} get_version GetFileVersionInfoW error: {:?}", self.path, e);
            FileInfoError::GetFileInfoError
           })?;
        // 第三步：查询固定文件信息
        let mut fixed_info_ptr = std::ptr::null_mut();
        let mut fixed_info_len = 0;
        let success = unsafe {
            VerQueryValueW(
                buffer.as_ptr() as *const _,
                w!("\\"),
                &mut fixed_info_ptr,
                &mut fixed_info_len,
            )
        };
        if success != BOOL(1) {
            trace!("{:?} get_version VerQueryValueW error: {:?}", self.path, success);
            return Err(FileInfoError::GetFileInfoError.into());
        }
        // 第四步：提取版本号
        let fixed_info = unsafe { &*(fixed_info_ptr as *const VS_FIXEDFILEINFO) };
        let major = (fixed_info.dwFileVersionMS >> 16) as u16;
        let minor = (fixed_info.dwFileVersionMS & 0xFFFF) as u16;
        let build = (fixed_info.dwFileVersionLS >> 16) as u16;
        let revision = (fixed_info.dwFileVersionLS & 0xFFFF) as u16;
        Ok(format!("{}.{}.{}.{}", major, minor, build, revision))
    }
}
