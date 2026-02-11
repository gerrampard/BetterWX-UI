use crate::ConfigVecWrapperTrait;
use crate::cache::Cache;
use crate::convert_num;
use crate::dfetures::DFeatures;
use crate::errors::ConfigError;
use crate::errors::Result;
use crate::features::COEXISTS_CODE;
use crate::features::Features;
use crate::files::FileRules;
use crate::patches::Patches;
use crate::paths::Paths;
use crate::serders::skippers::skip_if_empty;
use crate::variables::ISMAIN_CODE;
use crate::variables::NUM_CODE;
use crate::variables::NUM_HEX_CODE;
use crate::variables::Variables;
use crate::views::orignal_view::OrignalViews;
use log::debug;
use log::error;
use log::info;
use log::trace;
use macros::FieldDescGetters;
use macros::FieldNameGetters;
use macros::ImpConfigVecIsEmptyTrait;
use macros::ImpConfigVecWrapperTrait;
use macros::SortedSerializeByIndex;
use serde::Deserialize;
use serde::Serialize;
use serde_repr::Deserialize_repr;
use serde_repr::Serialize_repr;
use tokio::task::JoinSet;
use utils::patch::types::Bytes;

#[derive(
    Debug, Default, ImpConfigVecIsEmptyTrait, ImpConfigVecWrapperTrait, SortedSerializeByIndex,
)]
pub struct Rules(pub Vec<Rule>);

#[derive(Debug, Clone, Serialize, Deserialize, FieldDescGetters, FieldNameGetters)]
pub struct Rule {
    pub code: String,
    pub index: usize,
    pub version: String,
    pub patches: Patches,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub extpatches: Vec<String>, // 全局额外补丁依赖
    #[serde(default)]
    pub rtype: RuleType,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub ismain: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub name: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub news: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub description: String, // 规则描述
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub disabled: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub supported: bool, // 搜索后使用 是否支持
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub patched: bool, // 搜索后使用 是否支持
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub installed: bool, // 查询路径后使用 是否安装
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub paths: Paths, // 路径配置
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub variables: Variables, // 变量配置
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub features: Features, // 功能配置
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub dfeatures: DFeatures, // 默认功能配置
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub hfeatures: Features, // 头部功能配置
}

/// init
impl Rule {
    pub fn get_path(&mut self) -> Result<&Self> {
        self.check_is_config_type()?;
        info!("正在获取 {} 安装位置...", self.get_name());
        self.init_path()?
            // 二次替换，确保全部变量替换完成
            .init_variables()?
            .init_variables()?
            .init_patches()?
            .init_dfeatures()?
            .init_features()?;
        self.rtype = RuleType::Pathed;
        info!("配置规则 {} 初始化完成", self.get_name());
        Ok(self)
    }

    pub fn search_address(&mut self) -> Result<()> {
        info!("正在搜索 {} 基址...", self.get_name());
        self.check_is_pathed_type()?;
        if !self.installed {
            return Err(ConfigError::NotInstalled(self.get_name().to_string()).into());
        }

        let mut rule = self.build_by_num(10)?;

        rule.patches.back_files()?;

        rule.patches.check_files_and_del(true, true)?;
        let mut cache = Cache::new();
        rule.patches.search(&mut cache, self.get_name(), &rule.code, &rule.version)?;
        self.patches.clone_pattern(&rule.patches)?;
        let pvariables = Variables::try_from(&self.patches)?;
        self.variables.extend(pvariables);
        self.rtype = RuleType::Search;
        self.supported = self.patches.is_supported();
        self.patched = self.patches.is_patched();
        // 清除头部功能，在搜搜完地址后应该保存
        // 清理无用字段
        self.name = String::default();
        self.description = String::default();
        self.news = String::default();
        self.hfeatures.clear();
        self.dfeatures = DFeatures::default();
        self.features
            .init_features(&self.variables, &self.patches)?;
        info!("搜索 {} 基址完成...", self.get_name());
        Ok(())
    }

    fn init_path(&mut self) -> Result<&mut Self> {
        match self.paths.init() {
            Ok(path_variables) => {
                self.installed = true;
                self.variables.extend(path_variables);
            }
            Err(_) => {
                return Err(ConfigError::NotInstalled(self.get_name().to_string()).into());
            }
        }
        Ok(self)
    }

    fn init_variables(&mut self) -> Result<&mut Self> {
        self.variables.init()?;
        trace!("替换后的变量为：\n{:?}", self.variables);
        Ok(self)
    }

