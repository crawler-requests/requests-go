use reqwest::header::HeaderMap;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use url::Url;

use crate::error::Result;
use crate::transport::{H2Settings, TLSExtensions};

pub fn percent_encode(input: &str) -> String {
    let mut output = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(byte as char);
            }
            _ => {
                output.push('%');
                output.push_str(&format!("{:02X}", byte));
            }
        }
    }
    output
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestParams {
    pub params: HashMap<String, Vec<String>>,
}

impl RequestParams {
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
        }
    }

    pub fn add(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.params.entry(key.into()).or_default().push(value.into());
    }

    pub fn encode(&self) -> String {
        let mut parts = Vec::new();
        for (key, values) in &self.params {
            for value in values {
                parts.push(format!(
                    "{}={}",
                    percent_encode(key),
                    percent_encode(value)
                ));
            }
        }
        parts.join("&")
    }
}

impl Default for RequestParams {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Request {
    pub method: Method,
    pub url: Url,
    pub params: Option<RequestParams>,
    pub headers: HeaderMap,
    pub cookies: Arc<reqwest::cookie::Jar>,
    pub data: Option<RequestParams>,
    pub files: Vec<FileUpload>,
    pub json: Option<serde_json::Value>,
    pub body: Option<Vec<u8>>,
    pub auth: Option<(String, String)>,
    pub stream: bool,
    pub allow_redirects: bool,
    pub tls_extensions: Option<TLSExtensions>,
    pub http2_settings: Option<H2Settings>,
    pub cert: Vec<String>,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            method: Method::GET,
            url: Url::parse("http://localhost").expect("default url"),
            params: None,
            headers: HeaderMap::new(),
            cookies: Arc::new(reqwest::cookie::Jar::default()),
            data: None,
            files: Vec::new(),
            json: None,
            body: None,
            auth: None,
            stream: false,
            allow_redirects: true,
            tls_extensions: None,
            http2_settings: None,
            cert: Vec::new(),
        }
    }
}

impl Request {
    pub fn new(method: Method, url: impl AsRef<str>) -> Result<Self> {
        let url = Url::parse(url.as_ref())?;
        Ok(Self {
            method,
            url,
            ..Default::default()
        })
    }

    pub fn get(url: impl AsRef<str>) -> Result<Self> {
        Self::new(Method::GET, url)
    }

    pub fn post(url: impl AsRef<str>) -> Result<Self> {
        Self::new(Method::POST, url)
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FileUploadInput {
    pub name: String,
    pub filename: String,
    pub content_type: String,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct FileUpload {
    pub name: String,
    pub filename: String,
    pub content_type: String,
    pub path: std::path::PathBuf,
}

#[derive(Debug, Clone)]
pub struct Response {
    pub url: Url,
    pub headers: HeaderMap,
    pub cookies: Vec<(String, String)>,
    pub content: Vec<u8>,
    pub body: Option<Vec<u8>>,
    pub status_code: reqwest::StatusCode,
    pub history: Vec<Response>,
    pub raw_response: Vec<u8>,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            url: Url::parse("http://localhost").expect("default url"),
            headers: HeaderMap::new(),
            cookies: Vec::new(),
            content: Vec::new(),
            body: None,
            status_code: reqwest::StatusCode::from_u16(200).unwrap(),
            history: Vec::new(),
            raw_response: Vec::new(),
        }
    }
}

impl Response {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.content).to_string()
    }

    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        Ok(serde_json::from_slice(&self.content)?)
    }

    pub fn ok(&self) -> bool {
        self.status_code.is_success() || self.status_code.is_redirection()
    }

    pub fn is_redirect(&self) -> bool {
        self.status_code.is_redirection()
    }

    pub fn raise_for_status(&self) -> Result<()> {
        if self.status_code.is_client_error() {
            return Err(crate::RequestsError::Http(format!(
                "{} Client Error",
                self.status_code.as_u16()
            )));
        }
        if self.status_code.is_server_error() {
            return Err(crate::RequestsError::Http(format!(
                "{} Server Error",
                self.status_code.as_u16()
            )));
        }
        Ok(())
    }
}

