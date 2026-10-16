use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

use crate::error::Result;
use crate::models::{Request, Response};
use crate::session::Session;
use crate::curl_impersonate::BrowserProfile;

/// High-level request client built on top of `Session`.
///
/// This wrapper is designed for the common anti-crawling / browser-emulation
/// workflow: configure once, then call `get` / `post` / `request` without
/// rebuilding the session each time.
#[derive(Clone)]
pub struct HttpClient {
    session: Arc<Session>,
}

impl HttpClient {
    /// Create a new client with default settings.
    pub fn new() -> Self {
        Self {
            session: Arc::new(Session::new()),
        }
    }

    /// Set a global request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.session = Arc::new(Session::with_timeout(Arc::unwrap_or_clone(self.session), timeout));
        self
    }

    /// Set a global proxy URL (e.g. `http://127.0.0.1:7890`).
    pub fn with_proxy(mut self, proxy: impl Into<String>) -> Self {
        self.session = Arc::new(Session::with_proxy(Arc::unwrap_or_clone(self.session), proxy));
        self
    }

    /// Disable TLS certificate verification.
    pub fn with_insecure(mut self) -> Self {
        self.session = Arc::new(Session::with_verify(Arc::unwrap_or_clone(self.session), false));
        self
    }

    /// Enable TLS certificate verification.
    pub fn with_verify(mut self, verify: bool) -> Self {
        self.session = Arc::new(Session::with_verify(Arc::unwrap_or_clone(self.session), verify));
        self
    }

    /// Set a custom JA3 fingerprint string.
    pub fn with_ja3(mut self, ja3: impl Into<String>) -> Self {
        self.session = Arc::new(Session::with_ja3(Arc::unwrap_or_clone(self.session), ja3));
        self
    }

    /// Enable randomized JA3 fingerprinting.
    pub fn with_random_ja3(mut self, enabled: bool) -> Self {
        self.session = Arc::new(Session::with_random_ja3(Arc::unwrap_or_clone(self.session), enabled));
        self
    }

    /// Force HTTP/1.1 only.
    pub fn with_http1(mut self) -> Self {
        self.session = Arc::new(Session::with_force_http1(Arc::unwrap_or_clone(self.session), true));
        self
    }

    /// Set HTTP/2 settings as a JSON string.
    pub fn with_http2_settings(mut self, settings: impl Into<String>) -> Self {
        self.session = Arc::new(Session::with_http2_settings(Arc::unwrap_or_clone(self.session), settings));
        self
    }

    /// Set a curl-impersonate browser profile for fingerprint emulation.
    pub fn with_browser_profile(mut self, profile: BrowserProfile, binary: impl Into<PathBuf>) -> Self {
        let inner = Arc::unwrap_or_clone(self.session);
        self.session = Arc::new(Session::with_browser_profile(inner, profile, binary));
        self
    }

    /// Set global retry count.
    pub fn with_retry(mut self, count: usize) -> Self {
        self.session = Arc::new(Session::with_retry(Arc::unwrap_or_clone(self.session), count));
        self
    }

    /// Set global retry delay.
    pub fn with_retry_delay(mut self, delay: Duration) -> Self {
        self.session = Arc::new(Session::with_retry_delay(Arc::unwrap_or_clone(self.session), delay));
        self
    }

    /// Disable automatic redirects.
    pub fn with_no_redirects(mut self) -> Self {
        self.session = Arc::new(Session::with_allow_redirects(Arc::unwrap_or_clone(self.session), false));
        self
    }

    /// Set client certificates for mutual TLS.
    pub fn with_cert(mut self, cert: Vec<String>) -> Self {
        self.session = Arc::new(Session::with_cert(Arc::unwrap_or_clone(self.session), cert));
        self
    }

    /// Configure connection pool max idle connections per host.
    pub fn with_pool_max_idle_per_host(mut self, max: usize) -> Self {
        self.session = Arc::new(Session::with_pool_max_idle_per_host(Arc::unwrap_or_clone(self.session), max));
        self
    }

    /// Configure connection pool idle timeout.
    pub fn with_pool_idle_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.session = Arc::new(Session::with_pool_idle_timeout(Arc::unwrap_or_clone(self.session), timeout));
        self
    }

    /// Configure TCP keepalive interval.
    pub fn with_tcp_keepalive(mut self, interval: std::time::Duration) -> Self {
        self.session = Arc::new(Session::with_tcp_keepalive(Arc::unwrap_or_clone(self.session), interval));
        self
    }

    /// Enable cookie persistence to a file.
    pub fn with_cookie_store_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.session = Arc::new(Session::with_cookie_store_path(Arc::unwrap_or_clone(self.session), path));
        self
    }



    /// Load cookies from the configured cookie store path.
    pub fn load_cookies(&self) -> Result<Option<Vec<(String, String)>>> {
        self.session.load_cookies()
    }

    /// Save cookies to the configured cookie store path.
    pub fn save_cookies(&self, cookies: &[(String, String)]) -> Result<()> {
        self.session.save_cookies(cookies)
    }

    /// Add a default header for all requests sent from this client.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let mut session = Arc::unwrap_or_clone(self.session);
        session.headers.insert(
            reqwest::header::HeaderName::from_bytes(key.into().as_bytes()).unwrap(),
            reqwest::header::HeaderValue::from_str(&value.into()).unwrap(),
        );
        self.session = Arc::new(session);
        self
    }

    /// Set default cookies for all requests.
    pub fn with_cookies(mut self, cookies: &[(String, String)]) -> Self {
        let session = Arc::unwrap_or_clone(self.session);
        for (k, v) in cookies {
            session.cookies.add_cookie_str(
                &format!("{}={}", k, v),
                &Url::parse("http://localhost").expect("default url"),
            );
        }
        self.session = Arc::new(session);
        self
    }

    /// Register a middleware into the session pipeline.
    pub fn with_middleware<F>(mut self, middleware: F) -> Self
    where
        F: Fn(crate::session::MiddlewareContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::error::Result<Response>> + Send>> + Send + Sync + 'static,
    {
        let mut session = Arc::unwrap_or_clone(self.session);
        session.use_middleware(middleware);
        self.session = Arc::new(session);
        self
    }

    /// Send a fully customized request.
    pub async fn request(&self, method: reqwest::Method, url: impl AsRef<str>, req: Request) -> Result<Response> {
        self.session.request(method, url, req).await
    }

    /// Send a GET request.
    pub async fn get(&self, url: impl AsRef<str>) -> Result<Response> {
        self.session.get(url).await
    }

    /// Send a POST request.
    pub async fn post(&self, url: impl AsRef<str>) -> Result<Response> {
        self.session.post(url).await
    }

    /// Send a PUT request.
    pub async fn put(&self, url: impl AsRef<str>) -> Result<Response> {
        self.session.request(reqwest::Method::PUT, url, Request::default()).await
    }

    /// Send a PATCH request.
    pub async fn patch(&self, url: impl AsRef<str>) -> Result<Response> {
        self.session.request(reqwest::Method::PATCH, url, Request::default()).await
    }

    /// Send a DELETE request.
    pub async fn delete(&self, url: impl AsRef<str>) -> Result<Response> {
        self.session.request(reqwest::Method::DELETE, url, Request::default()).await
    }

    /// Send a HEAD request.
    pub async fn head(&self, url: impl AsRef<str>) -> Result<Response> {
        self.session.request(reqwest::Method::HEAD, url, Request::default()).await
    }

    /// Send an OPTIONS request.
    pub async fn options(&self, url: impl AsRef<str>) -> Result<Response> {
        self.session.request(reqwest::Method::OPTIONS, url, Request::default()).await
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}
