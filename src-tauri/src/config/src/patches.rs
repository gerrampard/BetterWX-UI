use crate::views::orignal_view::OrignalViews;
use crate::ConfigVecWrapperTrait;
use crate::cache::Cache;
use crate::errors::ConfigError;
use crate::errors::Result;
use crate::features::COEXISTS_CODE;
use crate::features::Feature;
use crate::patterns::Pattern;
use crate::patterns::Patterns;
use crate::serders::skippers::skip_if_empty;
use crate::variables::Variables;
use log::debug;
use log::error;
use log::trace;
use macros::FieldDescGetters;
use macros::FieldNameGetters;
use macros::ImpConfigVecIsEmptyTrait;
use macros::ImpConfigVecWrapperTrait;
use serde::Deserialize;
use serde::Serialize;
use utils::file::del_directory;
use std::fmt::Debug;
use std::path::Path;
use utils::empty::Empty;
use utils::file::back_file;
use utils::file::file_is_equal;
use utils::file::remove_file;
use utils::file::copy_directory;
use utils::patch::patch::UPatch;

#[derive(
    Clone, Serialize, Deserialize, Default, ImpConfigVecIsEmptyTrait, ImpConfigVecWrapperTrait,
)]
pub struct Patches(pub Vec<Patch>);

impl Patches {
    pub fn init(&mut self, variables: &Variables) -> Result<()> {
        for patch in &mut self.0 {
            patch.init(variables)?;
        }
        Ok(())
    }

    pub fn set_patched(&mut self, data_cache: &mut Cache) -> Result<()> {
        for patch in &mut self.0 {
            let upatch = Self::build_upatch(patch, data_cache, false, "读取补丁状态", false)?;
            patch.set_patched(upatch)?;
        }
        Ok(())
    }

    pub fn back_files(&mut self) -> Result<()> {
        for patch in &mut self.0 {
            back_file(patch.get_basefile(), patch.get_backfile())?;
        }
        Ok(())
    }

    pub fn search(&mut self, data_cache: &mut Cache, name: &str) -> Result<()> {
        let all_bak_files = self
            .0
            .iter()
            .map(|patch| patch.backfile.clone())
            .collect::<Vec<String>>();
        for patch in &mut self.0 {
            let upatch = Self::build_upatch(patch, data_cache, true, "搜索基址", false)?;
            if let Err(e) = patch.search(&upatch) {
                error!("搜索基址失败：{}", e);
                for file in all_bak_files {
                    remove_file(file.as_str())?;
                }
                return Err(ConfigError::BackFileInvalid(
                    patch.get_name().to_string(),
                    name.to_string(),
                )
                .into());
            };
        }
        Ok(())
    }

    pub fn patch(&mut self, data_cache: &mut Cache, feature: &Feature, status: bool) -> Result<()> {
        // 制作共存时强制使用 backfile ，否则强制使用 save_file
        let use_backfile = feature.code.as_str() == COEXISTS_CODE;
        for code in &feature.dependpatches {
            let patch = self.find_mut_patch_by_pattern_code(code.as_str())?;
            let upatch = Self::build_upatch(patch, data_cache, use_backfile, &feature.code, true)?;
            patch.patch(upatch, code, status)?;
        }
        Ok(())
    }

    pub fn patch_extpatches(&mut self, data_cache: &mut Cache, feature: &Feature, status: bool) -> Result<()> {
        // 制作共存时强制使用 backfile ，否则强制使用 save_file
        let use_backfile = feature.code.as_str() == COEXISTS_CODE;
        // 处理附加补丁
        for code in &feature.extpatches {
            let patch = self.find_mut_patch_by_pattern_code(code.as_str())?;
            let upatch = Self::build_upatch(patch, data_cache, use_backfile, &feature.code, true)?;
            
            patch.patch(upatch, code, status)?;
        }
        Ok(())
    }

    pub fn patch_rule_extpatches(&mut self, data_cache: &mut Cache, feature: &Feature, extpatches: &Vec<String>, status: bool) -> Result<()> {
        // 制作共存时强制使用 backfile ，否则强制使用 save_file
        let use_backfile = feature.code.as_str() == COEXISTS_CODE;
        // 处理 全局 附加补丁
        for code in extpatches {
            let patch = self.find_mut_patch_by_pattern_code(code.as_str())?;
            let upatch = Self::build_upatch(patch, data_cache, use_backfile, &feature.code, true)?;
            
            patch.patch(upatch, code, status)?;
        }
        Ok(())
    }

    pub fn patch_by_replace(&mut self, data_cache: &mut Cache, feature: &Feature, ovs: &OrignalViews) -> Result<()> {
        for ov in &ovs.0 {
            let patch = self.find_mut_patch_by_pattern_code(&ov.pcode)?;

            let upatch = Self::build_upatch(patch, data_cache, false, &feature.code, true)?;
            patch.patch_by_replace(upatch, &ov.pcode,&ov.orignal)?;
        }
        Ok(())
    }

