use thiserror::Error;

#[derive(Error, Debug)]
pub enum RequestsError {
    #[error("HTTP error: {0}")]
    Http(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid method: {0}")]
    InvalidMethod(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Address parse error: {0}")]
    AddrParse(#[from] std::net::AddrParseError),

    #[error("Redirect limit exceeded: {0}")]
    RedirectLimit(usize),

    #[error("Proxy error: {0}")]
    Proxy(String),

    #[error("TLS error: {0}")]
    Tls(String),

    #[error("Streaming not supported in current context")]
    StreamingNotSupported,
}

impl From<reqwest::Error> for RequestsError {
    fn from(err: reqwest::Error) -> Self {
        RequestsError::Http(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, RequestsError>;
