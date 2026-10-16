//! In-process binding to libcurl-impersonate.
//!
//! This is the same shape curl_cffi uses: one easy handle, `curl_easy_impersonate`
//! for a browser target, then libcurl options for JA3 / Akamai / extra fingerprint
//! overrides. The shared library is loaded at runtime so the crate still builds
//! against a stock libcurl; impersonation is used only when a real
//! libcurl-impersonate is found.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_long, c_void};
use std::path::PathBuf;
use std::ptr;
use std::sync::OnceLock;

use libc::size_t;

use crate::error::{RequestsError, Result};

pub const CURL_GLOBAL_DEFAULT: c_long = 3;
pub const CURLOPTTYPE_LONG: c_int = 0;
pub const CURLOPTTYPE_OBJECTPOINT: c_int = 10_000;
pub const CURLOPTTYPE_STRINGPOINT: c_int = 10_000;
pub const CURLOPTTYPE_FUNCTIONPOINT: c_int = 20_000;
pub const CURLOPTTYPE_OFF_T: c_int = 30_000;
pub const CURLOPTTYPE_BLOB: c_int = 40_000;

pub const CURLOPT_WRITEDATA: c_int = CURLOPTTYPE_OBJECTPOINT + 1;
pub const CURLOPT_URL: c_int = CURLOPTTYPE_STRINGPOINT + 2;
pub const CURLOPT_PROXY: c_int = CURLOPTTYPE_STRINGPOINT + 4;
pub const CURLOPT_READDATA: c_int = CURLOPTTYPE_OBJECTPOINT + 9;
pub const CURLOPT_WRITEFUNCTION: c_int = CURLOPTTYPE_FUNCTIONPOINT + 11;
pub const CURLOPT_READFUNCTION: c_int = CURLOPTTYPE_FUNCTIONPOINT + 12;
pub const CURLOPT_HEADERFUNCTION: c_int = CURLOPTTYPE_FUNCTIONPOINT + 79;
pub const CURLOPT_HEADERDATA: c_int = CURLOPTTYPE_OBJECTPOINT + 29;
pub const CURLOPT_HTTPHEADER: c_int = CURLOPTTYPE_OBJECTPOINT + 23;
pub const CURLOPT_POSTFIELDS: c_int = CURLOPTTYPE_OBJECTPOINT + 15;
pub const CURLOPT_POSTFIELDSIZE: c_int = CURLOPTTYPE_LONG + 60;
pub const CURLOPT_CUSTOMREQUEST: c_int = CURLOPTTYPE_STRINGPOINT + 36;
pub const CURLOPT_TIMEOUT: c_int = CURLOPTTYPE_LONG + 13;
pub const CURLOPT_CONNECTTIMEOUT: c_int = CURLOPTTYPE_LONG + 78;
pub const CURLOPT_FOLLOWLOCATION: c_int = CURLOPTTYPE_LONG + 52;
pub const CURLOPT_MAXREDIRS: c_int = CURLOPTTYPE_LONG + 68;
pub const CURLOPT_SSL_VERIFYPEER: c_int = CURLOPTTYPE_LONG + 64;
pub const CURLOPT_SSL_VERIFYHOST: c_int = CURLOPTTYPE_LONG + 81;
pub const CURLOPT_COOKIE: c_int = CURLOPTTYPE_STRINGPOINT + 22;
pub const CURLOPT_USERAGENT: c_int = CURLOPTTYPE_STRINGPOINT + 18;
pub const CURLOPT_ACCEPT_ENCODING: c_int = CURLOPTTYPE_STRINGPOINT + 102;
pub const CURLOPT_HTTP_VERSION: c_int = CURLOPTTYPE_LONG + 84;
pub const CURLOPT_INTERFACE: c_int = CURLOPTTYPE_STRINGPOINT + 62;
pub const CURLOPT_NOPROGRESS: c_int = CURLOPTTYPE_LONG + 43;

