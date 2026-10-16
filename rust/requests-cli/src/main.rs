use clap::{Parser, Subcommand};
use requests_core::{Session, Request};
use std::collections::HashMap;
use std::time::Duration;
use base64::{engine::general_purpose::STANDARD, Engine};

#[derive(Parser)]
#[command(name = "requests")]
#[command(about = "HTTP client with TLS fingerprint customization", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, global = true)]
    session: Option<String>,

    #[arg(long, global = true)]
    proxy: Option<String>,

    #[arg(long, global = true, default_value = "true")]
    verify: bool,

    #[arg(long, global = true, default_value = "30")]
    timeout: u64,

    #[arg(long, global = true)]
    ja3: Option<String>,

    #[arg(long, global = true, default_value_t = false)]
    random_ja3: bool,

    #[arg(long, global = true, default_value_t = false)]
    http1: bool,

    #[arg(long, global = true, default_value_t = false)]
    debug: bool,

    #[arg(long, global = true)]
    browser_profile: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    Request {
        #[arg(long)]
        json: Option<String>,
    },
    Stream {
        #[arg(long)]
        json: Option<String>,

        #[arg(long, default_value = "4096")]
        chunk_size: usize,
    },
    Version,
}

#[derive(serde::Serialize)]
struct ResponseOutput {
    url: String,
    status_code: u16,
    headers: HashMap<String, Vec<String>>,
    cookies: Vec<CookieOutput>,
    content: String,
}

#[derive(serde::Serialize)]
struct CookieOutput {
    name: String,
    value: String,
    path: String,
    domain: String,
    expires: String,
    http_only: bool,
    secure: bool,
}