    pub fn read_orignal(
        &self,
        data_cache: &mut Cache,
        feature: &Feature,
    ) -> Result<OrignalViews> {
        let mut r = Vec::new();
        for code in &feature.dependpatches {
            let patch = self.find_patch_by_pattern_code(code.as_str())?;
            let pattern = self.get_pattern(code.as_str())?;
            if pattern.disabled {
                continue;
            }
            let upatch = Self::build_upatch(patch, data_cache, false, &feature.code, false)?;
            let mut ov = pattern.read_orignal(upatch)?;
            ov.pcode = code.to_string();
            ov.pname = pattern.get_name().to_string();
            r.push(ov);
        }
        Ok(OrignalViews(r))
    }

    pub fn check_files_and_del(&self, must_exist: bool, use_backfile: bool) -> Result<()> {
        let mut last_error = None;
        self.0.iter().for_each(|patch| {
            if let Err(e) = patch.check_file(must_exist, use_backfile) {
                last_error = Some(e);
            }
        });
        if let Some(e) = last_error {
            debug!("检查文件失败：{}", e);
            self.del_files()?;
            return Err(e);
        }
        Ok(())
    }

    pub fn copy_directory(&self) -> Result<()> {
        for patch in &self.0 {
            patch.copy_directory()?;
        }
        Ok(())
    }

    pub fn del_files(&self) -> Result<()> {
        for patch in &self.0 {
            // 删除拷贝的目录
            patch.del_directory()?;
            patch.del_file()?;
        }
        Ok(())
    }

    fn build_upatch<'a>(
        patch: &Patch,
        data_cache: &'a mut Cache,
        use_backfile: bool,
        name: &str,
        with_write: bool,
    ) -> Result<&'a mut UPatch> {
        let backfile = patch.get_backfile();
        let savefile = patch.get_savefile();

        let key = match use_backfile {
            true => backfile,
            false => savefile,
        };

        let exists = Path::new(key).exists();
        trace!(
            "\n正在执行操作：{}\n操作的：{}，文件是否存在：{}\n保存为：{}",
            name, key, exists, savefile
        );

        if exists {
            return data_cache.get_or_insert(key, key, savefile, with_write);
        }
        return Err(ConfigError::FileNotExistsError(key.to_string()).into());
    }
}

impl Patches {
    pub fn is_supported(&self) -> bool {
        self.0.iter().all(|patch| patch.is_supported())
    }

    pub fn is_searched(&self) -> bool {
        self.0.iter().any(|patch| patch.is_searched())
    }

    pub fn is_patched(&self) -> bool {
        self.0.iter().any(|patch| patch.is_patched())
    }

    pub fn find_mut_patch_by_pattern_code(&mut self, code: &str) -> Result<&mut Patch> {
        for patch in &mut self.0 {
            let p = patch.patterns.find(code);
            if let Some(_) = p {
                return Ok(patch);
            }
        }
        Err(ConfigError::DependPatchNotFoundError(code.to_string()).into())
    }

    pub fn find_patch_by_pattern_code(&self, code: &str) -> Result<&Patch> {
        for patch in &self.0 {
            let p = patch.patterns.find(code);
            if let Some(_) = p {
                return Ok(patch);
            }
        }
        Err(ConfigError::DependPatchNotFoundError(code.to_string()).into())
    }

    pub fn get_pattern(&self, code: &str) -> Result<&Pattern> {
        for patch in &self.0 {
            let p = patch.patterns.find(code);
            if let Some(p) = p {
                return Ok(p);
            }
        }
        Err(ConfigError::DependPatchNotFoundError(code.to_string()).into())
    }

    pub fn clone_pattern(&mut self, patches: &Patches) -> Result<&Self> {
        for patch in &mut self.0 {
            patch.patterns = patches.get(patch.code.as_str())?.patterns.clone();
        }
        Ok(self)
    }
}

