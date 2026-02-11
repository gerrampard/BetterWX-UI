use crate::errors::Result;
use crate::rule::config_init;
use config::Config;
use config::update::Update;
use config::update::Updates;
use config::update::VerData;
use config::views::config_view::ConfigViews;
use log::debug;
use log::info;
use setting::DEBUG_BASE_PATH;
use setting::DEBUG_CONFIG_NAME;
use setting::DEBUG_README_NAME;
use setting::DEBUG_UPDATE_NAME;
use setting::MAIN_PKG_NAME;
use setting::UPDATE_URL;
use std::fs;
use std::path::Path;
use utils::http::Http;
use utils::store::Store;
use utils::store::StoreData;

pub async fn update_check() -> Result<Update> {
    let data = if setting::DEBUG_MODEL {
        get_debug_data(DEBUG_UPDATE_NAME)?
    } else {
        let http = Http::new(Some(MAIN_PKG_NAME.to_string()))?;
        let data = http.fetch(UPDATE_URL).await?;
        data.get_data()
    };
    let mut updates: Updates = serde_json::from_str(&data)?;
    let update: Update = updates.get_update()?;
    Ok(update)
}

pub async fn config_check(config: &VerData) -> Result<ConfigViews> {
    let data = if setting::DEBUG_MODEL {
        get_debug_data(DEBUG_CONFIG_NAME)?
    } else {
        let store = Store::new(&format!("config"))?;
        let password = config.version.to_string();
        match store.get_by_version(config.version.as_str()) {
            Ok(data) => {
                info!("正在使用本地配置文件：v{}", password);
                data
            }
            Err(e) => {
                debug!("从本地缓存获取配置文件失败 {}", e);
                info!("正在从网络获取配置文件");
                let http = Http::new(Some(password.clone()))?;
                let data = http.fetch(&config.data).await?;
                let store_data = StoreData::new(&password, &data.orignal, true);
                store.save(store_data)?;
                data.get_data()
            }
        }
    };
    let config: Config = serde_json::from_str(&data)?;
    let config_views = config_init(config).await?;
    Ok(config_views)
}

pub async fn readme_check(readme: &VerData) -> Result<String> {
    if setting::DEBUG_MODEL {
        let path = Path::new(DEBUG_BASE_PATH).join(DEBUG_README_NAME);
        let data = fs::read_to_string(path)?;
        return Ok(data);
    }
    let store = Store::new(&format!("readme"))?;
    let password = readme.version.to_string();
    let data = match store.get_by_version(readme.version.as_str()) {
        Ok(data) => {
            info!("正在使用本地说明文档：v{}", password);
            data
        }
        Err(e) => {
            debug!("从本地缓存获取说明文档失败 {}", e);
            info!("正在从网络获取说明文档");
            let http = Http::new(Some(password.clone()))?;
            let data = http.fetch(&readme.data).await?;
            let store_data = StoreData::new(&password, &data.orignal, true);
            store.save(store_data)?;
            data.get_data()
        }
    };
    Ok(data)
}

fn get_debug_data(name: &str) -> Result<String> {
    let path = Path::new(DEBUG_BASE_PATH).join(name);
    let data = fs::read_to_string(path)?;
    Ok(data)
}
