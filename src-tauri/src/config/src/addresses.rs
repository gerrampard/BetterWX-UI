use crate::errors::ConfigError;
use crate::errors::Result;
use crate::serders::skippers::skip_if_empty;
use crate::variables::Variables;
use crate::views::orignal_view::OrignalView;
use log::debug;
use log::error;
use macros::ImpConfigVecIsEmptyTrait;
use serde::Deserialize;
use serde::Serialize;
use utils::patch::patch::UPatch;
use utils::tools::replace_ellipsis;
use utils::tools::replace_wildcards;

#[derive(Debug, Clone, Serialize, Deserialize, Default, ImpConfigVecIsEmptyTrait)]
pub struct Addresses(pub Vec<Address>);

impl Addresses {
    pub fn init(&mut self, variables: &crate::variables::Variables,pattern_code:&str) -> Result<()> {
        for address in &mut self.0 {
            address.init(variables,pattern_code)?;
        }
        Ok(())
    }

    pub fn create(
        upatch: &UPatch,
        poses: Vec<usize>,
        replace: &str,
        len: usize,
        usereplace: bool,
    ) -> Result<Self> {
        let mut addresses = Vec::new();
        for pos in poses {
            match upatch.read_hex(pos, len) {
                Ok(orignal) => {
                    debug!("地址：{}， 原始补丁：{:?}", pos, orignal);
                    debug!("地址：{}， 替换补丁：{:?}", pos, replace);
                    let start_rva = if let Ok(rva) = upatch.foa_to_rva(pos as u64) {
                        rva as usize
                    } else {
                        0
                    };
                    addresses.push(Address::new(
                        orignal,
                        replace.to_string(),
                        pos,
                        start_rva,
                        len,
                        usereplace,
                    ));
                }
                Err(e) => {
                    error!("读取地址失败！地址：{}， 长度：{}， 错误：{}", pos, len, e);
                    return Err(e.into());
                }
            }
        }
        Ok(Addresses(addresses))
    }

    pub fn get_patched(&mut self, upatch: &UPatch) -> Result<bool> {
        for address in &mut self.0 {
            address.get_patched(upatch)?;
        }
        let patched = self.0.iter().filter(|address| !(address.replace.is_empty() && address.replace != "...")).all(|address| address.patched);
        Ok(patched)
    }

    pub fn read_orignal(&self, upatch: &UPatch) -> Result<OrignalView> {
        for address in &self.0 {
            match upatch.read_hex(address.start, address.len) {
                Ok(replace) => {
                    return Ok(OrignalView::new(
                        String::new(),
                        String::new(),
                        replace,
                        address.start,
                        address.len,
                    ));
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
        Err(ConfigError::AddressesEmptyError)
    }

    pub fn patch(&mut self, upatch: &mut UPatch, status: bool) -> Result<()> {
        for address in &mut self.0 {
            address.patch(upatch, status)?;
        }
        Ok(())
    }

    pub fn patch_by_replace(&mut self, upatch: &mut UPatch, replace: &str) -> Result<()> {
        for address in &mut self.0 {
            address.patch_by_replace(upatch, replace)?;
        }
        Ok(())
    }

    pub fn is_patched(&self) -> bool {
        self.0.iter().any(|address| address.patched)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub orignal: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub replace: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub start: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub start_rva: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub len: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub end: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub patched: bool,
}

impl Address {
    pub fn new(orignal: String, replace: String, start: usize,start_rva: usize, len: usize, patched: bool) -> Self {
        Self {
            orignal,
            replace,
            start,
            start_rva,
            len,
            end: start + len,
            patched,
        }
    }

    pub fn init(&mut self, variables: &Variables,pattern_code:&str) -> Result<()> {
        let _ = variables.get_num_hex()?;
        if self.replace.is_empty() || self.replace.as_str() == "..." {
            self.replace = "".to_string();
            return Ok(())
        }
        let mut replace = self.replace.to_string();
        replace = variables.substitute_add(replace,false,pattern_code)?;
        replace = variables.substitute(replace);
        replace = replace_ellipsis(replace.as_str(), self.orignal.as_ref())?;
        self.replace = replace_wildcards(replace.as_str(), self.orignal.as_ref())?;
        //初始化 patched
        self.check_replace_data()?;
        self.patched = false;
        Ok(())
    }

    pub fn get_patched(&mut self, upatch: &UPatch) -> Result<bool> {
        let data = upatch.read_hex(self.start, self.len)?;
        self.patched = data != self.orignal;
        Ok(self.patched)
    }

    pub fn patch(&mut self, upatch: &mut UPatch, status: bool) -> Result<()> {
        self.patched = status;
        let base_data = upatch.read_hex(self.start, self.len)?;
        let new_data = if status {
            self.replace.as_str()
        } else {
            self.orignal.as_str()
        };

        if new_data.is_empty() {
            return Ok(())
        }

        debug!(
            "基址：{}\n原始数据：{:?}\n当前数据：{:?}\n替换数据：{:?}",
            self.start, self.orignal.as_str(), base_data, new_data
        );
        upatch.write(self.start, new_data.into())?;
        Ok(())
    }

    pub fn patch_by_replace(&mut self, upatch: &mut UPatch, replace: &str) -> Result<()> {
        if replace.len()/2 > self.len {
            return Err(ConfigError::InitPatchReplaceDataError(
                self.orignal.to_string(),
                self.replace.to_string(),
            )
            .into());
        }
        self.patched = true;
        upatch.write(self.start, replace.into())?;
        Ok(())
    }

    pub fn check_replace_data(&self) -> Result<()> {
        let v = Variables::create_js_varibales(&self.replace);
        if !v.is_empty()
            || self.orignal.len() != self.replace.len()
            || self.replace.len() / 2 != self.len
        {
            return Err(ConfigError::InitPatchReplaceDataError(
                self.orignal.to_string(),
                self.replace.to_string(),
            )
            .into());
        }
        Ok(())
    }
}