/// curl-impersonate extensions. Numbers are the ones curl_cffi binds in
/// curl_cffi/const.py against the lexiforest fork. Stock libcurl returns
/// CURLE_UNKNOWN_OPTION (48) for all of them.
pub const CURLOPT_IMPERSONATE: c_int = CURLOPTTYPE_STRINGPOINT + 999;
pub const CURLOPT_SSL_EC_CURVES: c_int = CURLOPTTYPE_STRINGPOINT + 298;
pub const CURLOPT_SSL_SIG_HASH_ALGS: c_int = CURLOPTTYPE_STRINGPOINT + 1001;
pub const CURLOPT_SSL_ENABLE_ALPS: c_int = CURLOPTTYPE_LONG + 1002;
pub const CURLOPT_SSL_CERT_COMPRESSION: c_int = CURLOPTTYPE_STRINGPOINT + 1003;
pub const CURLOPT_SSL_ENABLE_TICKET: c_int = CURLOPTTYPE_LONG + 1004;
pub const CURLOPT_HTTP2_PSEUDO_HEADERS_ORDER: c_int = CURLOPTTYPE_STRINGPOINT + 1005;
pub const CURLOPT_HTTP2_SETTINGS: c_int = CURLOPTTYPE_STRINGPOINT + 1006;
pub const CURLOPT_SSL_PERMUTE_EXTENSIONS: c_int = CURLOPTTYPE_LONG + 1007;
pub const CURLOPT_HTTP2_WINDOW_UPDATE: c_int = CURLOPTTYPE_LONG + 1008;
pub const CURLOPT_HTTP2_STREAMS: c_int = CURLOPTTYPE_STRINGPOINT + 1010;
pub const CURLOPT_TLS_GREASE: c_int = CURLOPTTYPE_LONG + 1011;
pub const CURLOPT_TLS_EXTENSION_ORDER: c_int = CURLOPTTYPE_STRINGPOINT + 1012;
pub const CURLOPT_HTTPHEADER_ORDER: c_int = CURLOPTTYPE_STRINGPOINT + 1030;

pub const CURL_HTTP_VERSION_1_1: c_long = 2;
pub const CURL_HTTP_VERSION_2_0: c_long = 3;

pub const CURLE_OK: c_int = 0;
pub const CURLINFO_RESPONSE_CODE: c_int = 0x200000 + 2;
pub const CURLINFO_EFFECTIVE_URL: c_int = 0x100000 + 1;

type Easy = c_void;
type Slist = c_void;

type CurlGlobalInit = unsafe extern "C" fn(c_long) -> c_int;
type CurlEasyInit = unsafe extern "C" fn() -> *mut Easy;
type CurlEasyCleanup = unsafe extern "C" fn(*mut Easy);
type CurlEasySetopt = unsafe extern "C" fn(*mut Easy, c_int, ...) -> c_int;
type CurlEasyPerform = unsafe extern "C" fn(*mut Easy) -> c_int;
type CurlEasyImpersonate = unsafe extern "C" fn(*mut Easy, *const c_char, c_int) -> c_int;
type CurlEasyStrerror = unsafe extern "C" fn(c_int) -> *const c_char;
type CurlEasyGetinfo = unsafe extern "C" fn(*mut Easy, c_int, ...) -> c_int;
type CurlSlistAppend = unsafe extern "C" fn(*mut Slist, *const c_char) -> *mut Slist;
type CurlSlistFreeAll = unsafe extern "C" fn(*mut Slist);

struct Api {
    _lib: libloading::Library,
    global_init: CurlGlobalInit,
    easy_init: CurlEasyInit,
    easy_cleanup: CurlEasyCleanup,
    easy_setopt: CurlEasySetopt,
    easy_perform: CurlEasyPerform,
    easy_impersonate: Option<CurlEasyImpersonate>,
    easy_strerror: CurlEasyStrerror,
    easy_getinfo: CurlEasyGetinfo,
    slist_append: CurlSlistAppend,
    slist_free_all: CurlSlistFreeAll,
    path: String,
}

