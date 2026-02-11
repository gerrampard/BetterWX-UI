use utils::store::Store;
use utils::store::StoreData;
use utils::version::Version;
use crate::errors::Result;

pub fn store_read(name:&str)->Result<String>{
    let version = Version::default();
    let store = Store::new(name)?;
    Ok(store.get_by_version(version.as_str())?)
}

pub fn store_save(name:&str,data:&str)->Result<()>{
    let store_data = StoreData::new(Version::default().as_str(), data, false);
    let store = Store::new(name)?;
    Ok(store.save(store_data)?)
}
