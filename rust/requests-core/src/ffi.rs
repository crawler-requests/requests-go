use crate::{Request, Session};
use crate::curl_impersonate::BrowserProfile;
use std::ffi::{CString, CStr};
use std::os::raw::c_char;
use std::path::PathBuf;

#[repr(C)]
pub struct FFIResponse {
    pub json: *mut c_char,
    pub error: *mut c_char,
}

#[no_mangle]
pub extern "C" fn requests_version() -> *const c_char {
    b"requests-core 0.1.0\0".as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn requests_request(
    config_json: *const c_char,
) -> *mut FFIResponse {
    if config_json.is_null() {
        return box_ffi_result(
            Err(crate::RequestsError::InvalidUrl("null config".to_string())),
        );
    }

    let json_str = match unsafe { CStr::from_ptr(config_json) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            return box_ffi_result(Err(crate::RequestsError::Url(url::ParseError::EmptyHost)));
        }
    };

    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(_) => {
            return box_ffi_result(Err(crate::RequestsError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "failed to create tokio runtime",
            ))));
        }
    };

    let result = rt.block_on(async {
        let input: crate::FfiRequestInput = serde_json::from_str(json_str)
            .map_err(crate::RequestsError::Json)?;

        let mut session = Session::new();

        if let Some(timeout) = input.timeout {
            session = session.with_timeout(std::time::Duration::from_secs(timeout));
        }

        if let Some(verify) = input.verify {
            session = session.with_verify(verify);
        }

        if let Some(proxy) = input.proxy {
            session = session.with_proxy(proxy);
        }

        if let Some(ja3) = input.ja3 {
            session = session.with_ja3(ja3);
        }

        if input.random_ja3 {
            session = session.with_random_ja3(true);
        }

        if let Some(ref h2) = input.http2_settings {
            session = session.with_http2_settings(h2.clone());
        }

        if let Some(target) = input.impersonate {
            session = session.with_impersonate(target);
        }
        if let Some(akamai) = input.akamai {
            session = session.with_akamai(akamai);
        }
        if let Some(enabled) = input.impersonate_default_headers {
            session = session.with_impersonate_default_headers(enabled);
        }

        if let Some((profile_name, binary_path)) = input.browser_profile {
            let profile = match profile_name.as_str() {
                "Chrome110" | "chrome110" => BrowserProfile::Chrome110,
                "Chrome120" | "chrome120" => BrowserProfile::Chrome120,
                "Firefox" | "firefox" | "Firefox115" | "firefox115" => BrowserProfile::Firefox,
                "Safari" | "safari" | "Safari15" | "safari15" => BrowserProfile::Safari,
                "Edge" | "edge" | "Edge110" | "edge110" => BrowserProfile::Edge,
                _ => BrowserProfile::Chrome110,
            };
            session = session.with_browser_profile(profile, PathBuf::from(binary_path));
        }

        if input.http1 {
            session = session.with_force_http1(true);
        }

        if input.allow_redirects == Some(false) {
            session = session.with_max_redirects(0);
        }

        if let Some(cert) = input.cert {
            session.cert = cert;
        }

        if let Some(max_idle) = input.pool_max_idle_per_host {
            session = session.with_pool_max_idle_per_host(max_idle);
        }
        if let Some(idle_timeout) = input.pool_idle_timeout {
            session = session.with_pool_idle_timeout(std::time::Duration::from_secs(idle_timeout));
        }
        if let Some(keepalive) = input.tcp_keepalive {
            session = session.with_tcp_keepalive(std::time::Duration::from_secs(keepalive));
        }
        if let Some(path) = input.cookie_store_path {
            session = session.with_cookie_store_path(std::path::PathBuf::from(path));
        }

        if let Some(ua) = input.user_agent {
            session = session.with_user_agent(ua);
        }
        if let Some(accept) = input.accept {
            session = session.with_accept(accept);
        }
        if let Some(encoding) = input.accept_encoding {
            session = session.with_accept_encoding(encoding);
        }
        if let Some(language) = input.accept_language {
            session = session.with_accept_language(language);
        }
        if let Some(ct) = input.content_type {
            session = session.with_content_type(ct);
        }
        if let Some(referer) = input.referer {
            session = session.with_referer(referer);
        }
        if let Some(auth) = input.basic_auth {
            if auth.len() >= 2 {
                session = session.with_basic_auth(&auth[0], &auth[1]);
            }
        }
        if let Some(token) = input.bearer_token {
            session = session.with_bearer_token(token);
        }
        if let Some(size) = input.max_response_size {
            session = session.with_max_response_size(size);
        }
        if let Some(disable) = input.disable_cookie_jar {
            session = session.with_disable_cookie_jar(disable);
        }
        if let Some(addr) = input.local_address {
            session = session.with_local_address(addr);
        }
        if let Some(iface) = input.interface {
            session = session.with_interface(iface);
        }
        if let Some(nodelay) = input.tcp_nodelay {
            session = session.with_tcp_nodelay(nodelay);
        }
        if let Some(timeout) = input.connect_timeout {
            session = session.with_connect_timeout(std::time::Duration::from_secs(timeout));
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

        // 新增配置：重试
        if let Some(retry) = input.retry {
            session = session.with_retry(retry);
        }
        if let Some(delay) = input.retry_delay {
            session = session.with_retry_delay(std::time::Duration::from_secs(delay));
        }

        // 新增配置：HTTP/2 窗口
        if let Some(size) = input.http2_window_size {
            session = session.with_http2_window_size(size);
        }
        if let Some(size) = input.http2_connection_window_size {
            session = session.with_http2_connection_window_size(size);
        }
        if let Some(size) = input.http2_max_frame_size {
            session = session.with_http2_max_frame_size(size);
        }

        // 新增配置：TCP keepalive 间隔
        if let Some(interval) = input.tcp_keepalive_interval {
            session = session.with_tcp_keepalive_interval(std::time::Duration::from_secs(interval));
        }

        // Build request
        let method = match input.method.as_deref() {
            Some("POST") => reqwest::Method::POST,
            Some("PUT") => reqwest::Method::PUT,
            Some("PATCH") => reqwest::Method::PATCH,
            Some("DELETE") => reqwest::Method::DELETE,
            Some("HEAD") => reqwest::Method::HEAD,
            Some("OPTIONS") => reqwest::Method::OPTIONS,
            _ => reqwest::Method::GET,
        };

        let mut req = Request::new(method.clone(), &input.url)?;

        if let Some(params) = input.params {
            let mut p = crate::RequestParams::new();
            for (k, v) in params {
                p.add(k, v);
            }
            req.params = Some(p);
        }

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

        if let Some(cookies) = input.cookies {
            for (k, v) in cookies {
                req.headers.insert(
                    reqwest::header::COOKIE,
                    reqwest::header::HeaderValue::from_str(&format!("{}={}", k, v)).unwrap(),
                );
            }
        }

        if let Some(data) = input.data {
            let mut d = crate::RequestParams::new();
            for (k, v) in data {
                d.add(k, v);
            }
            req.data = Some(d);
        }

        if let Some(files) = input.files {
            for f in files {
                req.files.push(crate::FileUpload {
                    name: f.name,
                    filename: f.filename,
                    content_type: f.content_type,
                    path: std::path::PathBuf::from(f.path),
                });
            }
        }

        if let Some(json_body) = input.json {
            req.json = Some(json_body);
        }

        if let Some(body) = input.body {
            req.body = Some(body.into_bytes());
        }

        if let Some(auth) = input.auth {
            req.auth = Some((auth[0].clone(), auth[1].clone()));
        }

        if input.stream {
            req.stream = true;
        }

        let resp = session.request(method, input.url, req).await?;

        let headers = resp
            .headers
            .iter()
            .map(|(k, v)| {
                let vals = vec![String::from_utf8_lossy(v.as_bytes()).to_string()];
                (k.clone().to_string(), vals)
            })
            .collect();

        let output = crate::FfiResponseOutput {
            url: resp.url.to_string(),
            status_code: resp.status_code.as_u16(),
            headers,
            cookies: resp.cookies,
            content: resp.content,
        };

        Ok::<crate::FfiResponseOutput, crate::RequestsError>(output)
    });

    box_ffi_result(result)
}