impl From<Response> for FfiResponseOutput {
    fn from(resp: Response) -> Self {
        let headers = resp
            .headers
            .iter()
            .map(|(k, v)| {
                let vals = vec![String::from_utf8_lossy(v.as_bytes()).to_string()];
                (k.clone().to_string(), vals)
            })
            .collect();
        Self {
            url: resp.url.to_string(),
            status_code: resp.status_code.as_u16(),
            headers,
            cookies: resp.cookies,
            content: resp.content,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FfiResponseOutput {
    pub url: String,
    pub status_code: u16,
    pub headers: std::collections::HashMap<String, Vec<String>>,
    pub cookies: Vec<(String, String)>,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct FfiRequestInput {
    pub url: String,
    pub method: Option<String>,
    pub timeout: Option<u64>,
    pub verify: Option<bool>,
    pub proxy: Option<String>,
    pub ja3: Option<String>,
    pub random_ja3: bool,
    pub http1: bool,
    pub http2_settings: Option<String>,
    pub browser_profile: Option<(String, String)>,
    pub allow_redirects: Option<bool>,
    pub cert: Option<Vec<String>>,
    pub params: Option<std::collections::HashMap<String, String>>,
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub cookies: Option<std::collections::HashMap<String, String>>,
    pub data: Option<std::collections::HashMap<String, String>>,
    pub files: Option<Vec<FileUploadInput>>,
    pub json: Option<serde_json::Value>,
    pub body: Option<String>,
    pub auth: Option<Vec<String>>,
    pub stream: bool,
    pub pool_max_idle_per_host: Option<usize>,
    pub pool_idle_timeout: Option<u64>,
    pub tcp_keepalive: Option<u64>,
    pub cookie_store_path: Option<String>,
    pub user_agent: Option<String>,
    pub accept: Option<String>,
    pub accept_encoding: Option<String>,
    pub accept_language: Option<String>,
    pub content_type: Option<String>,
    pub referer: Option<String>,
    pub basic_auth: Option<Vec<String>>,
    pub bearer_token: Option<String>,
    pub max_response_size: Option<usize>,
    pub disable_cookie_jar: Option<bool>,
    pub local_address: Option<String>,
    pub interface: Option<String>,
    pub tcp_nodelay: Option<bool>,
    pub connect_timeout: Option<u64>,
    pub retry_backoff: Option<bool>,
    pub user_agent_pool: Option<Vec<String>>,
    pub dns_cache: Option<bool>,
    pub dns_servers: Option<Vec<String>>,
    pub http2_header_order: Option<Vec<String>>,
    pub connection_reuse: Option<bool>,
    pub tls_min_version: Option<String>,
    pub tls_max_version: Option<String>,
    pub tls_alpn: Option<Vec<String>>,
    /// libcurl-impersonate target name, e.g. "chrome136", "firefox135", "safari18_0".
    pub impersonate: Option<String>,
    /// Akamai HTTP/2 fingerprint: `settings|window_update|streams|pseudo_header_order`.
    pub akamai: Option<String>,
    /// When false, curl_easy_impersonate skips its built-in browser headers.
    pub impersonate_default_headers: Option<bool>,
    // 重试配置
    pub retry: Option<usize>,
    pub retry_delay: Option<u64>,
    // HTTP/2 窗口配置
    pub http2_window_size: Option<u32>,
    pub http2_connection_window_size: Option<u32>,
    pub http2_max_frame_size: Option<u32>,
    // TCP keepalive 间隔
    pub tcp_keepalive_interval: Option<u64>,
}

impl Default for FfiRequestInput {
    fn default() -> Self {
        Self {
            url: String::new(),
            method: None,
            timeout: None,
            verify: None,
            proxy: None,
            ja3: None,
            random_ja3: false,
            http1: false,
            http2_settings: None,
            browser_profile: None,
            allow_redirects: None,
            cert: None,
            params: None,
            headers: None,
            cookies: None,
            data: None,
            files: None,
            json: None,
            body: None,
            auth: None,
            stream: false,
            pool_max_idle_per_host: None,
            pool_idle_timeout: None,
            tcp_keepalive: None,
            cookie_store_path: None,
            user_agent: None,
            accept: None,
            accept_encoding: None,
            accept_language: None,
            content_type: None,
            referer: None,
            basic_auth: None,
            bearer_token: None,
            max_response_size: None,
            disable_cookie_jar: None,
            local_address: None,
            interface: None,
            tcp_nodelay: None,
            connect_timeout: None,
            retry_backoff: None,
            user_agent_pool: None,
            dns_cache: None,
            dns_servers: None,
            http2_header_order: None,
            connection_reuse: None,
            tls_min_version: None,
            tls_max_version: None,
            tls_alpn: None,
            impersonate: None,
            akamai: None,
            impersonate_default_headers: None,
            retry: None,
            retry_delay: None,
            http2_window_size: None,
            http2_connection_window_size: None,
            http2_max_frame_size: None,
            tcp_keepalive_interval: None,
        }
    }
}
