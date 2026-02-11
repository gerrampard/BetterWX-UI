use std::num::ParseIntError;

use crate::update::UpdatesError;
use thiserror::Error;
use tokio::task::JoinError;
use utils::errors::UtilsError;
use winsys::errors::WinsysError;

pub type Result<T> = core::result::Result<T, ConfigError>;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("配置文件错误，{0} 字段缺失")]
    ConfigFieldMissing(String),

    #[error("未检测到 {0}，请尝试放到 {0} 安装目录内运行")]
    NotInstalled(String),

    #[error("获取安装位置失败，请尝试放到 {0} 安装目录内运行")]
    GetPathFailedError(String),

    #[error("获取安装位置，执行 {0} 方法失败")]
    GetPathRunMethodError(String),

    #[error("无效的共存序号 {0}")]
    InvalidCoexistNum(String),

    #[error("请使用原始规则操作")]
    PleaseUseConfigRule,

    #[error("请使用路径规则操作")]
    PleaseUsePathedRule,

    #[error("请使用基址规则操作")]
    PleaseUseSearchRule,

    #[error("请使用文件规则操作")]
    PleaseUseFileedRule,

    #[error("特征码不支持该版本：{0}，请升级")]
    PatternNotSupported(String),

    #[error("无法获取对应版本数据：{0}")]
    TakeByVersionError(String),

    #[error("校验文件失败，被校验文件：{0}")]
    GetPathCheckFailedError(String),

    #[error("文件不存在：{0}")]
    FileNotExistsError(String),

    #[error("解析默认功能失败，请检查配置文件")]
    DefaultFeaturesDeserializeError,

    #[error("获取 {0} 数组成员失败，请检查配置文件")]
    GetVecItemNotFindByCode(String),

    #[error("依赖补丁 {0} 未找到，请检查配置文件")]
    DependPatchNotFoundError(String),

    #[error("获取路径替换规则无效 {0} ，请检查配置文件")]
    InvalidPatternReplace(String),

    #[error("获取变量值 {0} 失败")]
    GetVariabledValueError(String),

    #[error("变量 {0} 无效，请检查配置文件")]
    InvalidVariable(String),

    #[error("变量 {0} 类型无效，请检查配置文件")]
    InvalidVariableType(String),

    #[error("前置功能 {0} 未启用")]
    DependFeatureStatusError(String),

    #[error("缓存初始化失败")]
    CacheNotFindError,

    #[error("特征码补丁校验失败！\n原始数据：{0}\n替换数据：{1}")]
    InitPatchReplaceDataError(String, String),

    #[error("主程序文件不存在，路径: {0}，请尝试重启软件")]
    BaseFileInvalid(String),

    #[error("备份的 {0} 被修补过或者损毁，请尝试重装 {1}")]
    BackFileInvalid(String, String),

    #[error("检测到版本发生变化，请重启软件")]
    FileVersionChage,

    #[error("备份文件 {0} 被修补过")]
    BackFileIsPatched(String),

    #[error("共存文件序号无效")]
    CoexistNumInvalid(String),

    #[error("当前共存文件异常，请删除重建")]
    SaveFileInvalid,

    #[error("请在文件规则中使用该功能")]
    IsNotFileRule,

    #[error("获取配置文件失败")]
    GetConfigFileError,

    #[error("基址为空")]
    AddressesEmptyError,

    #[error("计算 {0} 地址失败")]
    CalcAddressError(String),

    #[error("找到的 {0} 基址 {1} 个，期待 {2} 个，可能不支持当前版本")]
    AddressesTooMuchError(String, usize, usize),

    #[error("基址无效")]
    InvalidAddress,

    #[error("CacheLockError")]
    CacheLockError,

    #[error("写入补丁基址缓存失败")]
    WritePacthecCacheError,

    
    #[error("缓存补丁基址失效")]
    CachePacthecInvalidError,

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    UpdatesError(#[from] UpdatesError),

    #[error(transparent)]
    UtilsError(#[from] UtilsError),

    #[error(transparent)]
    WinsysError(#[from] WinsysError),

    #[error(transparent)]
    JsonError(#[from] JoinError),

    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
}