#[no_mangle]
pub extern "C" fn requests_free_response(resp: *mut FFIResponse) {
    if resp.is_null() {
        return;
    }
    unsafe {
        if !(*resp).json.is_null() {
            let _ = CString::from_raw((*resp).json);
        }
        if !(*resp).error.is_null() {
            let _ = CString::from_raw((*resp).error);
        }
        let _ = Box::from_raw(resp);
    }
}

fn box_ffi_result(result: crate::Result<crate::FfiResponseOutput>) -> *mut FFIResponse {
    match result {
        Ok(output) => {
            let json = match serde_json::to_string(&output) {
                Ok(s) => s,
                Err(_) => {
                    let error_c = CString::new("failed to serialize response").unwrap();
                    return Box::into_raw(Box::new(FFIResponse {
                        json: std::ptr::null_mut(),
                        error: error_c.into_raw(),
                    }));
                }
            };

            let json_c = CString::new(json).unwrap();
            Box::into_raw(Box::new(FFIResponse {
                json: json_c.into_raw(),
                error: std::ptr::null_mut(),
            }))
        }
        Err(e) => {
            let error_c = CString::new(e.to_string()).unwrap();
            Box::into_raw(Box::new(FFIResponse {
                json: std::ptr::null_mut(),
                error: error_c.into_raw(),
            }))
        }
    }
}

