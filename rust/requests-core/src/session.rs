use reqwest::Client;
use reqwest::Method;
use reqwest::Proxy;
use std::sync::Arc;
use std::path::PathBuf;
use std::time::Duration;
use url::Url;
use crate::error::Result;
use base64::Engine;
use crate::models::{Request, Response};
use crate::tls::build_rustls_client_config;
use crate::curl_impersonate::{CurlImpersonateConfig, BrowserProfile};

#[derive(Debug, Clone)]
pub struct MiddlewareContext {
    pub session: Arc<Session>,
    pub request: Request,
}

pub type MiddlewareFn =
    Arc<dyn Fn(MiddlewareContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response>> + Send>> + Send + Sync>;

#[derive(Clone)]
pub struct Session {
    pub params: std::collections::HashMap<String, Vec<String>>,
    pub headers: reqwest::header::HeaderMap,
    pub cookies: Arc<reqwest::cookie::Jar>,
    pub auth: Option<(String, String)>,
    pub timeout: std::time::Duration,
    pub proxies: Option<String>,
    pub verify: bool,
    pub cert: Vec<String>,
    pub ja3: Option<String>,
    pub random_ja3: bool,
    pub force_http1: bool,
    pub tls_extensions: Option<String>,
    pub http2_settings: Option<String>,
    pub max_redirects: usize,
    pub middlewares: Vec<MiddlewareFn>,
    pub curl_impersonate_binary: Option<PathBuf>,
    pub browser_profile: Option<BrowserProfile>,
    /// libcurl-impersonate target, e.g. "chrome136". Takes the in-process path.
    pub impersonate: Option<String>,
    /// Akamai HTTP/2 fingerprint (`settings|window|streams|priority`).
    pub akamai: Option<String>,
    /// Apply the impersonate target's default browser headers.
    pub impersonate_default_headers: bool,
    pub retry: usize,
    pub retry_delay: Duration,
    pub retry_backoff: bool,
    pub allow_redirects: bool,
    pub pool_max_idle_per_host: Option<usize>,
    pub pool_idle_timeout: Option<Duration>,
    pub tcp_keepalive: Option<Duration>,
    pub cookie_store_path: Option<PathBuf>,
    pub connect_timeout: Option<Duration>,
    pub http2_window_size: Option<u32>,
    pub http2_connection_window_size: Option<u32>,
    pub http2_max_frame_size: Option<u32>,
    pub user_agent: Option<String>,
    pub accept: Option<String>,
    pub accept_encoding: Option<String>,
    pub accept_language: Option<String>,
    pub content_type: Option<String>,
    pub referer: Option<String>,
    pub basic_auth: Option<(String, String)>,
    pub bearer_token: Option<String>,
    pub max_response_size: Option<usize>,
    pub disable_cookie_jar: bool,
    pub local_address: Option<String>,
    pub interface: Option<String>,
    pub tcp_nodelay: bool,
    pub tcp_keepalive_interval: Option<Duration>,
    pub user_agent_pool: Option<Vec<String>>,
    pub dns_cache: bool,
    pub dns_servers: Option<Vec<String>>,
    pub http2_header_order: Option<Vec<String>>,
    pub connection_reuse: bool,
    pub tls_min_version: Option<String>,
    pub tls_max_version: Option<String>,
    pub tls_alpn: Option<Vec<String>>,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("params", &self.params)
            .field("timeout", &self.timeout)
            .field("verify", &self.verify)
            .field("max_redirects", &self.max_redirects)
            .finish_non_exhaustive()
    }
}

impl Session {
    pub fn new() -> Self {
        Self {
            params: std::collections::HashMap::new(),
            headers: reqwest::header::HeaderMap::new(),
            cookies: Arc::new(reqwest::cookie::Jar::default()),
            auth: None,
            timeout: std::time::Duration::from_secs(30),
            proxies: None,
            verify: true,
            cert: Vec::new(),
            ja3: None,
            random_ja3: false,
            force_http1: false,
            tls_extensions: None,
            http2_settings: None,
            max_redirects: 30,
            middlewares: Vec::new(),
            curl_impersonate_binary: None,
            browser_profile: None,
            impersonate: None,
            akamai: None,
            impersonate_default_headers: true,
            retry: 0,
            retry_delay: Duration::from_secs(0),
            retry_backoff: false,
            allow_redirects: true,
            pool_max_idle_per_host: None,
            pool_idle_timeout: None,
            tcp_keepalive: None,
            cookie_store_path: None,
            connect_timeout: None,
            http2_window_size: None,
            http2_connection_window_size: None,
            http2_max_frame_size: None,
            user_agent: None,
            accept: None,
            accept_encoding: None,
            accept_language: None,
            content_type: None,
            referer: None,
            basic_auth: None,
            bearer_token: None,
            max_response_size: None,
            disable_cookie_jar: false,
            local_address: None,
            interface: None,
            tcp_nodelay: false,
            tcp_keepalive_interval: None,
            user_agent_pool: None,
            dns_cache: false,
            dns_servers: None,
            http2_header_order: None,
            connection_reuse: true,
            tls_min_version: None,
            tls_max_version: None,
            tls_alpn: None,
        }
    }

    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_proxy(mut self, proxy: impl Into<String>) -> Self {
        self.proxies = Some(proxy.into());
        self
    }