impl Debug for Patches {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for p in &self.0 {
            for p in &p.patterns.0 {
                if let Some(g) = &p.group {
                    writeln!(f, "特征码：{} = {}", p.get_name(), g.pattern)?;
                }
                if !p.addresses.is_empty() {
                    for a in &p.addresses.0 {
                        writeln!(f, "基址:{} = {}，len:{}", p.get_name(), a.start, a.len)?;
                    }
                    writeln!(f, "--------------------------------------------")?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FieldDescGetters, FieldNameGetters)]
pub struct Patch {
    pub code: String,
    pub savefile: String,
    pub backfile: String,
    pub basefile: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub copy_files: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub copy_from_directory: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub copy_to_directory: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub patterns: Patterns,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub name: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub description: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub supported: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub patched: bool,
}

impl Patch {
    pub fn init(&mut self, variables: &Variables) -> Result<()> {
        // 处理特征码，获取版本对应的特征码
        self.patterns.init(variables)?;
        // 传入 num 构建文件路径
        if let Ok(_) = variables.get_num() {
            self.savefile = variables.fix_main_target(self.savefile.as_str());

            self.backfile = variables.substitute(self.backfile.as_str());
            self.basefile = variables.substitute(self.basefile.as_str());
            self.savefile = variables.substitute(self.savefile.as_str());
            self.copy_from_directory = variables.substitute(self.copy_from_directory.as_str());
            self.copy_to_directory = variables.substitute(self.copy_to_directory.as_str());
        }
        Ok(())
    }

    pub fn set_patched(&mut self, upatch: &UPatch) -> Result<()> {
        let name = self.get_name().to_string();
        trace!(
            "********** 正在对 {} 文件读取状态，地址：{} **********",
            name,
            upatch.get_file()
        );
        self.patterns.set_patched(upatch)?;
        Ok(())
    }

    pub fn search(&mut self, upatch: &UPatch) -> Result<()> {
        let name = self.get_name().to_string();
        if !self.is_searched() {
            debug!(
                "********** 正在对 {} 文件搜索，地址：{} **********",
                name,
                upatch.get_file()
            );
            self.patterns.search(upatch)?;
        }
        self.supported = self.is_supported();
        self.patched = self.is_patched();
        Ok(())
    }

    pub fn patch(&mut self, upatch: &mut UPatch, code: &str, status: bool) -> Result<()> {
        let pattern = self.patterns.get_mut(code)?;
        pattern.patch(upatch, status)
    }

    pub fn patch_by_replace(&mut self, upatch: &mut UPatch, code: &str, orignal: &str) -> Result<()> {
        let pattern = self.patterns.get_mut(code)?;
        pattern.patch_by_replace(upatch, orignal)
    }
    

    pub fn check_file(&self, must_exist: bool, use_backfile: bool) -> Result<()> {
        let basefile = self.get_basefile();
        let savefile = self.get_savefile();
        let backfile = self.get_backfile();
        // 搜索基址，back 必须存在
        let key = match use_backfile {
            true => backfile,
            false => savefile,
        };
        // 补丁，save 必须存在 除了 make coexist
        if !Path::new(basefile).exists() {
            return Err(ConfigError::BaseFileInvalid(basefile.to_string()).into());
        }
        if must_exist {
            if !Path::new(key).exists() || !file_is_equal(basefile, key)? {
                return Err(ConfigError::SaveFileInvalid.into());
            }
        } else {
            if Path::new(key).exists() && !file_is_equal(basefile, key)? {
                return Err(ConfigError::SaveFileInvalid.into());
            }
        }
        Ok(())
    }

    pub fn copy_directory(&self) -> Result<()> {
        let copy_files = self.get_copy_files();
        let copy_from_directory = self.get_copy_from_directory();
        let copy_to_directory = self.get_copy_to_directory();
        if copy_files{
            debug!("拷贝文件夹: {:?}", self);
            copy_directory(copy_from_directory, copy_to_directory)?;
        }
        Ok(())
    }

    pub fn del_directory(&self) -> Result<()> {
        let copy_files = self.get_copy_files();
        let copy_to_directory = self.get_copy_to_directory();
        if copy_files{
            debug!("删除文件夹: {:?}", self);
            let _ = del_directory(copy_to_directory);
        }
        Ok(())
    }

    pub fn del_file(&self) -> Result<()> {
        let savefile = self.get_savefile();
        remove_file(savefile)?;
        Ok(())
    }
}

impl Patch {
    pub fn get_savefile(&self) -> &str {
        self.savefile.as_str()
    }

    pub fn get_copy_files(&self) -> bool {
        self.copy_files
    }

    pub fn get_copy_from_directory(&self) -> &str {
        self.copy_from_directory.as_str()
    }

    pub fn get_copy_to_directory(&self) -> &str {
        self.copy_to_directory.as_str()
    }

    pub fn get_basefile(&self) -> &str {
        self.basefile.as_str()
    }

    pub fn get_backfile(&self) -> &str {
        self.backfile.as_str()
    }

    pub fn find(&self, code: &str) -> Option<&Pattern> {
        self.patterns.find(code)
    }

    pub fn find_mut(&mut self, code: &str) -> Option<&mut Pattern> {
        self.patterns.find_mut(code)
    }

    pub fn is_supported(&self) -> bool {
        self.patterns.is_supported()
    }

    pub fn is_searched(&self) -> bool {
        self.patterns.is_searched()
    }

    pub fn is_patched(&self) -> bool {
        self.patterns.is_patched()
    }
}