#[no_mangle]
pub extern "C" fn requests_send_get(url: *const c_char) -> *mut FFIResponse {
    if url.is_null() {
        return box_ffi_result(Err(crate::RequestsError::InvalidUrl("null url".to_string())));
    }
    let url_str = match unsafe { CStr::from_ptr(url) }.to_str() {
        Ok(s) => s,
        Err(_) => return box_ffi_result(Err(crate::RequestsError::Url(url::ParseError::EmptyHost))),
    };

    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(_) => {
            return box_ffi_result(Err(crate::RequestsError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "failed to create tokio runtime",
            ))));
        }
    };

    let result = rt.block_on(async {
        let session = Session::new();
        let resp = session.get(url_str).await?;
        Ok::<crate::FfiResponseOutput, crate::RequestsError>(resp.into())
    });
    box_ffi_result(result)
}

#[no_mangle]
pub extern "C" fn requests_send_post(url: *const c_char, body: *const c_char) -> *mut FFIResponse {
    if url.is_null() {
        return box_ffi_result(Err(crate::RequestsError::InvalidUrl("null url".to_string())));
    }
    let url_str = match unsafe { CStr::from_ptr(url) }.to_str() {
        Ok(s) => s,
        Err(_) => return box_ffi_result(Err(crate::RequestsError::Url(url::ParseError::EmptyHost))),
    };
    let body_bytes = if body.is_null() {
        None
    } else {
        match unsafe { CStr::from_ptr(body) }.to_str() {
            Ok(s) => Some(s.as_bytes().to_vec()),
            Err(_) => None,
        }
    };

    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(_) => {
            return box_ffi_result(Err(crate::RequestsError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "failed to create tokio runtime",
            ))));
        }
    };

    let result = rt.block_on(async {
        let session = Session::new();
        let mut req = Request::new(reqwest::Method::POST, url_str)?;
        if let Some(bytes) = body_bytes {
            req.body = Some(bytes);
        }
        let resp = session.send(req).await?;
        Ok::<crate::FfiResponseOutput, crate::RequestsError>(resp.into())
    });
    box_ffi_result(result)
}

// ============================================================================
// 工具函数 FFI 包装
// ============================================================================

#[no_mangle]
pub extern "C" fn requests_encode_uri_component(input: *const c_char) -> *mut c_char {
    if input.is_null() {
        return std::ptr::null_mut();
    }
    let input_str = match unsafe { CStr::from_ptr(input) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let encoded = crate::encode_uri_component(input_str);
    match CString::new(encoded) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn requests_md5(data: *const u8, len: usize) -> *mut c_char {
    if data.is_null() {
        return std::ptr::null_mut();
    }
    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    let hash = crate::md5(slice);
    match CString::new(hash) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn requests_sha256(data: *const u8, len: usize) -> *mut c_char {
    if data.is_null() {
        return std::ptr::null_mut();
    }
    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    let hash = crate::sha256(slice);
    match CString::new(hash) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn requests_hmac_sha256(
    key: *const u8,
    key_len: usize,
    msg: *const u8,
    msg_len: usize,
) -> *mut c_char {
    if key.is_null() || msg.is_null() {
        return std::ptr::null_mut();
    }
    let key_slice = unsafe { std::slice::from_raw_parts(key, key_len) };
    let msg_slice = unsafe { std::slice::from_raw_parts(msg, msg_len) };
    let result = crate::hmac_sha256(key_slice, msg_slice);
    // hmac_sha256 返回 Vec<u8>，需要转为十六进制字符串
    let hex: String = result.iter().map(|b| format!("{:02x}", b)).collect();
    match CString::new(hex) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[repr(C)]
pub struct FfiBytes {
    pub data: *mut u8,
    pub len: usize,
}

#[no_mangle]
pub extern "C" fn requests_rc4(
    key: *const u8,
    key_len: usize,
    input: *const u8,
    input_len: usize,
) -> FfiBytes {
    if key.is_null() || input.is_null() {
        return FfiBytes {
            data: std::ptr::null_mut(),
            len: 0,
        };
    }
    let key_slice = unsafe { std::slice::from_raw_parts(key, key_len) };
    let input_slice = unsafe { std::slice::from_raw_parts(input, input_len) };
    let output = crate::rc4(key_slice, input_slice);
    let len = output.len();
    let mut boxed = output.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    std::mem::forget(boxed);
    FfiBytes { data: ptr, len }
}

#[no_mangle]
pub extern "C" fn requests_free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(s);
    }
}

#[no_mangle]
pub extern "C" fn requests_free_bytes(p: *mut u8, len: usize) {
    if p.is_null() || len == 0 {
        return;
    }
    unsafe {
        let _ = Box::from_raw(std::slice::from_raw_parts_mut(p, len));
    }
}