fn main() {
    let cli = Cli::parse();

    if let Commands::Version = cli.command {
        println!("requests 0.1.0");
        return;
    }

    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(async {
        let mut session = Session::new()
            .with_timeout(Duration::from_secs(cli.timeout))
            .with_verify(cli.verify);

        if let Some(proxy) = cli.proxy {
            session = session.with_proxy(proxy);
        }

        if let Some(ja3) = cli.ja3 {
            session = session.with_ja3(ja3);
        }

        if cli.random_ja3 {
            session = session.with_random_ja3(true);
        }

        if cli.http1 {
            session = session.with_force_http1(true);
        }

        if let Some(profile_str) = cli.browser_profile {
            let profile = match profile_str.as_str() {
                "Chrome110" | "chrome110" => requests_core::BrowserProfile::Chrome110,
                "Chrome120" | "chrome120" => requests_core::BrowserProfile::Chrome120,
                "Firefox" | "firefox" | "Firefox115" | "firefox115" => requests_core::BrowserProfile::Firefox,
                "Safari" | "safari" | "Safari15" | "safari15" => requests_core::BrowserProfile::Safari,
                "Edge" | "edge" | "Edge110" | "edge110" => requests_core::BrowserProfile::Edge,
                _ => requests_core::BrowserProfile::Chrome110,
            };

            let binary = std::env::var("CURL_IMPERSONATE_BINARY")
                .unwrap_or_else(|_| "curl-impersonate".to_string());

            session = session.with_browser_profile(profile, binary);
        }

        let result = match cli.command {
            Commands::Request { json } => handle_request(session, json).await,
            Commands::Stream { json, chunk_size } => handle_stream(session, json, chunk_size).await,
            Commands::Version => unreachable!(),
        };

        if let Err(e) = result {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    });
}

fn apply_session_options(session: Session, input: &RequestInput) -> Session {
    let mut session = session;

    // TLS / browser fingerprint options (JSON contract, mirrors FfiRequestInput)
    if let Some(verify) = input.verify {
        session = session.with_verify(verify);
    }
    if let Some(proxy) = &input.proxy {
        session = session.with_proxy(proxy.clone());
    }
    if let Some(timeout) = input.timeout {
        session = session.with_timeout(Duration::from_secs(timeout));
    }
    if let Some(ja3) = &input.ja3 {
        session = session.with_ja3(ja3.clone());
    }
    if input.random_ja3.unwrap_or(false) {
        session = session.with_random_ja3(true);
    }
    if input.http1.unwrap_or(false) {
        session = session.with_force_http1(true);
    }
    if let Some(target) = &input.impersonate {
        session = session.with_impersonate(target.clone());
    }
    if let Some(akamai) = &input.akamai {
        session = session.with_akamai(akamai.clone());
    }
    if let Some(enabled) = input.impersonate_default_headers {
        session = session.with_impersonate_default_headers(enabled);
    }
    if let Some(retry) = input.retry {
        session = session.with_retry(retry);
    }
    if let Some(delay) = input.retry_delay {
        session = session.with_retry_delay(Duration::from_secs(delay));
    }

    if let Some(ref h2) = input.http2_settings {
        session = session.with_http2_settings(h2.clone());
    }
    if let Some(allow) = input.allow_redirects {
        session = session.with_allow_redirects(allow);
    }
    if let Some(ref cert) = input.cert {
        session = session.with_cert(cert.clone());
    }
    if let Some(max_idle) = input.pool_max_idle_per_host {
        session = session.with_pool_max_idle_per_host(max_idle);
    }
    if let Some(idle_timeout) = input.pool_idle_timeout {
        session = session.with_pool_idle_timeout(Duration::from_secs(idle_timeout));
    }
    if let Some(keepalive) = input.tcp_keepalive {
        session = session.with_tcp_keepalive(Duration::from_secs(keepalive));
    }
    if let Some(ref path) = input.cookie_store_path {
        session = session.with_cookie_store_path(path.clone());
    }
    if let Some(ref ua) = input.user_agent {
        session = session.with_user_agent(ua.clone());
    }
    if let Some(ref accept) = input.accept {
        session = session.with_accept(accept.clone());
    }
    if let Some(ref encoding) = input.accept_encoding {
        session = session.with_accept_encoding(encoding.clone());
    }
    if let Some(ref language) = input.accept_language {
        session = session.with_accept_language(language.clone());
    }
    if let Some(ref ct) = input.content_type {
        session = session.with_content_type(ct.clone());
    }
    if let Some(ref referer) = input.referer {
        session = session.with_referer(referer.clone());
    }
    if let Some(ref auth) = input.basic_auth {
        if auth.len() >= 2 {
            session = session.with_basic_auth(&auth[0], &auth[1]);
        }
    }
    if let Some(ref token) = input.bearer_token {
        session = session.with_bearer_token(token.clone());
    }
    if let Some(size) = input.max_response_size {
        session = session.with_max_response_size(size);
    }
    if let Some(disable) = input.disable_cookie_jar {
        session = session.with_disable_cookie_jar(disable);
    }
    if let Some(ref addr) = input.local_address {
        session = session.with_local_address(addr.clone());
    }
    if let Some(ref iface) = input.interface {
        session = session.with_interface(iface.clone());
    }
    if let Some(nodelay) = input.tcp_nodelay {
        session = session.with_tcp_nodelay(nodelay);
    }
    if let Some(timeout) = input.connect_timeout {
        session = session.with_connect_timeout(Duration::from_secs(timeout));
    }
    if input.retry_backoff.unwrap_or(false) {
        session = session.with_retry_backoff(true);
    }
    if let Some(ref pool) = input.user_agent_pool {
        session = session.with_user_agent_pool(pool.clone());
    }
    if input.dns_cache.unwrap_or(false) {
        session = session.with_dns_cache(true);
    }
    if let Some(ref servers) = input.dns_servers {
        session = session.with_dns_servers(servers.clone());
    }
    if let Some(ref order) = input.http2_header_order {
        session = session.with_http2_header_order(order.clone());
    }
    if let Some(reuse) = input.connection_reuse {
        session = session.with_connection_reuse(reuse);
    }
    if let Some(ref min_ver) = input.tls_min_version {
        session = session.with_tls_min_version(min_ver.clone());
    }
    if let Some(ref max_ver) = input.tls_max_version {
        session = session.with_tls_max_version(max_ver.clone());
    }
    if let Some(ref alpn) = input.tls_alpn {
        session = session.with_tls_alpn(alpn.clone());
    }
    session
}

async fn handle_request(mut session: Session, json: Option<String>) -> anyhow::Result<()> {
    let input = read_json_input(json)?;
    session = apply_session_options(session, &input);

    let url = input.url;

    let method = input.method.unwrap_or_else(|| "GET".to_string());
    let method = reqwest::Method::from_bytes(method.as_bytes())?;
    let req_method = method.clone();
    let mut req = Request::new(req_method, &url)?;

    // Params
    if let Some(params) = input.params {
        let mut p = requests_core::RequestParams::new();
        for (k, v) in params {
            p.add(k, v);
        }
        req.params = Some(p);
    }

    // Headers
    if let Some(headers) = input.headers {
        let mut header_map = reqwest::header::HeaderMap::new();
        for (k, v) in headers {
            if k.to_lowercase() != "content-length" {
                if let (Ok(name), Ok(value)) = (
                    reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                    reqwest::header::HeaderValue::from_str(&v),
                ) {
                    header_map.insert(name, value);
                }
            }
        }
        req.headers = header_map;
    }

    // Data (form)
    if let Some(data) = input.data {
        let mut d = requests_core::RequestParams::new();
        for (k, v) in data {
            d.add(k, v);
        }
        req.data = Some(d);
    }

    // Files (multipart)
    if let Some(files) = input.files {
        for f in files {
            req.files.push(requests_core::FileUpload {
                name: f.name,
                filename: f.filename,
                content_type: f.content_type,
                path: std::path::PathBuf::from(f.path),
            });
        }
    }

    // JSON body
    if let Some(json_body) = input.json {
        req.json = Some(json_body);
    }

    // Raw body (Base64)
    if let Some(body) = input.body {
        use base64::{engine::general_purpose, Engine};
        req.body = Some(general_purpose::STANDARD.decode(body)?);
    }

    // Auth
    if let Some(auth) = input.auth {
        req.auth = Some((auth[0].clone(), auth[1].clone()));
    }

    // Cookies
    if let Some(cookies) = input.cookies {
        for (k, v) in cookies {
            // Simple cookie setting via header for now
            req.headers.insert(
                reqwest::header::COOKIE,
                reqwest::header::HeaderValue::from_str(&format!("{}={}", k, v))?,
            );
        }
    }

    // Stream flag
    if input.stream {
        req.stream = true;
    }

    session.ensure_cookies_loaded();
    let resp = session.request(method, url, req).await?;

    let _ = session.save_response_cookies(&resp.headers);

    let headers = resp
        .headers
        .iter()
        .map(|(k, v)| {
            let vals = vec![String::from_utf8_lossy(v.as_bytes()).to_string()];
            (k.clone().to_string(), vals)
        })
        .collect();

    let output = ResponseOutput {
        url: resp.url.to_string(),
        status_code: resp.status_code.as_u16(),
        headers,
        cookies: Vec::new(),
        content: STANDARD.encode(resp.content),
    };

    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}

async fn handle_stream(mut session: Session, json: Option<String>, chunk_size: usize) -> anyhow::Result<()> {
    let input = read_json_input(json)?;
    session = apply_session_options(session, &input);

    let url = input.url;

    let method = input.method.unwrap_or_else(|| "GET".to_string());
    let method = reqwest::Method::from_bytes(method.as_bytes())?;
    let req_method = method.clone();
    let mut req = Request::new(req_method, &url)?;
    req.stream = true;

    let resp = session.request(method, url, req).await?;

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    use std::io::Write;
    for chunk in resp.content.chunks(chunk_size) {
        let encoded = STANDARD.encode(chunk);
        writeln!(stdout, "{{\"chunk\":\"{}\",\"eof\":false}}", encoded)?;
    }
    writeln!(stdout, "{{\"eof\":true}}")?;
    Ok(())
}

#[derive(serde::Deserialize)]
struct RequestInput {
    method: Option<String>,
    url: String,
    params: Option<HashMap<String, String>>,
    headers: Option<HashMap<String, String>>,
    data: Option<HashMap<String, String>>,
    files: Option<Vec<FileUploadInput>>,
    body: Option<String>,
    json: Option<serde_json::Value>,
    auth: Option<Vec<String>>,
    cookies: Option<HashMap<String, String>>,
    stream: bool,
    http2_settings: Option<String>,
    allow_redirects: Option<bool>,
    cert: Option<Vec<String>>,
    pool_max_idle_per_host: Option<usize>,
    pool_idle_timeout: Option<u64>,
    tcp_keepalive: Option<u64>,
    cookie_store_path: Option<String>,
    user_agent: Option<String>,
    accept: Option<String>,
    accept_encoding: Option<String>,
    accept_language: Option<String>,
    content_type: Option<String>,
    referer: Option<String>,
    basic_auth: Option<Vec<String>>,
    bearer_token: Option<String>,
    max_response_size: Option<usize>,
    disable_cookie_jar: Option<bool>,
    local_address: Option<String>,
    interface: Option<String>,
    tcp_nodelay: Option<bool>,
    connect_timeout: Option<u64>,
    retry_backoff: Option<bool>,
    user_agent_pool: Option<Vec<String>>,
    dns_cache: Option<bool>,
    dns_servers: Option<Vec<String>>,
    http2_header_order: Option<Vec<String>>,
    connection_reuse: Option<bool>,
    tls_min_version: Option<String>,
    tls_max_version: Option<String>,
    tls_alpn: Option<Vec<String>>,
    // -- TLS / browser fingerprint (JSON contract, mirrors FfiRequestInput) --
    verify: Option<bool>,
    proxy: Option<String>,
    timeout: Option<u64>,
    ja3: Option<String>,
    random_ja3: Option<bool>,
    http1: Option<bool>,
    impersonate: Option<String>,
    akamai: Option<String>,
    impersonate_default_headers: Option<bool>,
    retry: Option<usize>,
    retry_delay: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
struct FileUploadInput {
    name: String,
    filename: String,
    content_type: String,
    path: String,
}

fn read_json_input(json: Option<String>) -> anyhow::Result<RequestInput> {
    let input = if let Some(json) = json {
        json
    } else {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        if buf.trim().is_empty() {
            anyhow::bail!("no input provided");
        }
        buf
    };

    Ok(serde_json::from_str(&input)?)
}