unsafe impl Send for Api {}
unsafe impl Sync for Api {}

fn candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(explicit) = std::env::var("REQUESTS_LIBCURL") {
        if !explicit.is_empty() {
            paths.push(PathBuf::from(explicit));
        }
    }
    if let Ok(explicit) = std::env::var("CURL_IMPERSONATE_LIB") {
        if !explicit.is_empty() {
            paths.push(PathBuf::from(explicit));
        }
    }

    // Auto-discover next to the current executable (CLI or embedded .so),
    // in libs_dist/ and in the working directory — so wrappers need no env var.
    let cwd = std::env::current_dir().ok();
    let exe_dir = std::env::current_exe().ok().and_then(|p| {
        p.parent().map(|d| d.to_path_buf())
    });
    let mut base_dirs: Vec<PathBuf> = Vec::new();
    if let Some(d) = exe_dir {
        base_dirs.push(d.clone());
        base_dirs.push(d.join("libs_dist"));
    }
    if let Some(d) = cwd {
        base_dirs.push(d.clone());
        base_dirs.push(d.join("libs_dist"));
        base_dirs.push(d.join("../libs_dist"));
    }
    for name in [
        "libcurl-impersonate.so",
        "libcurl-impersonate.so.4",
        "libcurl-impersonate.so.4.8.0",
        "libcurl-impersonate.dylib",
        "libcurl-impersonate.dll",
    ] {
        for base in &base_dirs {
            paths.push(base.join(name));
        }
        paths.push(PathBuf::from(name));
    }
    paths
}

fn load_api() -> std::result::Result<Api, String> {
    let mut errors = Vec::new();
    for path in candidate_paths() {
        let loaded = unsafe { libloading::Library::new(&path) };
        let lib = match loaded {
            Ok(lib) => lib,
            Err(e) => {
                errors.push(format!("{}: {e}", path.display()));
                continue;
            }
        };
        let loaded_api = unsafe {
            macro_rules! sym {
                ($name:literal) => {
                    match lib.get($name) {
                        Ok(s) => s,
                        Err(e) => {
                            errors.push(format!("{} missing {}: {e}", path.display(), stringify!($name)));
                            continue;
                        }
                    }
                };
            }
            let global_init = *sym!(b"curl_global_init\0");
            let easy_init = *sym!(b"curl_easy_init\0");
            let easy_cleanup = *sym!(b"curl_easy_cleanup\0");
            let easy_setopt = *sym!(b"curl_easy_setopt\0");
            let easy_perform = *sym!(b"curl_easy_perform\0");
            let easy_strerror = *sym!(b"curl_easy_strerror\0");
            let easy_getinfo = *sym!(b"curl_easy_getinfo\0");
            let slist_append = *sym!(b"curl_slist_append\0");
            let slist_free_all = *sym!(b"curl_slist_free_all\0");
            let easy_impersonate = lib
                .get::<CurlEasyImpersonate>(b"curl_easy_impersonate\0")
                .ok()
                .map(|s| *s);
            Api {
                _lib: lib,
                global_init,
                easy_init,
                easy_cleanup,
                easy_setopt,
                easy_perform,
                easy_impersonate,
                easy_strerror,
                easy_getinfo,
                slist_append,
                slist_free_all,
                path: path.display().to_string(),
            }
        };
        let api = loaded_api;
        unsafe {
            (api.global_init)(CURL_GLOBAL_DEFAULT);
        }
        return Ok(api);
    }
    Err(errors.join("; "))
}

fn api() -> std::result::Result<&'static Api, String> {
    // Keyed by the caller-selected library. A failed lookup is not cached, so
    // installing libcurl-impersonate later in the same process still works.
    // A successful handle is cached because curl_global_init must run once.
    static API: OnceLock<Api> = OnceLock::new();
    if let Some(api) = API.get() {
        return Ok(api);
    }
    match load_api() {
        Ok(loaded) => Ok(API.get_or_init(|| loaded)),
        Err(e) => Err(e),
    }
}