    fn init_patches(&mut self) -> Result<&mut Self> {
        let version = self.variables.get_install_version()?;
        let variables = &self.variables;
        match self.patches.init(variables) {
            Ok(_) => {
                trace!(
                    "获取 {} 版本：{} 特征码成功\n{:?}",
                    self.get_name(),
                    version,
                    self.patches
                );
            }
            Err(e) => {
                error!(
                    "获取 {} 版本：{} 特征码失败，{}",
                    self.get_name(),
                    version,
                    e
                );
                return Err(e);
            }
        }
        Ok(self)
    }

    fn init_dfeatures(&mut self) -> Result<&mut Self> {
        match self.dfeatures.init() {
            Ok(features) => {
                self.features.extend(features);
                let mut features = std::mem::take(&mut self.features);
                self.hfeatures = features.take_inhead();
                self.features = features;
                trace!(
                    "构建 {} 默认功能成功\n头部功能：\n{:?} ================\n 文件功能：  \n{:?} ",
                    self.get_name(),
                    self.hfeatures,
                    self.features
                );
            }
            Err(e) => {
                error!("构建 {} 默认功能失败，{:?}", self.get_name(), e);
                return Err(e);
            }
        }
        Ok(self)
    }

    fn init_features(&mut self) -> Result<&mut Self> {
        self.hfeatures
            .init_hfeatures(&self.variables, &self.patches)?;
        self.features
            .init_features(&self.variables, &self.patches)?;
        Ok(self)
    }
}

/// build
impl Rule {
    pub async fn walk_files(&self) -> Result<FileRules> {
        info!("正在检测 {} 共存文件...", self.get_name());
        self.check_is_search_type()?;
        let mut rules = Rules::default();
        let mut tasks = JoinSet::new();
        let has_coexists = self.features.get(COEXISTS_CODE).is_ok();
        let max_num = if has_coexists { 10 } else { 1 };
        // 启动所有异步任务
        for num in 0..max_num {
            let rule = self.clone();
            tasks.spawn(async move {
                let (_, name) = convert_num(num);
                match rule.build_by_num(num) {
                    Ok(mut new_rule) => match new_rule.check_files_and_del(true, false) {
                        Ok(_) => {
                            new_rule.features.retain_features(num == 0);
                            new_rule.set_patched(None)?;
                            debug!("检测到 {} {} 文件", rule.get_name(), name);
                            Ok(Some(new_rule))
                        }
                        Err(e) => {
                            debug!("检测 {} {} 文件失败，{}", rule.get_name(), name, e);
                            Ok(None)
                        }
                    },
                    Err(e) => Err(e),
                }
            });
        }

        // 收集所有任务结果
        while let Some(res) = tasks.join_next().await {
            match res? {
                Ok(Some(rule)) => rules.push(rule),
                Ok(None) => {}
                Err(e) => return Err(e),
            }
        }

        rules.0.sort_by(|a, b| a.index.cmp(&b.index));
        info!("共检测到 {} 个共存程序", rules.len());

        Ok(FileRules {
            code: self.code.clone(),
            rules,
        })
    }

    pub fn build_by_num(&self, num: usize) -> Result<Self> {
        if num > 10 {
            return Err(ConfigError::InvalidCoexistNum(num.to_string()).into());
        }

        match num {
            10 => self.check_is_pathed_type()?,
            _ => self.check_is_search_type()?,
        }

        let b = Bytes::new(num.to_string()).to_hex();
        let (num_hex, ismain) = if num == 0 || num == 10 {
            ("??", true)
        } else {
            (b.as_str(), false)
        };
        let mut rule = self.clone();
        rule.variables.set_value(ISMAIN_CODE, ismain);
        rule.variables.set_value(NUM_CODE, num);
        rule.variables.set_value(NUM_HEX_CODE, num_hex); 
        rule.init_variables()?.init_patches()?;
        if num == 10 {
            return Ok(rule);
        }

        // 给文件规则 添加额外信息
        let (ismain, name) = convert_num(num);
        rule.init_features()?;
        rule.rtype = RuleType::Fileed;
        rule.code = num.to_string();
        rule.name = name;
        rule.index = num;
        rule.ismain = ismain;
        rule.installed = false; 
        // 变量 后续不需要使用了
        rule.variables.clear();
        Ok(rule)
    }

    fn check_files_and_del(&self, must_exist: bool, use_backfile: bool) -> Result<()> {
        self.patches.check_files_and_del(must_exist, use_backfile)
    }

    pub fn del_coexist(&self) -> Result<()> {
        self.patches.del_files()
    }
}

