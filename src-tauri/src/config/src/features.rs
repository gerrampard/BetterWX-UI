use crate::ConfigVecWrapperTrait;
use crate::errors::ConfigError;
use crate::errors::Result;
use crate::patches::Patches;
use crate::serders::default::default_true;
use crate::serders::skippers::skip_if_empty;
use crate::serders::skippers::skip_if_tdelay;
use crate::variables::Variables;
use log::info;
use log::trace;
use macros::FieldDescGetters;
use macros::FieldNameGetters;
use macros::ImpConfigVecIsEmptyTrait;
use macros::ImpConfigVecWrapperTrait;
use macros::SortedSerializeByIndex;
use serde::Deserialize;
use serde::Serialize;
use utils::empty::Empty;

pub const COEXISTS_CODE: &str = "coexist";

#[derive(
    Clone, Default, ImpConfigVecIsEmptyTrait, ImpConfigVecWrapperTrait, SortedSerializeByIndex,
)]
pub struct Features(pub Vec<Feature>);

impl Features {
    pub fn init_hfeatures(&mut self, variables: &Variables, patches: &Patches) -> Result<()> {
        if let Err(_) = variables.get_num() {
            self.init_features(variables, patches)?;
        }
        Ok(())
    }

    pub fn init_features(&mut self, variables: &Variables, patches: &Patches) -> Result<()> {
        if self.is_empty() {
            return Ok(());
        }
        self.0
            .iter_mut()
            .try_for_each(|feature| feature.init(variables, patches))
    }

    pub fn set_status(&mut self, patches: &Patches) -> Result<()> {
        for feature in &mut self.0 {
            if !feature.supported || feature.disabled {
                feature.status = false;
                continue;
            }
            feature.status = feature
                .dependpatches
                .iter()
                .map(|pcode| patches.get_pattern(pcode).map(|p| p.patched))
                .collect::<Result<Vec<bool>>>()?
                .into_iter()
                .all(|patched| patched);
        }
        Ok(())
    }

    pub fn check_dependfeatures(&self, dependfeatures: &[String]) -> Result<()> {
        dependfeatures.iter().try_for_each(|code| {
            self.get(code).and_then(|feature| {
                if !feature.status {
                    return Err(ConfigError::DependFeatureStatusError(code.to_string()).into());
                }
                Ok(())
            })
        })
    }

    pub fn sort_by_key(&mut self) {
        self.0.sort_by_key(|f| f.index);
    }

    pub fn push(&mut self, feature: Feature) {
        self.0.push(feature);
        self.sort_by_key();
    }

    pub fn take_inhead(&mut self) -> Self {
        let mut hfeatures = Vec::new();
        (0..self.len()).rev().for_each(|index| {
            if self.0[index].inhead {
                hfeatures.push(self.0.remove(index));
            }
        });
        Features(hfeatures)
    }

    pub fn retain_features(&mut self, ismain: bool) {
        if ismain {
            self.retain_inmain();
        } else {
            self.retain_incoexist();
        }
    }

    pub fn retain_inmain(&mut self) {
        self.0.retain(|f| f.inmain)
    }

    pub fn retain_incoexist(&mut self) {
        self.0.retain(|f| f.incoexist)
    }

    pub fn extend(&mut self, mut features: Features) {
        features.0.retain(|f| self.find(f.code.as_str()).is_none());
        self.0.extend(features.0);
        self.sort_by_key();
    }

    pub fn check_other_patch_feature_open(&self,code: &str) -> bool {
        self.0.iter().any(|f| f.method.as_str() == "patch" && f.status && f.code.as_str() != code)
    }
}

impl std::fmt::Debug for Features {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for feature in &self.0 {
            writeln!(
                f,
                "{:?}: index = {:?}  ,inhead = {:?}  ,inmain = {:?}  ,incoexist = {:?}",
                feature.get_name(),
                feature.index,
                feature.inhead,
                feature.inmain,
                feature.incoexist
            )?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Bntype {
    Switch,
    Button,
    Checkbox,
}

impl Default for Bntype {
    fn default() -> Self {
        Self::Button
    }
}

impl Empty for Bntype {
    fn is_empty(&self) -> bool {
        if let Self::Button = self { true } else { false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FieldDescGetters, FieldNameGetters)]
pub struct Feature {
    pub code: String, // 唯一标识
    pub index: usize, // 排序
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub name: String, // 名称
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub method: String, // 名称
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub icon: String, // 名称
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub description: String, // 描述
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub detaildesc: String, // 描述
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub inhead: bool, // 是否头部功能区显示
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub inmain: bool, // 是否主程序显示
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub incoexist: bool, // 是否共存显示
    #[serde(default)]
    pub bntype: Bntype,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub severity: String, // 按钮显示样式
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub tips: String, // 提示
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub disabled: bool, // 是否禁用
    #[serde(default = "default_true")]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub supported: bool, // 是否支持
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub target: String, // 目标，用于指定目标程序/目录
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub selected: bool, // 当前选中状态,用于checkbox
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub status: bool, // 当前功能状态
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_tdelay")]
    pub tdelay: usize, // 文本显示延迟
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub dependpatches: Vec<String>, // 补丁依赖
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub extpatches: Vec<String>, // 额外补丁依赖
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub dependfeatures: Vec<String>, // 前置功能
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub mutexfeatures: Vec<String>, // 互斥功能
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub syncclosefeatures: Vec<String>, // 同步关闭
}

impl Feature {
    pub fn init(&mut self, variables: &Variables, patches: &Patches) -> Result<()> {
        if self.disabled {
            return Ok(());
        }

        let num = match variables.get_num() {
            Ok(num) => num,
            Err(_) => 10,
        };

        // 修复 头部功能 target ,或者 构建时 target
        if !self.dependpatches.is_empty() && num == 10 && patches.is_searched() {
            let mut all_supported = true;
            let mut all_disabled = true;
            for code in self.dependpatches.iter() {
                let p = patches.get_pattern(code)?;
                all_supported = all_supported && p.supported;
                all_disabled = all_disabled && p.disabled;
            }
            self.disabled = all_disabled;
            self.supported = all_supported;
            info!(
                "功能：{}，修正 supported：{} ,disabled：{}",
                self.get_name(),
                self.supported,
                self.disabled
            );
        }

        // 修复 头部功能 target ,或者 构建时 target
        if !self.target.is_empty() && (self.inhead || num != 10) {
            self.target = variables.fix_main_target(self.target.as_str());

            self.target = variables.substitute(self.target.as_str());

            trace!("功能：{}，修正target：{}", self.get_name(), self.target);
        }
        Ok(())
    }
}