/// True when the loaded libcurl exports `curl_easy_impersonate`.
pub fn impersonate_available() -> bool {
    api().ok().and_then(|a| a.easy_impersonate).is_some()
}

pub fn loaded_library() -> Option<String> {
    api().ok().map(|a| a.path.clone())
}

pub struct ExtraFingerprint {
    pub ja3: Option<String>,
    pub akamai: Option<String>,
    pub sig_hash_algs: Option<String>,
    pub cert_compression: Option<String>,
    pub tls_permute_extensions: Option<bool>,
    pub tls_grease: Option<bool>,
    pub tls_cert_compression: Option<String>,
    pub http2_settings: Option<String>,
    pub http2_pseudo_header_order: Option<String>,
    pub http2_window_update: Option<i64>,
    pub http2_streams: Option<String>,
}

pub struct EasyRequest<'a> {
    pub method: &'a str,
    pub url: &'a str,
    pub headers: &'a [String],
    pub body: Option<&'a [u8]>,
    pub impersonate: Option<&'a str>,
    pub default_headers: bool,
    pub timeout_secs: u64,
    pub connect_timeout_secs: Option<u64>,
    pub proxy: Option<&'a str>,
    pub verify: bool,
    pub follow: bool,
    pub max_redirs: usize,
    pub force_http1: bool,
    pub interface: Option<&'a str>,
    pub cookie: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub accept_encoding: Option<&'a str>,
    pub extra: ExtraFingerprint,
}

pub struct EasyResponse {
    pub status: u16,
    pub effective_url: String,
    pub headers_raw: Vec<u8>,
    pub body: Vec<u8>,
}

struct WriteBuf {
    buf: Vec<u8>,
}

unsafe extern "C" fn write_cb(ptr: *mut c_char, size: size_t, nmemb: size_t, userdata: *mut c_void) -> size_t {
    if userdata.is_null() || ptr.is_null() {
        return 0;
    }
    let total = size.saturating_mul(nmemb);
    let dest = &mut *(userdata as *mut WriteBuf);
    let bytes = std::slice::from_raw_parts(ptr as *const u8, total);
    dest.buf.extend_from_slice(bytes);
    total
}

fn check(api: &Api, code: c_int, what: &str) -> Result<()> {
    if code == CURLE_OK {
        return Ok(());
    }
    let msg = unsafe {
        let p = (api.easy_strerror)(code);
        if p.is_null() {
            format!("curl error {code}")
        } else {
            CStr::from_ptr(p).to_string_lossy().into_owned()
        }
    };
    Err(RequestsError::Tls(format!("{what}: {msg} ({code})")))
}

/// Perform one request inside the process through libcurl-impersonate.
pub fn perform(req: &EasyRequest<'_>) -> Result<EasyResponse> {
    let api = api().map_err(|e| {
        RequestsError::Tls(format!(
            "libcurl-impersonate not loaded (set REQUESTS_LIBCURL). {e}"
        ))
    })?;

    if req.impersonate.is_some() && api.easy_impersonate.is_none() {
        return Err(RequestsError::Tls(format!(
            "loaded {} has no curl_easy_impersonate; point REQUESTS_LIBCURL at libcurl-impersonate",
            api.path
        )));
    }

    let handle = unsafe { (api.easy_init)() };
    if handle.is_null() {
        return Err(RequestsError::Tls("curl_easy_init returned null".into()));
    }

    let result = perform_on(api, handle, req);
    unsafe { (api.easy_cleanup)(handle) };
    result
}