/// patch
impl Rule {
    pub fn patch(
        &mut self,
        fcode: &str,
        status: bool,
        old_cache: Option<&mut Cache>,
    ) -> Result<()> {
        let (save, cache) = match old_cache {
            Some(cache) => (false, cache),
            None => (true, &mut Cache::new()),
        };

        if self.rtype != RuleType::Fileed {
            return Err(ConfigError::IsNotFileRule.into());
        }

        let feature = self.features.get(fcode)?.clone();
        let name = feature.get_name().to_string();
        info!("正在执行 {} 补丁", name.as_str());
        let use_backfile = fcode == COEXISTS_CODE; 
        // 判断是否 不是主程序, 是制作共存程序 。或者 其他 补丁功能已经开启
        let is_patch_ext = !self.ismain || use_backfile || status || self.features.check_other_patch_feature_open(fcode);
        if save {
            // 递归前检查
            self.patches.check_files_and_del(true, use_backfile)?;
        }

        // 检验依赖功能已经开启
        self.features
            .check_dependfeatures(&feature.dependfeatures)?;

        // 关闭互斥功能
        let close_features = if status {
            &feature.mutexfeatures
        // 关闭需要同步关闭的功能
        } else {
            &feature.syncclosefeatures
        };
        for code in close_features {
            if let Ok(feature) = self.features.get(code)
                && feature.status
            {
                debug!("关闭功能 {} ", feature.get_name());
                self.patch(code, false, Some(cache))?;
            }
        }

        
        info!("执行全局额外补丁功能 status = {}", is_patch_ext);
        // 执行全局额外补丁功能, 忽略错误
        let _ = self.patches.patch_rule_extpatches(cache, &feature, &self.extpatches, is_patch_ext).map_err(|e|{
            error!("执行 全局 额外补丁功能失败，{}", e);
        });

        info!("执行补丁功能 status = {}", is_patch_ext);
        // 执行补丁功能
        self.patches.patch(cache, &feature, status)?;

        info!("执行额外补丁功能 status = {}", is_patch_ext);
        // 执行额外补丁功能, 忽略错误
        let _ = self.patches.patch_extpatches(cache, &feature, is_patch_ext).map_err(|e|{
            error!("执行 {} 额外补丁功能失败，{}", feature.get_name(), e);
        });
       

        self.features.get_mut(fcode)?.status = status;

        // 写入到文件
        if save {
            // 读取补丁状态
            if !use_backfile {
                self.set_patched(Some(cache))?;
            }
            cache.save()?;
        }

        info!("{} 补丁 执行完毕", name);

        Ok(())
    }

    pub fn patch_by_replace(&mut self, fcode: &str, ovs: &OrignalViews) -> Result<()> {
        let mut cache = Cache::new();
        if self.rtype != RuleType::Fileed {
            return Err(ConfigError::IsNotFileRule.into());
        }
        let feature = self.features.get(fcode)?;
        // 执行补丁功能
        self.patches.patch_by_replace(&mut cache, feature, ovs)?;

        // 写入到文件
        cache.save()?;

        info!("{} 补丁 执行完毕", self.get_name());

        Ok(())
    }

    pub fn set_patched(&mut self, old_cache: Option<&mut Cache>) -> Result<&Self> {
        let cache = match old_cache {
            Some(cache) => cache,
            None => &mut Cache::new(),
        };
        if self.rtype != RuleType::Fileed {
            return Err(ConfigError::IsNotFileRule.into());
        }
        self.patches.set_patched(cache)?;
        self.features.set_status(&self.patches)?;
        Ok(self)
    }
}

/// read
impl Rule {
    pub fn read_orignal(&self, fcode: &str) -> Result<OrignalViews> {
        let mut cache = Cache::new();
        let feature = self.features.get(fcode)?;
        self.patches.read_orignal(&mut cache, &feature)
    }
}

/// chech rtype
impl Rule {
    pub fn check_is_config_type(&self) -> Result<()> {
        if self.rtype != RuleType::Config {
            return Err(ConfigError::PleaseUseConfigRule.into());
        }
        Ok(())
    }

    pub fn check_is_pathed_type(&self) -> Result<()> {
        if self.rtype != RuleType::Pathed {
            return Err(ConfigError::PleaseUsePathedRule.into());
        }
        Ok(())
    }

    pub fn check_is_search_type(&self) -> Result<()> {
        if self.rtype != RuleType::Search {
            return Err(ConfigError::PleaseUseSearchRule.into());
        }
        Ok(())
    }

    pub fn check_is_fileed_type(&self) -> Result<()> {
        if self.rtype != RuleType::Fileed {
            return Err(ConfigError::IsNotFileRule.into());
        }
        Ok(())
    }

    pub fn check_is_installed(&self) -> Result<()> {
        if !self.installed {
            return Err(ConfigError::NotInstalled(self.get_name().to_string()).into());
        }
        Ok(())
    }
}

#[repr(usize)]
#[derive(Debug, Clone, Serialize_repr, Deserialize_repr, PartialEq, PartialOrd)]
pub enum RuleType {
    Config = 0,
    Pathed = 1,
    Search = 2,
    Fileed = 3,
}

impl Default for RuleType {
    fn default() -> Self {
        RuleType::Config
    }
}
