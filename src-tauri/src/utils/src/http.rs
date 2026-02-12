use crate::base64::Base64;
use crate::errors::Result;
use log::error;
use reqwest::Client;
use std::time::Duration;
use thiserror::Error;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36 Edg/138.0.0.0";
const HTTPS_PREFIX: &str = "https://";
const HTTP_PREFIX: &str = "http://";
const ERROR_PREFIX: &str = "网络请求错误";

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("{ERROR_PREFIX}，请检查网络")]
    RequestError,

    #[error("{ERROR_PREFIX}：{0}")]
    RequestStatusError(String),

    #[error("{ERROR_PREFIX}，请求超时")]
    TimeoutError,
}

pub struct Http {
    passwrd: Option<String>,
    client: Client,
}

pub struct HttpResult {
    pub detext: Option<String>,
    pub orignal: String,
}

impl HttpResult {
    pub fn new(detext: Option<String>, orignal: String) -> Self {
        Self { detext, orignal }
    }

    pub fn get_data(self) -> String {
        match self.detext {
            Some(dtext) => dtext,
            None => self.orignal,
        }
    }
}

impl Http {
    pub fn new(passwrd: Option<String>) -> Result<Self> {
        let http = Self {
            passwrd,
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent(USER_AGENT)
                .build()
                .map_err(|e| {
                    error!("创建http客户端失败:{}", e);
                    HttpError::RequestError
                })?,
        };
        Ok(http)
    }

    fn fix_url(&self, url: &str) -> String {
        if !url.starts_with(HTTPS_PREFIX) && !url.starts_with(HTTP_PREFIX) {
            let base = setting::BASE_URL.trim_end_matches('/');
            let cargo = setting::CARGO_URL.trim_end_matches('/');
            let path = url.trim_start_matches('/');
            return format!("{}/{}/{}", base, cargo, path);
        }
        return url.to_string();
    }

    pub async fn fetch(&self, url: &str) -> Result<HttpResult> {
        let url = self.fix_url(url);
        let response = self.client.get(&url).send().await.map_err(|e| {
            if e.is_timeout() {
                error!("请求超时:{}", e);
                HttpError::TimeoutError
            } else {
                error!("请求失败:{}", e);
                HttpError::RequestError
            }
        })?;

        if !response.status().is_success() {
            error!("请求状态码错误：{},url:{}", response.status(), url);
            return Err(HttpError::RequestStatusError(response.status().to_string()).into());
        }

        let r = response.text().await.map_err(|e| {
            error!("请求失败:{}", e);
            HttpError::RequestError
        })?;
        if let Some(passwrd) = self.passwrd.as_ref() {
            let base64 = Base64::new(passwrd.as_str());
            if let Ok(base64) = base64 {
                let dtext = base64.decode(&r)?;
                return Ok(HttpResult::new(Some(dtext), r));
            }
        }
        Ok(HttpResult::new(None, r))
    }
}