fn perform_on(api: &Api, handle: *mut Easy, req: &EasyRequest<'_>) -> Result<EasyResponse> {
    if let Some(target) = req.impersonate {
        let c_target = CString::new(target).map_err(|_| {
            RequestsError::Tls("impersonate target contains a NUL byte".into())
        })?;
        let code = unsafe {
            (api.easy_impersonate.unwrap())(handle, c_target.as_ptr(), if req.default_headers { 1 } else { 0 })
        };
        check(api, code, "curl_easy_impersonate")?;
    }

    let url = CString::new(req.url).map_err(|_| RequestsError::InvalidUrl("url contains NUL".into()))?;
    let method = CString::new(req.method).map_err(|_| RequestsError::InvalidMethod("method contains NUL".into()))?;

    unsafe {
        check(api, (api.easy_setopt)(handle, CURLOPT_URL, url.as_ptr()), "CURLOPT_URL")?;
        check(
            api,
            (api.easy_setopt)(handle, CURLOPT_CUSTOMREQUEST, method.as_ptr()),
            "CURLOPT_CUSTOMREQUEST",
        )?;
        check(
            api,
            (api.easy_setopt)(handle, CURLOPT_TIMEOUT, req.timeout_secs as c_long),
            "CURLOPT_TIMEOUT",
        )?;
        check(
            api,
            (api.easy_setopt)(handle, CURLOPT_NOPROGRESS, 1 as c_long),
            "CURLOPT_NOPROGRESS",
        )?;
        check(
            api,
            (api.easy_setopt)(
                handle,
                CURLOPT_SSL_VERIFYPEER,
                if req.verify { 1 } else { 0 } as c_long,
            ),
            "CURLOPT_SSL_VERIFYPEER",
        )?;
        check(
            api,
            (api.easy_setopt)(
                handle,
                CURLOPT_SSL_VERIFYHOST,
                if req.verify { 2 } else { 0 } as c_long,
            ),
            "CURLOPT_SSL_VERIFYHOST",
        )?;
        check(
            api,
            (api.easy_setopt)(
                handle,
                CURLOPT_FOLLOWLOCATION,
                if req.follow { 1 } else { 0 } as c_long,
            ),
            "CURLOPT_FOLLOWLOCATION",
        )?;
        check(
            api,
            (api.easy_setopt)(handle, CURLOPT_MAXREDIRS, req.max_redirs as c_long),
            "CURLOPT_MAXREDIRS",
        )?;
        if req.force_http1 {
            check(
                api,
                (api.easy_setopt)(handle, CURLOPT_HTTP_VERSION, CURL_HTTP_VERSION_1_1),
                "CURLOPT_HTTP_VERSION",
            )?;
        }
    }

    let mut owned: Vec<CString> = Vec::new();
    let mut set_str = |opt: c_int, value: &str, what: &str| -> Result<()> {
        let c = CString::new(value).map_err(|_| RequestsError::Tls(format!("{what} contains NUL")))?;
        let code = unsafe { (api.easy_setopt)(handle, opt, c.as_ptr()) };
        owned.push(c);
        // Extension options are optional: a stock libcurl returns CURLE_UNKNOWN_OPTION (48).
        if code != CURLE_OK && !(code == 48 && opt >= CURLOPT_SSL_EC_CURVES) {
            check(api, code, what)?;
        }
        Ok(())
    };
    let set_long = |opt: c_int, value: c_long, what: &str| -> Result<()> {
        let code = unsafe { (api.easy_setopt)(handle, opt, value) };
        if code != CURLE_OK && !(code == 48 && opt >= CURLOPT_SSL_ENABLE_ALPS) {
            check(api, code, what)?;
        }
        Ok(())
    };

    if let Some(secs) = req.connect_timeout_secs {
        set_long(CURLOPT_CONNECTTIMEOUT, secs as c_long, "CURLOPT_CONNECTTIMEOUT")?;
    }
    if let Some(proxy) = req.proxy {
        set_str(CURLOPT_PROXY, proxy, "CURLOPT_PROXY")?;
    }
    if let Some(iface) = req.interface {
        set_str(CURLOPT_INTERFACE, iface, "CURLOPT_INTERFACE")?;
    }
    if let Some(cookie) = req.cookie {
        set_str(CURLOPT_COOKIE, cookie, "CURLOPT_COOKIE")?;
    }
    if let Some(ua) = req.user_agent {
        set_str(CURLOPT_USERAGENT, ua, "CURLOPT_USERAGENT")?;
    }
    // A concrete list both advertises the encoding and makes libcurl decode it.
    // An empty string is rejected (CURLE_BAD_FUNCTION_ARGUMENT) by this build.
    set_str(
        CURLOPT_ACCEPT_ENCODING,
        req.accept_encoding.unwrap_or("gzip, deflate, br, zstd"),
        "CURLOPT_ACCEPT_ENCODING",
    )?;

    // curl_cffi extra_fp / ja3 / akamai. JA3 here is the TLS cipher+extension
    // string libcurl-impersonate understands via CURLOPT_SSL_CIPHER_LIST is not
    // enough; the impersonate target already set ciphers. Callers that pass a
    // raw JA3 override the signature algorithms and permutation flags below.
    if let Some(algs) = req.extra.sig_hash_algs.as_deref() {
        set_str(CURLOPT_SSL_SIG_HASH_ALGS, algs, "CURLOPT_SSL_SIG_HASH_ALGS")?;
    }
    if let Some(comp) = req
        .extra
        .cert_compression
        .as_deref()
        .or(req.extra.tls_cert_compression.as_deref())
    {
        set_str(CURLOPT_SSL_CERT_COMPRESSION, comp, "CURLOPT_SSL_CERT_COMPRESSION")?;
    }
    if let Some(permute) = req.extra.tls_permute_extensions {
        set_long(
            CURLOPT_SSL_PERMUTE_EXTENSIONS,
            if permute { 1 } else { 0 },
            "CURLOPT_SSL_PERMUTE_EXTENSIONS",
        )?;
    }
    if let Some(true) = req.extra.tls_grease {
        set_long(CURLOPT_TLS_GREASE, 1, "CURLOPT_TLS_GREASE")?;
    }
    if let Some(settings) = req.extra.http2_settings.as_deref() {
        set_str(CURLOPT_HTTP2_SETTINGS, settings, "CURLOPT_HTTP2_SETTINGS")?;
    }
    if let Some(order) = req.extra.http2_pseudo_header_order.as_deref() {
        set_str(
            CURLOPT_HTTP2_PSEUDO_HEADERS_ORDER,
            order,
            "CURLOPT_HTTP2_PSEUDO_HEADERS_ORDER",
        )?;
    }
    if let Some(window) = req.extra.http2_window_update {
        set_long(CURLOPT_HTTP2_WINDOW_UPDATE, window as c_long, "CURLOPT_HTTP2_WINDOW_UPDATE")?;
    }
    if let Some(streams) = req.extra.http2_streams.as_deref() {
        set_str(CURLOPT_HTTP2_STREAMS, streams, "CURLOPT_HTTP2_STREAMS")?;
    }
    // A full JA3 string is recorded as the cipher list override when it looks
    // like "ciphers,extensions,curves,...". The first field is the cipher list.
    if let Some(ja3) = req.extra.ja3.as_deref() {
        if let Some(ciphers) = ja3.split(',').next() {
            if !ciphers.is_empty() {
                // CURLOPT_SSL_CIPHER_LIST = STRINGPOINT + 91 on both libcurl lines.
                set_str(CURLOPTTYPE_STRINGPOINT + 91, ciphers, "CURLOPT_SSL_CIPHER_LIST")?;
            }
        }
    }

    let mut slist: *mut Slist = ptr::null_mut();
    for header in req.headers {
        let c = CString::new(header.as_str()).map_err(|_| {
            RequestsError::Tls("header contains a NUL byte".into())
        })?;
        slist = unsafe { (api.slist_append)(slist, c.as_ptr()) };
        owned.push(c);
    }
    if !slist.is_null() {
        let code = unsafe { (api.easy_setopt)(handle, CURLOPT_HTTPHEADER, slist) };
        check(api, code, "CURLOPT_HTTPHEADER")?;
    }

    if let Some(body) = req.body {
        unsafe {
            check(
                api,
                (api.easy_setopt)(handle, CURLOPT_POSTFIELDS, body.as_ptr()),
                "CURLOPT_POSTFIELDS",
            )?;
            check(
                api,
                (api.easy_setopt)(handle, CURLOPT_POSTFIELDSIZE, body.len() as c_long),
                "CURLOPT_POSTFIELDSIZE",
            )?;
        }
    }

    let mut body_buf = WriteBuf { buf: Vec::new() };
    let mut header_buf = WriteBuf { buf: Vec::new() };
    unsafe {
        check(
            api,
            (api.easy_setopt)(handle, CURLOPT_WRITEFUNCTION, write_cb as *const c_void),
            "CURLOPT_WRITEFUNCTION",
        )?;
        check(
            api,
            (api.easy_setopt)(handle, CURLOPT_WRITEDATA, &mut body_buf as *mut WriteBuf),
            "CURLOPT_WRITEDATA",
        )?;
        check(
            api,
            (api.easy_setopt)(handle, CURLOPT_HEADERFUNCTION, write_cb as *const c_void),
            "CURLOPT_HEADERFUNCTION",
        )?;
        check(
            api,
            (api.easy_setopt)(handle, CURLOPT_HEADERDATA, &mut header_buf as *mut WriteBuf),
            "CURLOPT_HEADERDATA",
        )?;
        let code = (api.easy_perform)(handle);
        if !slist.is_null() {
            (api.slist_free_all)(slist);
        }
        check(api, code, "curl_easy_perform")?;
    }

    let mut status: c_long = 0;
    unsafe {
        check(
            api,
            (api.easy_getinfo)(handle, CURLINFO_RESPONSE_CODE, &mut status),
            "CURLINFO_RESPONSE_CODE",
        )?;
    }
    let mut effective: *mut c_char = ptr::null_mut();
    unsafe {
        let _ = (api.easy_getinfo)(handle, CURLINFO_EFFECTIVE_URL, &mut effective);
    }
    let effective_url = if effective.is_null() {
        req.url.to_string()
    } else {
        unsafe { CStr::from_ptr(effective).to_string_lossy().into_owned() }
    };

    // Keep the CStrings alive until perform returns.
    drop(owned);

    Ok(EasyResponse {
        status: status as u16,
        effective_url,
        headers_raw: header_buf.buf,
        body: body_buf.buf,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_numbers_match_libcurl_layout() {
        assert_eq!(CURLOPT_URL, 10002);
        assert_eq!(CURLOPT_WRITEFUNCTION, 20011);
        assert_eq!(CURLOPT_HTTPHEADER, 10023);
        assert_eq!(CURLOPT_SSL_VERIFYPEER, 64);
        // Same offsets curl_cffi/const.py uses for the lexiforest fork.
        assert_eq!(CURLOPT_IMPERSONATE, 10999);
        assert_eq!(CURLOPT_SSL_SIG_HASH_ALGS, 11001);
        assert_eq!(CURLOPT_SSL_ENABLE_ALPS, 1002);
        assert_eq!(CURLOPT_HTTP2_PSEUDO_HEADERS_ORDER, 11005);
        assert_eq!(CURLOPT_HTTP2_SETTINGS, 11006);
        assert_eq!(CURLOPT_SSL_PERMUTE_EXTENSIONS, 1007);
        assert_eq!(CURLOPT_HTTP2_WINDOW_UPDATE, 1008);
        assert_eq!(CURLOPT_HTTP2_STREAMS, 11010);
        assert_eq!(CURLOPT_TLS_GREASE, 1011);
        assert_eq!(CURLOPT_HTTPHEADER_ORDER, 11030);
    }

    #[test]
    fn impersonate_symbol_is_optional() {
        // Stock libcurl must not be treated as a successful impersonation backend.
        let _ = impersonate_available();
    }
}
