use crate::errors::ConfigError;
use crate::errors::Result;
use crate::serders::skippers::skip_if_empty;
use crate::variables::Variable;
use crate::variables::Variables;
use log::debug;
use log::error;
use log::info;
use log::trace;
use macros::FieldDescGetters;
use macros::FieldNameGetters;
use macros::ImpConfigVecIsEmptyTrait;
use macros::SortedSerializeByIndex;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Display;
use std::path::Path;
use utils::empty::Empty;
use utils::runtime::Runtime;
use winsys::fileinfo::FileInfo;
use winsys::registry::Registry;

const VALUE_CODE: &str = "value";
const PATH_CODE: &str = "path";
const FILE_CODE: &str = "file";
const FIELD_CODE: &str = "field";

#[derive(Debug, Clone, Default, ImpConfigVecIsEmptyTrait, SortedSerializeByIndex)]

pub struct Paths(pub Vec<PathItem>);

impl Paths {
    pub fn init(&mut self) -> Result<Variables> {
        let mut path_variables: Variables = Variables::default();
        for path_item in &mut self.0 {
            let variables = path_item.init(&path_variables);
            match variables {
                Ok(v) => {
                    path_variables.extend(v);
                }
                Err(e) => {
                    error!("尝试获取：{}。失败！错误：{}", path_item.get_name(), e);
                    return Err(e);
                }
            }
        }
        *self = Paths::default();
        Ok(path_variables)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FieldDescGetters, FieldNameGetters)]
pub struct PathItem {
    pub code: String,
    pub index: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub name: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub description: String,
    pub methods: Methods,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub value: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub path: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub file: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub fix: PathFix,
}

impl PathItem {
    pub fn init(&mut self, path_variables: &Variables) -> Result<Variables> {
        let name = self.get_name().to_string();
        for method in &mut self.methods.0 {
            let msg = format!("获取：{}，方法名：{}", name, method.method);
            let value = match method.init(path_variables) {
                Ok(v) => v,
                Err(e) => {
                    error!("{}。失败！错误：{}", msg, e);
                    continue;
                }
            };
            let value = self.fix.run(value)?;
            let mut temp_vars = Variables::default();
            temp_vars.set_value(VALUE_CODE, &value);
            if !self.path.is_empty() {
                let path = path_variables.substitute(temp_vars.substitute(&self.path));
                temp_vars.set_value(PATH_CODE, &path);
            }
            if !self.file.is_empty() {
                let file = path_variables.substitute(temp_vars.substitute(&self.file));
                temp_vars.set_value(FILE_CODE, &file);
                if !Path::new(&file).exists() {
                    error!(
                        "{}。失败！错误：{}",
                        msg,
                        ConfigError::GetPathCheckFailedError(file),
                    );
                    continue;
                }
            }
            info!("{}。成功！结果：{}", msg, value);
            let v1 = Variable::new(self.code.clone(), value);
            let vs = Variables(vec![v1]);
            return Ok(vs);
        }
        Err(ConfigError::GetPathFailedError(name).into())
    }
}

