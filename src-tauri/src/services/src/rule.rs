use crate::errors::Result;
use crate::errors::ServicesError;
use config::views::orignal_view::OrignalViews;
use config::Config;
use config::ConfigArc;
use config::ConfigVecWrapperTrait;
use config::errors::ConfigError;
use config::features::COEXISTS_CODE;
use config::rules::Rule;
use config::views::address_view::AddressView;
use config::views::config_view::ConfigViews;
use config::views::features_view::FeaturesView;
use config::views::files_view::FileView;
use config::views::files_view::FilesView;
use config::views::path_view::PathView;
use log::debug;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::Mutex;

pub static TEST_CONFIG: OnceLock<ConfigArc> = OnceLock::new();

pub async fn config_init(config: Config) -> Result<ConfigViews> {
    let init_views = ConfigViews::try_from(&config)?;
    match TEST_CONFIG.get() {
        None => {
            let config = Arc::new(Mutex::new(config));
            let _r = TEST_CONFIG.set(ConfigArc(config));
        }
        Some(config_arc) => {
            let mut guard = config_arc.0.lock().await;
            *guard = config;
        }
    }
    Ok(init_views)
}

async fn rule_config_fn<F, T>(f: F) -> Result<T>
where
    F: FnOnce(&mut Config) -> Result<T>,
{
    let config = TEST_CONFIG.get().ok_or(ServicesError::GetConfigError)?;
    let mut guard = config.0.lock().await;
    f(&mut *guard)
}

async fn rule_rule_fn<F, T>(code: &str, f: F) -> Result<T>
where
    F: FnOnce(&mut Rule) -> Result<T>,
{
    rule_config_fn(|config| {
        let rules = config.rules.get_mut(code)?;
        f(rules)
    })
    .await
}

async fn rule_file_fn<F, T>(code: &str, num: usize, f: F) -> Result<T>
where
    F: FnOnce(&mut Rule) -> Result<T>,
{
    rule_config_fn(|config| {
        let files = config.files.get_mut(code)?;
        let rule = files.rules.get_mut(num.to_string().as_str())?;
        f(rule)
    })
    .await
}

pub async fn rule_get_path(code: &str) -> Result<PathView> {
    rule_rule_fn(code, |rule| {
        rule.get_path()?;
        let r = PathView::from(&*rule);
        Ok(r)
    })
    .await
}

pub async fn rule_search_address(code: &str) -> Result<AddressView> {
    rule_rule_fn(code, |rule| {
        rule.search_address()?;
        let a = AddressView::from(&*rule);
        Ok(a)
    })
    .await
}

pub async fn rule_walk_files(code: &str) -> Result<FilesView> {
    let config = TEST_CONFIG.get().ok_or(ServicesError::GetConfigError)?;
    let mut guard = config.0.lock().await;
    let rules = guard.rules.get_mut(code)?;
    let files = rules.walk_files().await?;
    let view = FilesView::try_from(&files.rules)?;
    guard.files.push(files);
    Ok(view)
}

pub async fn rule_patch(code: &str, num: usize, fcode: &str, status: bool) -> Result<FeaturesView> {
    debug!(
        "正在 {}:{} 执行 {} 补丁,status:{}",
        code, num, fcode, status
    );
    rule_file_fn(code, num, |rule: &mut Rule| {
        rule.patch(fcode, status, None)?;
        let features_view = FeaturesView::from(&rule.features);
        Ok(features_view)
    })
    .await
}

pub async fn rule_make_coexist(code: &str, num: usize) -> Result<FileView> {
    rule_config_fn(|config| {
        let rule = config.rules.get_mut(code)?;
        let mut new_rule = rule.build_by_num(num)?;
        // 拷贝文件夹
        new_rule.patches.copy_directory()?;
        new_rule.patch(COEXISTS_CODE, true, None)?;
        new_rule.features.retain_features(num == 0);
        let files = config.files.get_mut(code)?;
        let file_view = FileView::try_from(&new_rule)?;
        files.rules.push(new_rule);
        Ok(file_view)
    })
    .await
}

pub async fn rule_del_coexist(code: &str, num: usize) -> Result<()> {
    rule_config_fn(|config| {
        let files = config.files.get_mut(code)?;
        let rule = files
            .rules
            .find(num.to_string().as_str())
            .ok_or(ConfigError::CoexistNumInvalid(num.to_string()))?;
        rule.del_coexist()?;
        files.rules.0.retain(|rule| rule.index != num);
        Ok(())
    })
    .await
}

pub async fn rule_read_orignal(code: &str, num: usize, fcode: &str) -> Result<OrignalViews>{
    rule_file_fn(code, num, |rule: &mut Rule| {
        let r = rule.read_orignal(fcode)?;
        Ok(r)
    })
    .await
}

pub async fn rule_patch_by_replace(code: &str, num: usize, fcode: &str, ovs: &OrignalViews) -> Result<()>{
    rule_file_fn(code, num, |rule: &mut Rule| {
        Ok(rule.patch_by_replace(fcode,ovs)?)
    })
    .await
}