    pub fn with_verify(mut self, verify: bool) -> Self {
        self.verify = verify;
        self
    }

    pub fn with_ja3(mut self, ja3: impl Into<String>) -> Self {
        self.ja3 = Some(ja3.into());
        self
    }

    pub fn with_random_ja3(mut self, random: bool) -> Self {
        self.random_ja3 = random;
        self
    }

    pub fn with_force_http1(mut self, force: bool) -> Self {
        self.force_http1 = force;
        self
    }

    pub fn with_browser_profile(mut self, profile: BrowserProfile, binary_path: impl Into<PathBuf>) -> Self {
        self.browser_profile = Some(profile);
        self.curl_impersonate_binary = Some(binary_path.into());
        if self.impersonate.is_none() {
            self.impersonate = Some(profile.profile_name().to_string());
        }
        self
    }

    /// Use libcurl-impersonate inside the process. `target` is a curl-impersonate
    /// profile such as `chrome136`, `firefox135`, `safari18_0`.
    pub fn with_impersonate(mut self, target: impl Into<String>) -> Self {
        self.impersonate = Some(target.into());
        self
    }

    /// Akamai HTTP/2 fingerprint: `settings|window_update|streams|pseudo_order`.
    pub fn with_akamai(mut self, akamai: impl Into<String>) -> Self {
        self.akamai = Some(akamai.into());
        self
    }

    pub fn with_impersonate_default_headers(mut self, enabled: bool) -> Self {
        self.impersonate_default_headers = enabled;
        self
    }

    pub fn with_curl_impersonate(self, binary_path: impl Into<PathBuf>, profile: BrowserProfile) -> Self {
        self.with_browser_profile(profile, binary_path)
    }

    pub fn with_http2_settings(mut self, settings: impl Into<String>) -> Self {
        self.http2_settings = Some(settings.into());
        self
    }

    pub fn with_max_redirects(mut self, max: usize) -> Self {
        self.max_redirects = max;
        self
    }

    pub fn with_retry(mut self, retry: usize) -> Self {
        self.retry = retry;
        self
    }

    pub fn with_retry_delay(mut self, delay: std::time::Duration) -> Self {
        self.retry_delay = delay;
        self
    }

    pub fn with_allow_redirects(mut self, allow: bool) -> Self {
        self.allow_redirects = allow;
        self
    }

    pub fn with_pool_max_idle_per_host(mut self, max: usize) -> Self {
        self.pool_max_idle_per_host = Some(max);
        self
    }

    pub fn with_pool_idle_timeout(mut self, timeout: Duration) -> Self {
        self.pool_idle_timeout = Some(timeout);
        self
    }

    pub fn with_tcp_keepalive(mut self, interval: Duration) -> Self {
        self.tcp_keepalive = Some(interval);
        self
    }