#[derive(Debug, Clone, SortedSerializeByIndex)]
pub struct Methods(pub Vec<Method>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Method {
    pub method: MethodType,
    pub index: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub args: Variables,
    #[serde(default)]
    pub retry: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub fix: PathFix,
}

impl Method {
    pub fn init(&mut self, path_variables: &Variables) -> Result<String> {
        // 替换参数变量
        for variable in &mut self.args.0 {
            let new_value = path_variables.substitute(variable.get_value().to_string());
            variable.set_value(new_value);
        }

        let retry = if self.retry < 1 { 1 } else { self.retry };
        for i in 0..retry {
            let result = match self.method {
                MethodType::RunTime => self.get_path_by_runtime(),
                MethodType::Regedit => self.get_path_by_regedit(),
                MethodType::FileInfo => self.get_path_by_fileinfo(),
                MethodType::Calculate => self.get_path_by_calculate(),
                MethodType::ReadFile => self.get_path_by_readfile(),
            };
            match result {
                Ok(r) => {
                    return Ok(r);
                }
                Err(e) => {
                    if i == retry - 1 {
                        return Err(e);
                    }
                }
            }
        }
        Err(ConfigError::GetPathRunMethodError(self.method.to_string()).into())
    }

    fn get_path_by_calculate(&mut self) -> Result<String> {
        let value = self.get_required_arg(VALUE_CODE)?.to_string();
        let value = self.fix.run(value)?;
        Ok(value)
    }

    fn get_path_by_fileinfo(&mut self) -> Result<String> {
        let path = self.get_required_arg(PATH_CODE)?;
        trace!("get_path_by_fileinfo path:{:?}", path);
        let value = FileInfo::new(path).get_version()?;
        trace!("get_path_by_fileinfo value:{:?}", value);
        let value = self.fix.run(value)?;
        Ok(value)
    }

    fn get_path_by_regedit(&mut self) -> Result<String> {
        let path = self.get_required_arg(PATH_CODE)?;
        trace!("get_path_by_regedit path:{:?}", path);
        let field = self.get_required_arg(FIELD_CODE)?;
        trace!("get_path_by_regedit field:{:?}", field);
        let value = Registry::new(&path)?.read_value(&field)?;
        trace!("get_path_by_regedit value:{:?}", value);
        let value = self.fix.run(value)?;
        Ok(value)
    }

    fn get_path_by_runtime(&mut self) -> Result<String> {
        let exe_path = Runtime::current_dir()?.to_string_lossy().to_string();
        trace!("get_path_by_runtime exe_path:{:?}", exe_path);
        let value = self.fix.run(exe_path)?;
        Ok(value)
    }

    fn get_path_by_readfile(&mut self) -> Result<String> {
        let path = self.get_required_arg(PATH_CODE)?;
        trace!("get_path_by_readfile path:{:?}", path);
        let field = self.get_required_arg(FIELD_CODE)?;
        trace!("get_path_by_readfile field:{:?}", field);
        let value = self.get_required_arg(VALUE_CODE)?.parse::<usize>()?;
        trace!("get_path_by_readfile value:{:?}", value);
        let re = Regex::new(&field)
            .map_err(|_| ConfigError::InvalidPatternReplace(field.to_string()))?;
        let data = std::fs::read_to_string(&path)?;
        let result = re.captures(&data);
        if let Some(caps) = result {
            let result = caps.get(value).map(|m| m.as_str().to_string());
            if let Some(result) = result {
                trace!("get_path_by_readfile value  captures get:{:?}", result);
                let value = self.fix.run(result)?;
                trace!("get_path_by_readfile value  fix:{:?}", value);
                return Ok(value);
            }
        }
        Err(ConfigError::GetPathCheckFailedError(path.to_string()).into())
    }

    fn get_required_arg(&self, key: &str) -> Result<String> {
        Ok(self
            .args
            .find_variable(key)
            .ok_or(ConfigError::ConfigFieldMissing(key.to_string()))?
            .get_value()
            .to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MethodType {
    RunTime,
    Calculate,
    FileInfo,
    Regedit,
    ReadFile,
}

impl Display for MethodType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MethodType::RunTime => write!(f, "运行目录"),
            MethodType::Calculate => write!(f, "计算"),
            MethodType::FileInfo => write!(f, "文件信息"),
            MethodType::Regedit => write!(f, "注册表"),
            MethodType::ReadFile => write!(f, "读取文件"),
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PathFix {
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub unprefix: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub unsuffix: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub prefix: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub suffix: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub pattern: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "skip_if_empty")]
    pub replace: String,
}

impl PathFix {
    fn run<S: Into<String>>(&self, result: S) -> Result<String> {
        let mut result = result.into();
        if !self.unprefix.is_empty() {
            let index = result.find(&self.unprefix).unwrap_or(0);
            if index > 0 {
                result = result[index + self.unprefix.len()..].to_string();
            }
        }
        if !self.unsuffix.is_empty() {
            let index = result.find(&self.unsuffix).unwrap_or(0);
            if index > 0 {
                result = result[..index].to_string();
            }
        }
        if !self.pattern.is_empty() {
            let re = Regex::new(&self.pattern)
                .map_err(|_| ConfigError::InvalidPatternReplace(self.pattern.to_string()))?;
            result = re.replace(&result, &self.replace).to_string();
        }
        if !self.prefix.is_empty() {
            result = format!("{}{}", self.prefix, result);
        }
        if !self.suffix.is_empty() {
            result = format!("{}{}", result, self.suffix);
        }
        debug!("fix_result {:?}", result);
        Ok(result)
    }
}

impl Empty for PathFix {
    fn is_empty(&self) -> bool {
        self.unprefix.is_empty()
            && self.unsuffix.is_empty()
            && self.prefix.is_empty()
            && self.suffix.is_empty()
            && self.pattern.is_empty()
            && self.replace.is_empty()
    }
}
