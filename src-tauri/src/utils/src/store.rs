use crate::base64::Base64;
use crate::errors::Result;
use known_folders::KnownFolder;
use known_folders::get_known_folder_path;
use log::error;
use serde::Deserialize;
use serde::Serialize;
use setting::MAIN_PKG_NAME;
use std::env::current_dir;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {

    #[error("序列化错误: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("无法获取数据目录")]
    GetDataFolderError,

    #[error("版本不匹配")]
    VersionError,

    #[error("读写锁被污染")]
    LockPoisoned,
}

#[derive(Default)]
pub struct Store {
    data: Arc<RwLock<StoreData>>,
    file_path: PathBuf,
}

impl Store {
    pub fn new(name: &str) -> Result<Self> {
        let roaming_dir = match get_known_folder_path(KnownFolder::RoamingAppData) {
            Some(dir) => dir,
            None => current_dir().map_err(|_| StoreError::GetDataFolderError)?,
        };

        let app_dir = roaming_dir.join(MAIN_PKG_NAME);
        fs::create_dir_all(&app_dir)?;
        let file_path = app_dir.join(format!("{}.data", name));
        let mut store = Self::default();
        store.file_path = file_path.clone();
        if !file_path.exists() {
            return Ok(store);
        }
        let contents = fs::read_to_string(&file_path)?;
        if contents.is_empty() {
            return Ok(store);
        }
        let store_data: StoreData = serde_json::from_str(&contents).map_err(|e| {
            error!("从本地缓存获取数据解析失败: {:?}", e);
            StoreError::SerializationError(e)
        })?;
        store.data = Arc::new(RwLock::new(store_data));
        return Ok(store);
    }

    pub fn get(&self) -> Result<String> {
        let data = self.data.read().map_err(|_| StoreError::LockPoisoned)?;
        if !data.encoded {
            return Ok(data.data.clone());
        }
        let password = data.version.to_string();
        let base64 = Base64::new(password.as_str())?;
        let decode_text = base64.decode(&data.data)?;
        Ok(decode_text)
    }

    pub fn get_by_version(&self, version: &str) -> Result<String> {
        let data = self.data.read().map_err(|_| StoreError::LockPoisoned)?;
        if data.version != version {
            return Err(StoreError::VersionError.into());
        }
        if !data.encoded {
            return Ok(data.data.clone());
        }
        let password = data.version.to_string();
        let base64 = Base64::new(password.as_str())?;
        let decode_text = base64.decode(&data.data)?;
        Ok(decode_text)
    }

    pub fn save(&self, mut data: StoreData) -> Result<()> {
        if !data.encoded {
            let password = data.version.to_string();
            let base64 = Base64::new(password.as_str())?;
            let encode_text = base64.encode(&data.data);
            data.data = encode_text;
            data.encoded = true;
        }
        let json = serde_json::to_string(&data).map_err(|e| StoreError::SerializationError(e))?;
        fs::write(&self.file_path, json)?;
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StoreData {
    pub version: String,
    pub data: String,
    pub encoded: bool,
}

impl StoreData {
    pub fn new(version: &str, data: &str, encoded: bool) -> Self {
        Self {
            version: version.into(),
            data: data.to_string(),
            encoded,
        }
    }
}