    pub fn with_cookie_store_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.cookie_store_path = Some(path.into());
        self
    }

    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    pub fn with_http2_window_size(mut self, size: u32) -> Self {
        self.http2_window_size = Some(size);
        self
    }

    pub fn with_http2_connection_window_size(mut self, size: u32) -> Self {
        self.http2_connection_window_size = Some(size);
        self
    }

    pub fn with_http2_max_frame_size(mut self, size: u32) -> Self {
        self.http2_max_frame_size = Some(size);
        self
    }

    pub fn with_user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    pub fn with_accept(mut self, accept: impl Into<String>) -> Self {
        self.accept = Some(accept.into());
        self
    }

    pub fn with_accept_encoding(mut self, encoding: impl Into<String>) -> Self {
        self.accept_encoding = Some(encoding.into());
        self
    }

    pub fn with_accept_language(mut self, language: impl Into<String>) -> Self {
        self.accept_language = Some(language.into());
        self
    }

    pub fn with_content_type(mut self, ct: impl Into<String>) -> Self {
        self.content_type = Some(ct.into());
        self
    }

    pub fn with_referer(mut self, referer: impl Into<String>) -> Self {
        self.referer = Some(referer.into());
        self
    }

    pub fn with_basic_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.basic_auth = Some((username.into(), password.into()));
        self
    }

    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.bearer_token = Some(token.into());
        self
    }

    pub fn with_max_response_size(mut self, size: usize) -> Self {
        self.max_response_size = Some(size);
        self
    }

    pub fn with_disable_cookie_jar(mut self, disable: bool) -> Self {
        self.disable_cookie_jar = disable;
        self
    }

    pub fn with_local_address(mut self, addr: impl Into<String>) -> Self {
        self.local_address = Some(addr.into());
        self
    }

    pub fn with_interface(mut self, iface: impl Into<String>) -> Self {
        self.interface = Some(iface.into());
        self
    }

    pub fn with_tcp_nodelay(mut self, enabled: bool) -> Self {
        self.tcp_nodelay = enabled;
        self
    }

    pub fn with_tcp_keepalive_interval(mut self, interval: Duration) -> Self {
        self.tcp_keepalive_interval = Some(interval);
        self
    }

    pub fn with_retry_backoff(mut self, backoff: bool) -> Self {
        self.retry_backoff = backoff;
        self
    }

    pub fn with_user_agent_pool(mut self, pool: Vec<String>) -> Self {
        self.user_agent_pool = Some(pool);
        self
    }

    pub fn with_dns_cache(mut self, enabled: bool) -> Self {
        self.dns_cache = enabled;
        self
    }

    pub fn with_dns_servers(mut self, servers: Vec<String>) -> Self {
        self.dns_servers = Some(servers);
        self
    }

    pub fn with_http2_header_order(mut self, order: Vec<String>) -> Self {
        self.http2_header_order = Some(order);
        self
    }

    pub fn with_connection_reuse(mut self, enabled: bool) -> Self {
        self.connection_reuse = enabled;
        self
    }

    pub fn with_tls_min_version(mut self, version: impl Into<String>) -> Self {
        self.tls_min_version = Some(version.into());
        self
    }

    pub fn with_tls_max_version(mut self, version: impl Into<String>) -> Self {
        self.tls_max_version = Some(version.into());
        self
    }

    pub fn with_tls_alpn(mut self, alpn: Vec<String>) -> Self {
        self.tls_alpn = Some(alpn);
        self
    }


    pub fn load_cookies(&self) -> Result<Option<Vec<(String, String)>>> {
        let path = match &self.cookie_store_path {
            Some(p) => p,
            None => return Ok(None),
        };
        let data = std::fs::read_to_string(path)?;
        let cookies: Vec<(String, String)> = serde_json::from_str(&data)?;
        Ok(Some(cookies))
    }

    /// Save cookies from a provided list of `(name, value)` pairs to the configured path.
    pub fn save_cookies(&self, cookies: &[(String, String)]) -> Result<()> {
        let path = match &self.cookie_store_path {
            Some(p) => p,
            None => return Ok(()),
        };
        let data = serde_json::to_string_pretty(&cookies)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Extract `Set-Cookie` headers from a response and persist them to the cookie store file.
    pub fn save_response_cookies(&self, headers: &reqwest::header::HeaderMap) -> Result<()> {
        let path = match &self.cookie_store_path {
            Some(p) => p,
            None => return Ok(()),
        };
        let mut cookies = Vec::new();
        for value in headers.get_all(reqwest::header::SET_COOKIE) {
            let header_str = String::from_utf8_lossy(value.as_bytes());
            if let Some((name, rest)) = header_str.split_once('=') {
                let value = rest.split(';').next().unwrap_or("").trim();
                if !name.is_empty() {
                    cookies.push((name.to_string(), value.to_string()));
                }
            }
        }
        if cookies.is_empty() {
            return Ok(());
        }
        let data = serde_json::to_string_pretty(&cookies)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    pub fn ensure_cookies_loaded(&self) {
        if let Ok(Some(cookies)) = self.load_cookies() {
            for (name, value) in cookies {
                let _ = self.cookies.add_cookie_str(
                    &format!("{}={}", name, value),
                    &Url::parse("http://localhost").expect("default url"),
                );
            }
        }
    }

    pub fn with_cert(mut self, cert: Vec<String>) -> Self {
        self.cert = cert;
        self
    }

    pub fn use_middleware<F>(&mut self, middleware: F)
    where
        F: Fn(MiddlewareContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response>> + Send>> + Send + Sync + 'static,
    {
        self.middlewares.push(Arc::new(middleware));
    }

    async fn build_client(&self) -> Result<Client> {
        self.ensure_cookies_loaded();

        let mut cookie_provider = None;
        if !self.disable_cookie_jar {
            cookie_provider = Some(self.cookies.clone());
        }

        let mut client_builder = Client::builder()
            .timeout(self.timeout);

        if let Some(connect_timeout) = self.connect_timeout {
            client_builder = client_builder.connect_timeout(connect_timeout);
        }

        if let Some(cookie_jar) = cookie_provider {
            client_builder = client_builder.cookie_provider(cookie_jar);
        }

        if !self.allow_redirects {
            client_builder = client_builder.redirect(reqwest::redirect::Policy::none());
        } else {
            client_builder = client_builder.redirect(reqwest::redirect::Policy::limited(self.max_redirects));
        }

        if !self.verify {
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }

        if let Some(proxy_url) = &self.proxies {
            client_builder = client_builder.proxy(Proxy::all(proxy_url)?);
        }

        if self.force_http1 {
            client_builder = client_builder.http1_only();
        }

        if self.tcp_nodelay {
            client_builder = client_builder.tcp_nodelay(true);
        }

        if let Some(addr) = &self.local_address {
            client_builder = client_builder.local_address(addr.parse::<std::net::IpAddr>()?);
        }

        if let Some(iface) = &self.interface {
            client_builder = client_builder.interface(iface);
        }

        if let Some(window) = self.http2_window_size {
            client_builder = client_builder.http2_initial_stream_window_size(window);
        }
        if let Some(conn_window) = self.http2_connection_window_size {
            client_builder = client_builder.http2_initial_connection_window_size(conn_window);
        }
        if let Some(frame_size) = self.http2_max_frame_size {
            client_builder = client_builder.http2_max_frame_size(frame_size);
        }

        // Apply custom DNS resolver when explicit DNS servers are configured.
        if let Some(servers) = &self.dns_servers {
            if !servers.is_empty() {
                let mut resolver = crate::dns::HickoryDnsResolver::new(servers.clone());
                if self.dns_cache {
                    resolver = resolver.with_cache(std::sync::Arc::new(crate::dns::DnsCache::new()));
                }
                client_builder = client_builder.dns_resolver(std::sync::Arc::new(resolver));
            }
        }

        // Apply rustls TLS config when fingerprinting is requested.
        // Otherwise rely on reqwest's default TLS stack.
        if self.ja3.is_some() || self.tls_extensions.is_some() {
            let ja3_fp = crate::transport::JA3Fingerprint::new(
                self.ja3.clone(),
                self.random_ja3,
                self.tls_extensions.clone(),
            );
            let tls_cfg = build_rustls_client_config(
                self.verify,
                &Some(ja3_fp),
                &crate::transport::TLSExtensions::from_json(self.tls_extensions.as_deref()),
                &self.tls_min_version,
                &self.tls_max_version,
                &self.tls_alpn,
            )?;
            client_builder = client_builder.use_preconfigured_tls(tls_cfg);
        }

        if let Some(max_idle) = self.pool_max_idle_per_host {
            client_builder = client_builder.pool_max_idle_per_host(max_idle);
        }
        if let Some(idle_timeout) = self.pool_idle_timeout {
            client_builder = client_builder.pool_idle_timeout(idle_timeout);
        }
        if let Some(keepalive) = self.tcp_keepalive {
            client_builder = client_builder.tcp_keepalive(keepalive);
        }
        if let Some(keepalive_interval) = self.tcp_keepalive_interval {
            client_builder = client_builder.tcp_keepalive_interval(keepalive_interval);
        }

        // DNS / connection reuse settings
        if !self.connection_reuse {
            client_builder = client_builder.pool_max_idle_per_host(0);
        }

        client_builder.build().map_err(|e| crate::RequestsError::Http(format!("build client failed: {}", e)))
    }

    fn akamai_parts(&self) -> (Option<String>, Option<i64>, Option<String>, Option<String>) {
        let Some(raw) = &self.akamai else {
            return (None, None, None, None);
        };
        let mut parts = raw.split('|');
        let settings = parts.next().filter(|s| !s.is_empty()).map(|s| s.to_string());
        let window = parts.next().and_then(|s| s.parse::<i64>().ok());
        let streams = parts.next().filter(|s| !s.is_empty()).map(|s| s.to_string());
        let order = parts.next().filter(|s| !s.is_empty()).map(|s| s.to_string());
        (settings, window, streams, order)
    }

    fn send_via_libcurl(&self, req: &Request, headers: &[String], body: Option<&[u8]>) -> Result<Response> {
        let target = self.impersonate.clone().or_else(|| {
            self.browser_profile.map(|p| p.profile_name().to_string())
        });
        let (h2_settings, h2_window, h2_streams, h2_order) = self.akamai_parts();
        let url_owned = req.url.to_string();
        let resp = crate::libcurl_impersonate::perform(&crate::libcurl_impersonate::EasyRequest {
            method: req.method.as_str(),
            url: &url_owned,
            headers,
            body,
            impersonate: target.as_deref(),
            default_headers: self.impersonate_default_headers && headers.is_empty(),
            timeout_secs: self.timeout.as_secs().max(1),
            connect_timeout_secs: self.connect_timeout.map(|d| d.as_secs()),
            proxy: self.proxies.as_deref(),
            verify: self.verify,
            follow: self.allow_redirects,
            max_redirs: self.max_redirects,
            force_http1: self.force_http1,
            interface: self.interface.as_deref(),
            cookie: None,
            user_agent: self.user_agent.as_deref(),
            accept_encoding: self.accept_encoding.as_deref(),
            extra: crate::libcurl_impersonate::ExtraFingerprint {
                ja3: self.ja3.clone(),
                akamai: self.akamai.clone(),
                sig_hash_algs: None,
                cert_compression: None,
                tls_permute_extensions: if self.random_ja3 { Some(true) } else { None },
                tls_grease: None,
                tls_cert_compression: None,
                http2_settings: h2_settings.or_else(|| self.http2_settings.clone()),
                http2_pseudo_header_order: h2_order.or_else(|| {
                    self.http2_header_order.as_ref().map(|v| v.join(","))
                }),
                http2_window_update: h2_window.or_else(|| self.http2_window_size.map(|n| n as i64)),
                http2_streams: h2_streams,
            },
        })?;

        let header_text = String::from_utf8_lossy(&resp.headers_raw);
        let mut header_map = reqwest::header::HeaderMap::new();
        for line in header_text.lines() {
            let line = line.trim_end_matches('\r');
            if let Some((name, value)) = line.split_once(": ") {
                if let (Ok(name), Ok(value)) = (
                    reqwest::header::HeaderName::from_bytes(name.as_bytes()),
                    reqwest::header::HeaderValue::from_str(value),
                ) {
                    header_map.append(name, value);
                }
            }
        }
        let url = Url::parse(&resp.effective_url).unwrap_or_else(|_| req.url.clone());
        Ok(Response {
            url,
            headers: header_map,
            cookies: Vec::new(),
            content: resp.body,
            body: None,
            status_code: reqwest::StatusCode::from_u16(resp.status).unwrap_or(reqwest::StatusCode::OK),
            history: Vec::new(),
            raw_response: resp.headers_raw,
        })
    }

    pub async fn send(&self, req: Request) -> Result<Response> {
        // In-process libcurl-impersonate. This is the curl_cffi path: one easy
        // handle, curl_easy_impersonate, then JA3 / Akamai overrides.
        let wants_libcurl = self.impersonate.is_some() || self.akamai.is_some() || self.browser_profile.is_some();
        if wants_libcurl {
            let url_str = req.url.to_string();
            let mut header_strings: Vec<String> = req
                .headers
                .iter()
                .map(|(k, v)| format!("{}: {}", k, String::from_utf8_lossy(v.as_bytes())))
                .collect();
            if let Some(ua) = &self.user_agent {
                let key = reqwest::header::USER_AGENT.as_str();
                if !header_strings.iter().any(|h| h.to_ascii_lowercase().starts_with(&key.to_ascii_lowercase())) {
                    header_strings.push(format!("{key}: {ua}"));
                }
            }
            if let Some(token) = &self.bearer_token {
                if !header_strings.iter().any(|h| h.to_ascii_lowercase().starts_with("authorization")) {
                    header_strings.push(format!("Authorization: Bearer {token}"));
                }
            }
            let body_owned = req.body.clone();
            let mut last_err = None;
            let attempts = if self.retry == 0 { 1 } else { self.retry };
            for attempt in 0..attempts {
                if attempt > 0 {
                    let delay = if self.retry_backoff {
                        std::time::Duration::from_secs_f64(self.retry_delay.as_secs_f64() * attempt as f64)
                    } else {
                        self.retry_delay
                    };
                    tokio::time::sleep(delay).await;
                }
                let mut headers_for_attempt = header_strings.clone();
                if let Some(pool) = &self.user_agent_pool {
                    if !pool.is_empty() {
                        let ua = &pool[attempt % pool.len()];
                        headers_for_attempt.retain(|h| !h.to_ascii_lowercase().starts_with("user-agent"));
                        headers_for_attempt.push(format!("User-Agent: {ua}"));
                    }
                }
                match self.send_via_libcurl(&req, &headers_for_attempt, body_owned.as_deref()) {
                    Ok(resp) => {
                        let _ = self.save_response_cookies(&resp.headers);
                        return Ok(resp);
                    }
                    Err(e) => last_err = Some(e),
                }
            }
            let _ = url_str;
            return Err(last_err.unwrap_or_else(|| {
                crate::RequestsError::Http("libcurl-impersonate retry failed".into())
            }));
        }

        // Legacy out-of-process curl-impersonate binary. Kept so existing
        // callers that pass an explicit binary still work when the shared
        // library is not installed.
        if let (Some(binary_path), Some(profile)) = (
            &self.curl_impersonate_binary,
            self.browser_profile,
        ) {
            let cfg = CurlImpersonateConfig::new(binary_path.clone(), profile)
                .with_timeout(self.timeout.as_secs());

            let url_str = req.url.to_string();
            let method = req.method.to_string();
            let mut header_strings: Vec<String> = req.headers
                .iter()
                .map(|(k, v)| format!("{}: {}", k, String::from_utf8_lossy(v.as_bytes())))
                .collect();

            // Merge session-level defaults when request-level headers are missing.
            if let Some(ua) = &self.user_agent {
                let key = reqwest::header::USER_AGENT.as_str();
                if !header_strings.iter().any(|h| h.starts_with(key)) {
                    header_strings.push(format!("{}: {}", key, ua));
                }
            }
            if let Some(accept) = &self.accept {
                let key = reqwest::header::ACCEPT.as_str();
                if !header_strings.iter().any(|h| h.starts_with(key)) {
                    header_strings.push(format!("{}: {}", key, accept));
                }
            }
            if let Some(encoding) = &self.accept_encoding {
                let key = reqwest::header::ACCEPT_ENCODING.as_str();
                if !header_strings.iter().any(|h| h.starts_with(key)) {
                    header_strings.push(format!("{}: {}", key, encoding));
                }
            }
            if let Some(ct) = &self.content_type {
                let key = reqwest::header::CONTENT_TYPE.as_str();
                if !header_strings.iter().any(|h| h.starts_with(key)) {
                    header_strings.push(format!("{}: {}", key, ct));
                }
            }
            if let Some((user, pass)) = &self.basic_auth {
                if !header_strings.iter().any(|h| h.starts_with("Authorization")) {
                    let encoded = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", user, pass));
                    header_strings.push(format!("Authorization: Basic {}", encoded));
                }
            }
            if let Some(token) = &self.bearer_token {
                if !header_strings.iter().any(|h| h.starts_with("Authorization")) {
                    header_strings.push(format!("Authorization: Bearer {}", token));
                }
            }
            for (k, v) in req.headers.iter() {
                if k == reqwest::header::COOKIE {
                    header_strings.push(format!("{}: {}", k, String::from_utf8_lossy(v.as_bytes())));
                }
            }

            let body_bytes = req.body.as_deref();

            let mut last_err = None;
            let attempts = if self.retry == 0 { 1 } else { self.retry };
            for attempt in 0..attempts {
                if attempt > 0 {
                    if self.retry_backoff {
                        let backoff_delay = std::time::Duration::from_secs_f64(
                            self.retry_delay.as_secs_f64() * (attempt as f64),
                        );
                        tokio::time::sleep(backoff_delay).await;
                    } else {
                        tokio::time::sleep(self.retry_delay).await;
                    }
                }

                // Rotate UA in curl-impersonate path if pool is configured.
                let mut headers_for_attempt = header_strings.clone();
                if let Some(pool) = &self.user_agent_pool {
                    if !pool.is_empty() {
                        let ua = &pool[attempt % pool.len()];
                        let key = reqwest::header::USER_AGENT.as_str();
                        headers_for_attempt.retain(|h| !h.starts_with(key));
                        headers_for_attempt.push(format!("{}: {}", key, ua));
                    }
                }

                match cfg.execute(&method, &url_str, &headers_for_attempt, body_bytes) {
                    Ok((status_code, headers, body)) => {
                        let resp = Response {
                            url: req.url.clone(),
                            headers,
                            cookies: Vec::new(),
                            content: body,
                            body: None,
                            status_code: reqwest::StatusCode::from_u16(status_code).unwrap_or(reqwest::StatusCode::OK),
                            history: Vec::new(),
                            raw_response: Vec::new(),
                        };
                        let _ = self.save_response_cookies(&resp.headers);
                        return Ok(resp);
                    }
                    Err(e) => last_err = Some(e),
                }
            }
            return Err(last_err.unwrap_or_else(|| crate::RequestsError::Http("curl-impersonate retry failed".into())));
        }

        // Normal reqwest path with retry
        let client = self.build_client().await?;
        let session = Arc::new(self.clone());

        // Merge settings: request-level takes precedence, fallback to session defaults.
        let mut req = req;
        if req.headers.is_empty() {
            if let Some(ua) = &session.user_agent {
                req.headers.insert(
                    reqwest::header::USER_AGENT,
                    reqwest::header::HeaderValue::from_str(ua.as_str()).unwrap_or_else(|_| reqwest::header::HeaderValue::from_static("")),
                );
            }
            if let Some(accept) = &session.accept {
                req.headers.insert(
                    reqwest::header::ACCEPT,
                    reqwest::header::HeaderValue::from_str(accept.as_str()).unwrap_or_else(|_| reqwest::header::HeaderValue::from_static("")),
                );
            }
            if let Some(encoding) = &session.accept_encoding {
                req.headers.insert(
                    reqwest::header::ACCEPT_ENCODING,
                    reqwest::header::HeaderValue::from_str(encoding.as_str()).unwrap_or_else(|_| reqwest::header::HeaderValue::from_static("")),
                );
            }
            if let Some(ct) = &session.content_type {
                req.headers.insert(
                    reqwest::header::CONTENT_TYPE,
                    reqwest::header::HeaderValue::from_str(ct.as_str()).unwrap_or_else(|_| reqwest::header::HeaderValue::from_static("")),
                );
            }
        }

        // Base handler
        let base_handler: MiddlewareFn = Arc::new(move |ctx| {
            let client = client.clone();
            let req = ctx.request;
            let session = ctx.session.clone();
            Box::pin(async move {
                let url_str = req.url.to_string();
                let method = req.method;
                let mut req_builder = client.request(method, url_str);

                // Headers: request-level first, then session-level.
                // When `http2_header_order` is configured, order matching headers
                // first, then append remaining headers. reqwest/hyper manages
                // HTTP/2 pseudo-headers automatically.
                let mut ordered_headers: Vec<(reqwest::header::HeaderName, reqwest::header::HeaderValue)> = Vec::new();
                let mut normal_headers: Vec<(reqwest::header::HeaderName, reqwest::header::HeaderValue)> = Vec::new();

                let mut collect_headers = |map: &reqwest::header::HeaderMap| {
                    for (key, value) in map.iter() {
                        normal_headers.push((key.clone(), value.clone()));
                    }
                };

                if !req.headers.is_empty() {
                    collect_headers(&req.headers);
                } else {
                    collect_headers(&session.headers);
                }

                // Session-level auth fallback when request did not set explicit auth.
                if req.auth.is_none() {
                    if let Some((user, pass)) = &session.basic_auth {
                        let encoded = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", user, pass));
                        if let Ok(value) = reqwest::header::HeaderValue::from_str(&format!("Basic {}", encoded)) {
                            normal_headers.push((reqwest::header::AUTHORIZATION, value));
                        }
                    } else if let Some(token) = &session.bearer_token {
                        let header_value = format!("Bearer {}", token);
                        if let Ok(value) = reqwest::header::HeaderValue::from_str(&header_value) {
                            normal_headers.push((reqwest::header::AUTHORIZATION, value));
                        }
                    }
                } else if let Some((user, pass)) = &req.auth {
                    let encoded = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", user, pass));
                    if let Ok(value) = reqwest::header::HeaderValue::from_str(&format!("Basic {}", encoded)) {
                        normal_headers.push((reqwest::header::AUTHORIZATION, value));
                    }
                }

                if let Some(order) = &session.http2_header_order {
                    if !order.is_empty() {
                        let mut remaining = normal_headers;
                        for header_name in order {
                            if let Ok(name) = reqwest::header::HeaderName::from_bytes(header_name.as_bytes()) {
                                if let Some(pos) = remaining.iter().position(|(n, _)| n == &name) {
                                    ordered_headers.push(remaining.remove(pos));
                                }
                            }
                        }
                        ordered_headers.extend(remaining);
                    } else {
                        ordered_headers.extend(normal_headers);
                    }
                } else {
                    ordered_headers.extend(normal_headers);
                }
                for (key, value) in ordered_headers {
                    req_builder = req_builder.header(key, value);
                }

                // Body: prefer multipart when files are present, otherwise form/json/raw.
                if !req.files.is_empty() {
                    let mut form = reqwest::multipart::Form::new();
                    if let Some(form_data) = &req.data {
                        for (key, values) in &form_data.params {
                            for value in values {
                                form = form.text(key.clone(), value.clone());
                            }
                        }
                    }
                    for file in &req.files {
                        let file_bytes = match std::fs::read(&file.path) {
                            Ok(b) => b,
                            Err(_) => continue,
                        };
                        let part = reqwest::multipart::Part::bytes(file_bytes).file_name(file.filename.clone());
                        form = form.part(file.name.clone(), part);
                    }
                    req_builder = req_builder.multipart(form);
                } else if let Some(body) = req.body {
                    req_builder = req_builder.body(body);
                } else if let Some(json) = req.json {
                    req_builder = req_builder.json(&json);
                } else if let Some(form) = req.data {
                    let form_data: Vec<(String, String)> = form
                        .params
                        .iter()
                        .flat_map(|(key, values)| {
                            values.iter().map(move |value| (key.clone(), value.clone()))
                        })
                        .collect();
                    req_builder = req_builder.form(&form_data);
                }

                let response = req_builder.send().await?;

                let status = response.status();
                let headers = response.headers().clone();
                let url = response.url().clone();
                let content = if req.stream {
                    let mut buf = Vec::new();
                    let mut stream = std::pin::pin!(response.bytes_stream());
                    use futures_util::TryStreamExt;
                    while let Some(chunk) = stream.try_next().await? {
                        buf.extend_from_slice(&chunk);
                    }
                    buf
                } else {
                    response.bytes().await?.to_vec()
                };

                Ok(Response {
                    url,
                    headers,
                    cookies: Vec::new(),
                    content,
                    body: None,
                    status_code: status,
                    history: Vec::new(),
                    raw_response: Vec::new(),
                })
            })
        });

        // Build middleware chain
        let mut handler = base_handler;
        for middleware in session.middlewares.iter().rev() {
            let _next = handler.clone();
            let middleware = middleware.clone();
            handler = Arc::new(move |ctx| {
                let middleware = middleware.clone();
                Box::pin(async move { middleware(MiddlewareContext { session: ctx.session.clone(), request: ctx.request }).await })
            });
        }

        let mut last_err = None;
        let attempts = if self.retry == 0 { 1 } else { self.retry };
        for attempt in 0..attempts {
            if attempt > 0 {
                if self.retry_backoff {
                    let backoff_delay = std::time::Duration::from_secs_f64(
                        self.retry_delay.as_secs_f64() * (attempt as f64),
                    );
                    tokio::time::sleep(backoff_delay).await;
                } else {
                    tokio::time::sleep(self.retry_delay).await;
                }
            }

            // Rotate User-Agent per attempt if a pool is configured.
            let mut req_for_attempt = req.clone();
            if let Some(pool) = &self.user_agent_pool {
                if !pool.is_empty() {
                    let index = attempt % pool.len();
                    if let Ok(value) = reqwest::header::HeaderValue::from_str(&pool[index]) {
                        req_for_attempt.headers.insert(reqwest::header::USER_AGENT, value);
                    }
                }
            }

            match handler(MiddlewareContext { session: Arc::clone(&session), request: req_for_attempt }).await {
                Ok(resp) => {
                    let _ = self.save_response_cookies(&resp.headers);
                    return Ok(resp);
                }
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| crate::RequestsError::Http("request failed after retries".into())))
    }

    pub async fn request(&self, method: Method, url: impl AsRef<str>, request: Request) -> Result<Response> {
        let mut req = request;
        req.method = method;
        req.url = Url::parse(url.as_ref())?;
        self.send(req).await
    }

    pub async fn get(&self, url: impl AsRef<str>) -> Result<Response> {
        self.request(Method::GET, url, Request::default()).await
    }

    pub async fn post(&self, url: impl AsRef<str>) -> Result<Response> {
        self.request(Method::POST, url, Request::default()).await
    }
}
